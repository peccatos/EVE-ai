use std::fs;
use std::path::Path;

use crate::contracts::{sha256_digest, ReleaseProposal, ReleaseProposalItem};
use crate::evolution::{
    governance_status, latest_record_for_run, memory, promotion_ready_approved,
};

pub fn build_release_proposal(
    project_root: &str,
    memory_root: &str,
) -> Result<ReleaseProposal, String> {
    let ready = promotion_ready_approved(project_root, memory_root)?;
    let status = governance_status(project_root, memory_root)?;
    let mut items = ready
        .into_iter()
        .map(|item| {
            let approval = latest_record_for_run(memory_root, &item.run_id)?
                .ok_or_else(|| format!("missing approval for {}", item.run_id))?;
            Ok(ReleaseProposalItem {
                run_id: item.run_id,
                mutation_kind: item.mutation_kind,
                target_file: item.target_file,
                score: item.score,
                risk: item.risk,
                approval_reason: approval.reason,
                report_path: item.report_path,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    items.sort_by(|left, right| left.run_id.cmp(&right.run_id));
    let proposal_seed = items
        .iter()
        .map(|item| {
            format!(
                "{}:{}:{:.2}:{:.2}:{}",
                item.run_id, item.target_file, item.score, item.risk, item.approval_reason
            )
        })
        .collect::<Vec<_>>()
        .join("|");
    let proposal_id = format!("proposal-{}", &sha256_digest(&proposal_seed)[..8]);
    let created_at = items
        .iter()
        .filter_map(|item| {
            latest_record_for_run(memory_root, &item.run_id)
                .ok()
                .flatten()
        })
        .map(|record| record.created_at)
        .max()
        .unwrap_or(0);
    let proposal = ReleaseProposal {
        proposal_id,
        items,
        auto_promote: false,
        forbidden_targets_preserved: true,
        rejected_count: status.rejected_count,
        deferred_count: status.deferred_count,
        ready_approved_count: status.promotion_ready_approved_count,
        created_at,
    };
    write_release_proposal(memory_root, &proposal)?;
    Ok(proposal)
}

pub fn print_release_proposal(project_root: &str, memory_root: &str) -> Result<String, String> {
    let proposal = build_release_proposal(project_root, memory_root)?;
    Ok(render_release_proposal_markdown(&proposal))
}

pub fn print_release_proposal_json(
    project_root: &str,
    memory_root: &str,
) -> Result<String, String> {
    let proposal = build_release_proposal(project_root, memory_root)?;
    serde_json::to_string_pretty(&proposal)
        .map_err(|error| format!("failed to serialize release proposal: {error}"))
}

pub fn release_proposal_count(memory_root: &str) -> Result<usize, String> {
    let dir = Path::new(memory_root)
        .join("governance")
        .join("release_proposals");
    if !dir.exists() {
        return Ok(0);
    }
    Ok(fs::read_dir(dir)
        .map_err(|error| format!("failed to read release proposals: {error}"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .count())
}

fn write_release_proposal(memory_root: &str, proposal: &ReleaseProposal) -> Result<(), String> {
    let dir = Path::new(memory_root)
        .join("governance")
        .join("release_proposals");
    fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create release proposal dir: {error}"))?;
    memory::write_json(dir.join(format!("{}.json", proposal.proposal_id)), proposal)?;
    fs::write(
        dir.join(format!("{}.ru.md", proposal.proposal_id)),
        render_release_proposal_markdown(proposal),
    )
    .map_err(|error| format!("failed to write release proposal markdown: {error}"))
}

fn render_release_proposal_markdown(proposal: &ReleaseProposal) -> String {
    let items = if proposal.items.is_empty() {
        "(none)".to_string()
    } else {
        proposal
            .items
            .iter()
            .map(|item| {
                format!(
                    "- {} kind={} target={} score={:.1} risk={:.2} reason={} report={}",
                    item.run_id,
                    item.mutation_kind,
                    item.target_file,
                    item.score,
                    item.risk,
                    item.approval_reason,
                    item.report_path
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "# Release Proposal EVA\n\nproposal_id={}\nauto_promote={}\nforbidden_targets_preserved={}\nrejected_count={}\ndeferred_count={}\nready_approved_count={}\n\n{}\n",
        proposal.proposal_id,
        proposal.auto_promote,
        proposal.forbidden_targets_preserved,
        proposal.rejected_count,
        proposal.deferred_count,
        proposal.ready_approved_count,
        items
    )
}
