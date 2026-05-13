use std::fs;
use std::path::Path;

use crate::agent::propose::{load_proposal, save_proposal, validate_patch_proposal};
use crate::agent::safe_paths::validate_patch_path;
use crate::agent::snapshot::create_snapshot;
use crate::agent::storage::{id, memory_path, now_unix, save_json_pretty};
use crate::agent::task::{load_task, update_task};
use crate::contracts::{
    AgentTaskStatus, ApplyResult, ApplyStatus, PatchOperationKind, ProposalStatus,
};

pub fn apply_proposal(
    project_root: &str,
    memory_root: &str,
    proposal_id: &str,
) -> Result<ApplyResult, String> {
    let mut proposal = load_proposal(memory_root, proposal_id)?;
    validate_patch_proposal(&mut proposal);
    if !proposal.approved || proposal.status != ProposalStatus::Approved {
        return Ok(refused(proposal_id, &proposal.task_id, "not_approved"));
    }
    if !proposal.blockers.is_empty() {
        return Ok(refused(
            proposal_id,
            &proposal.task_id,
            &proposal.blockers.join(","),
        ));
    }
    let (snapshot_path, rollback_path) =
        create_snapshot(project_root, memory_root, &proposal.patch_ops)?;
    let mut files_changed = Vec::new();
    for op in &proposal.patch_ops {
        if let Err(error) = validate_patch_path(&op.path) {
            return Ok(refused(proposal_id, &proposal.task_id, &error.to_string()));
        }
        let path = Path::new(project_root).join(&op.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("create parent {}: {error}", parent.display()))?;
        }
        match op.op {
            PatchOperationKind::CreateFile => {
                if path.exists() {
                    return Ok(refused(
                        proposal_id,
                        &proposal.task_id,
                        "create_file_exists",
                    ));
                }
                fs::write(&path, op.content.clone().unwrap_or_default())
                    .map_err(|error| format!("write {}: {error}", path.display()))?;
            }
            PatchOperationKind::AppendFile => {
                use std::io::Write;
                if !path.exists() {
                    return Ok(refused(
                        proposal_id,
                        &proposal.task_id,
                        "append_file_missing",
                    ));
                }
                let mut file = fs::OpenOptions::new()
                    .append(true)
                    .open(&path)
                    .map_err(|error| format!("open append {}: {error}", path.display()))?;
                file.write_all(op.content.clone().unwrap_or_default().as_bytes())
                    .map_err(|error| format!("append {}: {error}", path.display()))?;
            }
            PatchOperationKind::ReplaceFileIfExists => {
                if path.exists() {
                    fs::write(&path, op.content.clone().unwrap_or_default())
                        .map_err(|error| format!("replace {}: {error}", path.display()))?;
                } else {
                    return Ok(refused(
                        proposal_id,
                        &proposal.task_id,
                        "replace_file_missing",
                    ));
                }
            }
            PatchOperationKind::ReplaceExactText => {
                let contents = fs::read_to_string(&path)
                    .map_err(|error| format!("read replace {}: {error}", path.display()))?;
                let find = op.find.as_deref().unwrap_or_default();
                let matches = contents.matches(find).count();
                if matches == 0 {
                    return Ok(refused(
                        proposal_id,
                        &proposal.task_id,
                        "exact_text_not_found",
                    ));
                }
                if matches > 1 {
                    return Ok(refused(
                        proposal_id,
                        &proposal.task_id,
                        "ambiguous_exact_text_match",
                    ));
                }
                let next = contents.replace(find, op.replace.as_deref().unwrap_or_default());
                fs::write(&path, next)
                    .map_err(|error| format!("write replace {}: {error}", path.display()))?;
            }
        }
        files_changed.push(op.path.clone());
    }
    proposal.status = ProposalStatus::Applied;
    proposal.updated_at = now_unix();
    save_proposal(memory_root, &proposal)?;
    let mut task = load_task(memory_root, &proposal.task_id)?;
    task.status = AgentTaskStatus::Applied;
    let result = ApplyResult {
        apply_id: id("apply"),
        proposal_id: proposal_id.into(),
        task_id: proposal.task_id.clone(),
        status: ApplyStatus::Applied,
        applied_at: now_unix(),
        files_changed,
        snapshot_id: Some(snapshot_path),
        rollback_manifest: Some(rollback_path),
        warnings: Vec::new(),
        blockers: Vec::new(),
    };
    task.apply_id = Some(result.apply_id.clone());
    update_task(memory_root, task)?;
    save_json_pretty(
        &memory_path(
            memory_root,
            &["applies", &format!("{}.json", result.apply_id)],
        ),
        &result,
    )?;
    save_json_pretty(
        &memory_path(memory_root, &["applies", "latest_apply.json"]),
        &result,
    )?;
    Ok(result)
}

pub fn dry_run_apply(
    project_root: &str,
    memory_root: &str,
    proposal_id: &str,
) -> Result<String, String> {
    let mut proposal = load_proposal(memory_root, proposal_id)?;
    validate_patch_proposal(&mut proposal);
    let mut blockers = proposal.blockers.clone();
    if !proposal.approved || proposal.status != ProposalStatus::Approved {
        blockers.push("not_approved".to_string());
    }
    for op in &proposal.patch_ops {
        if let Err(error) = validate_patch_path(&op.path) {
            blockers.push(error.to_string());
        }
        let path = Path::new(project_root).join(&op.path);
        match op.op {
            PatchOperationKind::CreateFile if path.exists() => {
                blockers.push("create_file_exists".to_string());
            }
            PatchOperationKind::AppendFile if !path.exists() => {
                blockers.push("append_file_missing".to_string());
            }
            PatchOperationKind::ReplaceFileIfExists if !path.exists() => {
                blockers.push("replace_file_missing".to_string());
            }
            PatchOperationKind::ReplaceExactText => {
                let contents = fs::read_to_string(&path).unwrap_or_default();
                let find = op.find.as_deref().unwrap_or_default();
                let count = if find.is_empty() {
                    0
                } else {
                    contents.matches(find).count()
                };
                if count == 0 {
                    blockers.push("exact_text_not_found".to_string());
                } else if count > 1 {
                    blockers.push("ambiguous_exact_text_match".to_string());
                }
            }
            _ => {}
        }
    }
    blockers.sort();
    blockers.dedup();
    Ok(format!(
        "EVA Apply Dry Run\nproposal_id={}\nwould_apply={}\nfiles_changed={}\nops={}\nsnapshot_required=true\napproval_required=true\napproved={}\nblockers={}\npreview={}",
        proposal.proposal_id,
        blockers.is_empty(),
        proposal.files_to_change.join(","),
        proposal
            .patch_ops
            .iter()
            .map(|op| format!("{}:{:?}", op.path, op.op))
            .collect::<Vec<_>>()
            .join(","),
        proposal.approved,
        if blockers.is_empty() { "none".to_string() } else { blockers.join(",") },
        proposal.summary
    ))
}

fn refused(proposal_id: &str, task_id: &str, reason: &str) -> ApplyResult {
    ApplyResult {
        apply_id: id("apply-refused"),
        proposal_id: proposal_id.into(),
        task_id: task_id.into(),
        status: ApplyStatus::Refused,
        applied_at: now_unix(),
        files_changed: Vec::new(),
        snapshot_id: None,
        rollback_manifest: None,
        warnings: Vec::new(),
        blockers: vec![reason.into()],
    }
}

pub fn print_apply_proposal(
    project_root: &str,
    memory_root: &str,
    proposal_id: &str,
) -> Result<String, String> {
    let result = apply_proposal(project_root, memory_root, proposal_id)?;
    if result.status == ApplyStatus::Refused {
        return Ok(format!(
            "apply refused\nproposal_id={proposal_id}\nreason={}",
            result.blockers.join(",")
        ));
    }
    Ok(format!(
        "proposal applied\nproposal_id={}\napply_id={}\nfiles_changed={}\nsnapshot_id={}\nrollback_manifest={}",
        result.proposal_id,
        result.apply_id,
        result.files_changed.join(","),
        result.snapshot_id.as_deref().unwrap_or("missing"),
        result.rollback_manifest.as_deref().unwrap_or("missing")
    ))
}

pub fn print_apply_dry_run(
    project_root: &str,
    memory_root: &str,
    proposal_id: &str,
) -> Result<String, String> {
    dry_run_apply(project_root, memory_root, proposal_id)
}
