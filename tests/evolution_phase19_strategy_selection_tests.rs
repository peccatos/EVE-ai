mod agent_v2_support;

use std::fs;

use agent_v2_support::temp_agent_root;
use eva_runtime_with_task_validator::agent::storage::{memory_path, save_json_pretty};
use eva_runtime_with_task_validator::{select_strategy, TaskOutcome};

fn doc_success() -> TaskOutcome {
    TaskOutcome {
        outcome_id: "outcome-doc".to_string(),
        task_id: "task-doc".to_string(),
        goal: "document agent".to_string(),
        planner: "rule_based".to_string(),
        proposer: "rule_based".to_string(),
        llm_used: false,
        proposal_id: None,
        approval_id: None,
        apply_id: None,
        validation_id: None,
        report_id: None,
        pr_summary_id: None,
        files_changed: vec!["docs/x.md".to_string()],
        patch_ops_count: 1,
        risk_level: "low".to_string(),
        approved: true,
        applied: true,
        validation_status: "passed".to_string(),
        success: true,
        failure_reason: None,
        lessons: Vec::new(),
        warnings: Vec::new(),
        blockers: Vec::new(),
        created_at: 1,
    }
}

#[test]
fn strategy_select_prefers_docs_and_refuses_unsafe() {
    let root = temp_agent_root("strategy-select");
    let memory = root.join("memory");
    save_json_pretty(
        &memory_path(
            memory.to_str().unwrap(),
            &["task_outcomes", "task-doc.json"],
        ),
        &doc_success(),
    )
    .expect("outcome");
    let (strategy, _, _) =
        select_strategy(memory.to_str().unwrap(), "document agent behavior").expect("strategy");
    assert_eq!(strategy, "docs_only");
    let (strategy, _, _) =
        select_strategy(memory.to_str().unwrap(), "edit .git/config").expect("strategy");
    assert_eq!(strategy, "refuse_unsafe");
    let (strategy, _, _) =
        select_strategy(memory.to_str().unwrap(), "do everything everywhere").expect("strategy");
    assert_eq!(strategy, "manual_decomposition_required");
    fs::remove_dir_all(root).expect("cleanup");
}
