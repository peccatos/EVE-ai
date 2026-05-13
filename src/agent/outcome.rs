use std::fs;

use crate::agent::propose::load_proposal;
use crate::agent::storage::{id, load_json, memory_path, now_unix, save_json_pretty};
use crate::agent::task::{list_tasks, load_task};
use crate::contracts::{
    AgentPlan, AgentValidationStatus, ApplyResult, ApplyStatus, PatchProposal, TaskOutcome,
    ValidationRun,
};

pub fn build_task_outcome(memory_root: &str, task_id: &str) -> Result<TaskOutcome, String> {
    let task = load_task(memory_root, task_id)?;
    let proposal = task
        .proposal_id
        .as_ref()
        .and_then(|proposal_id| load_proposal(memory_root, proposal_id).ok());
    let plan = task.plan_id.as_ref().and_then(|plan_id| {
        load_json::<AgentPlan>(&memory_path(
            memory_root,
            &["plans", &format!("{plan_id}.json")],
        ))
        .ok()
    });
    let apply = task.apply_id.as_ref().and_then(|apply_id| {
        load_json::<ApplyResult>(&memory_path(
            memory_root,
            &["applies", &format!("{apply_id}.json")],
        ))
        .ok()
    });
    let validation = task
        .validation_id
        .as_ref()
        .and_then(|validation_id| {
            load_json::<ValidationRun>(&memory_path(
                memory_root,
                &["validations", &format!("{validation_id}.json")],
            ))
            .ok()
        })
        .or_else(|| {
            load_json::<ValidationRun>(&memory_path(
                memory_root,
                &["validations", "latest_validation.json"],
            ))
            .ok()
        });
    let applied = apply
        .as_ref()
        .map(|value| value.status == ApplyStatus::Applied)
        .unwrap_or(false);
    let validation_status = validation
        .as_ref()
        .map(|value| format!("{:?}", value.status).to_ascii_lowercase())
        .unwrap_or_else(|| "not_run".to_string());
    let success = applied
        && validation
            .as_ref()
            .map(|value| value.status == AgentValidationStatus::Passed)
            .unwrap_or(false);
    let outcome = TaskOutcome {
        outcome_id: id("task-outcome"),
        task_id: task.task_id.clone(),
        goal: task.goal.clone(),
        planner: plan
            .as_ref()
            .map(|value| value.planner.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        proposer: proposal
            .as_ref()
            .map(|value| value.proposer.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        llm_used: proposal
            .as_ref()
            .map(|value| value.llm_used)
            .unwrap_or(false),
        proposal_id: task.proposal_id.clone(),
        approval_id: task.approval_id.clone(),
        apply_id: task.apply_id.clone(),
        validation_id: validation.as_ref().map(|value| value.validation_id.clone()),
        report_id: task.report_id.clone(),
        pr_summary_id: task.pr_summary_id.clone(),
        files_changed: apply
            .as_ref()
            .map(|value| value.files_changed.clone())
            .unwrap_or_default(),
        patch_ops_count: proposal
            .as_ref()
            .map(PatchProposal::patch_ops_count)
            .unwrap_or(0),
        risk_level: proposal
            .as_ref()
            .map(|value| value.risk_level.clone())
            .unwrap_or_else(|| task.risk_level.clone()),
        approved: proposal
            .as_ref()
            .map(|value| value.approved)
            .unwrap_or(false),
        applied,
        validation_status: validation_status.clone(),
        success,
        failure_reason: infer_failure_reason(&proposal, &apply, validation.as_ref()),
        lessons: infer_lessons(&task.goal, success, &validation_status),
        warnings: task.warnings.clone(),
        blockers: task.blockers.clone(),
        created_at: now_unix(),
    };
    save_outcome(memory_root, &outcome)?;
    Ok(outcome)
}

pub fn save_outcome(memory_root: &str, outcome: &TaskOutcome) -> Result<(), String> {
    save_json_pretty(
        &memory_path(
            memory_root,
            &["task_outcomes", &format!("{}.json", outcome.task_id)],
        ),
        outcome,
    )?;
    save_json_pretty(
        &memory_path(memory_root, &["task_outcomes", "latest_task_outcome.json"]),
        outcome,
    )?;
    let outcomes = list_task_outcomes(memory_root)?;
    save_json_pretty(
        &memory_path(memory_root, &["task_outcomes", "index.json"]),
        &outcomes
            .iter()
            .map(|value| value.task_id.clone())
            .collect::<Vec<_>>(),
    )
}

pub fn list_task_outcomes(memory_root: &str) -> Result<Vec<TaskOutcome>, String> {
    let dir = memory_path(memory_root, &["task_outcomes"]);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut outcomes = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|error| format!("read {}: {error}", dir.display()))? {
        let entry = entry.map_err(|error| format!("read task outcome entry: {error}"))?;
        let path = entry.path();
        if path.file_name().and_then(|name| name.to_str()) == Some("latest_task_outcome.json")
            || path.file_name().and_then(|name| name.to_str()) == Some("index.json")
            || path.extension().and_then(|ext| ext.to_str()) != Some("json")
        {
            continue;
        }
        if let Ok(outcome) = load_json::<TaskOutcome>(&path) {
            outcomes.push(outcome);
        }
    }
    outcomes.sort_by(|a, b| a.task_id.cmp(&b.task_id));
    Ok(outcomes)
}

pub fn print_task_outcomes(memory_root: &str) -> Result<String, String> {
    let outcomes = list_task_outcomes(memory_root)?;
    let latest = outcomes
        .last()
        .map(|value| value.task_id.as_str())
        .unwrap_or("none");
    Ok(format!(
        "EVA Task Outcomes\ncount={}\nlatest={}",
        outcomes.len(),
        latest
    ))
}

pub fn print_task_outcome(memory_root: &str, task_id: &str) -> Result<String, String> {
    let path = memory_path(memory_root, &["task_outcomes", &format!("{task_id}.json")]);
    if !path.exists() && load_task(memory_root, task_id).is_ok() {
        build_task_outcome(memory_root, task_id)?;
    }
    match load_json::<TaskOutcome>(&path) {
        Ok(outcome) => Ok(format!(
            "EVA Task Outcome\ntask_id={}\nsuccess={}\nvalidation_status={}\napproved={}\napplied={}\nfailure_reason={}",
            outcome.task_id,
            outcome.success,
            outcome.validation_status,
            outcome.approved,
            outcome.applied,
            outcome.failure_reason.as_deref().unwrap_or("none")
        )),
        Err(_) => Ok(format!("task outcome not found\ntask_id={task_id}")),
    }
}

pub fn refresh_all_task_outcomes(memory_root: &str) -> Result<Vec<TaskOutcome>, String> {
    let mut outcomes = Vec::new();
    for task in list_tasks(memory_root)? {
        outcomes.push(build_task_outcome(memory_root, &task.task_id)?);
    }
    Ok(outcomes)
}

fn infer_failure_reason(
    proposal: &Option<PatchProposal>,
    apply: &Option<ApplyResult>,
    validation: Option<&ValidationRun>,
) -> Option<String> {
    if let Some(proposal) = proposal {
        if !proposal.blockers.is_empty() {
            return Some(proposal.blockers.join(","));
        }
        if !proposal.approved {
            return Some("not_approved".to_string());
        }
    }
    if let Some(apply) = apply {
        if !apply.blockers.is_empty() {
            return Some(apply.blockers.join(","));
        }
    }
    if let Some(validation) = validation {
        if validation.status != AgentValidationStatus::Passed {
            return Some(format!("{:?}", validation.status).to_ascii_lowercase());
        }
    }
    None
}

fn infer_lessons(goal: &str, success: bool, validation_status: &str) -> Vec<String> {
    let kind = crate::evolution::task_strategy_memory::classify_goal(goal);
    if success {
        vec![format!("{kind}_strategy_succeeded")]
    } else {
        vec![
            format!("{kind}_strategy_needs_smaller_scope"),
            format!("validation_status={validation_status}"),
        ]
    }
}

trait PatchProposalExt {
    fn patch_ops_count(&self) -> usize;
}

impl PatchProposalExt for PatchProposal {
    fn patch_ops_count(&self) -> usize {
        self.patch_ops.len()
    }
}
