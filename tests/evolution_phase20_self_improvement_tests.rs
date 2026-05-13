mod agent_v2_support;

use std::fs;

use agent_v2_support::temp_agent_root;
use eva_runtime_with_task_validator::agent::storage::{memory_path, save_json_pretty};
use eva_runtime_with_task_validator::{propose_self_improvement, TaskOutcome};

fn failed_outcome(task_id: &str) -> TaskOutcome {
    TaskOutcome {
        outcome_id: format!("outcome-{task_id}"),
        task_id: task_id.to_string(),
        goal: "broad unsafe request".to_string(),
        planner: "rule_based".to_string(),
        proposer: "rule_based".to_string(),
        llm_used: false,
        proposal_id: None,
        approval_id: None,
        apply_id: None,
        validation_id: None,
        report_id: None,
        pr_summary_id: None,
        files_changed: Vec::new(),
        patch_ops_count: 0,
        risk_level: "low".to_string(),
        approved: false,
        applied: false,
        validation_status: "not_run".to_string(),
        success: false,
        failure_reason: Some("manual_decomposition_required".to_string()),
        lessons: Vec::new(),
        warnings: Vec::new(),
        blockers: Vec::new(),
        created_at: 1,
    }
}

#[test]
fn self_improve_refuses_without_outcomes() {
    let root = temp_agent_root("self-improve-empty");
    let proposal =
        propose_self_improvement(root.join("memory").to_str().unwrap()).expect("proposal");
    assert!(proposal.blockers.contains(&"no_task_outcomes".to_string()));
    assert!(proposal.approval_required);
    assert!(proposal.proposal_id.is_none());
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn self_improve_creates_governed_proposal_only_from_repeated_weakness() {
    let root = temp_agent_root("self-improve-weakness");
    let memory = root.join("memory");
    save_json_pretty(
        &memory_path(memory.to_str().unwrap(), &["task_outcomes", "task-a.json"]),
        &failed_outcome("task-a"),
    )
    .expect("outcome");
    save_json_pretty(
        &memory_path(memory.to_str().unwrap(), &["task_outcomes", "task-b.json"]),
        &failed_outcome("task-b"),
    )
    .expect("outcome");
    let proposal = propose_self_improvement(memory.to_str().unwrap()).expect("proposal");
    assert!(proposal.blockers.is_empty());
    assert!(proposal.approval_required);
    assert!(proposal.proposal_id.is_none());
    assert!(proposal
        .allowed_scope
        .iter()
        .any(|path| path == "src/agent/"));
    assert!(proposal
        .forbidden_paths
        .iter()
        .any(|path| path == "memory/"));
    assert!(memory.join("self_improvement/latest_task.json").exists());
    assert!(!root.join("docs").join("self_applied.md").exists());
    fs::remove_dir_all(root).expect("cleanup");
}
