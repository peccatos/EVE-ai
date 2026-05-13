mod agent_v2_support;

use std::fs;

use agent_v2_support::temp_agent_root;
use eva_runtime_with_task_validator::agent::storage::{memory_path, save_json_pretty};
use eva_runtime_with_task_validator::{
    build_agent_patterns, build_task_outcome_analysis, build_task_strategy_memory, TaskOutcome,
};

fn outcome(task_id: &str, goal: &str, success: bool) -> TaskOutcome {
    TaskOutcome {
        outcome_id: format!("outcome-{task_id}"),
        task_id: task_id.to_string(),
        goal: goal.to_string(),
        planner: "rule_based".to_string(),
        proposer: "rule_based".to_string(),
        llm_used: false,
        proposal_id: Some(format!("proposal-{task_id}")),
        approval_id: Some(format!("approval-{task_id}")),
        apply_id: Some(format!("apply-{task_id}")),
        validation_id: Some(format!("validation-{task_id}")),
        report_id: Some(format!("report-{task_id}")),
        pr_summary_id: Some(format!("pr-{task_id}")),
        files_changed: vec!["docs/x.md".to_string()],
        patch_ops_count: 1,
        risk_level: "low".to_string(),
        approved: true,
        applied: true,
        validation_status: if success { "passed" } else { "failed" }.to_string(),
        success,
        failure_reason: if success {
            None
        } else {
            Some("failed_validation_after_apply".to_string())
        },
        lessons: Vec::new(),
        warnings: Vec::new(),
        blockers: Vec::new(),
        created_at: 1,
    }
}

#[test]
fn outcome_analyzer_handles_empty_and_counts_outcomes() {
    let root = temp_agent_root("outcome-analysis");
    let memory = root.join("memory");
    let empty = build_task_outcome_analysis(memory.to_str().unwrap()).expect("empty");
    assert_eq!(empty.total_tasks, 0);
    assert!(empty.warnings.contains(&"no_task_outcomes".to_string()));
    save_json_pretty(
        &memory_path(
            memory.to_str().unwrap(),
            &["task_outcomes", "task-doc.json"],
        ),
        &outcome("task-doc", "document behavior", true),
    )
    .expect("outcome");
    save_json_pretty(
        &memory_path(
            memory.to_str().unwrap(),
            &["task_outcomes", "task-broad.json"],
        ),
        &outcome("task-broad", "do everything", false),
    )
    .expect("outcome");
    let analysis = build_task_outcome_analysis(memory.to_str().unwrap()).expect("analysis");
    assert_eq!(analysis.total_tasks, 2);
    assert_eq!(analysis.successful_tasks, 1);
    assert_eq!(analysis.failed_tasks, 1);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn pattern_extraction_and_strategy_memory_use_real_outcomes() {
    let root = temp_agent_root("patterns");
    let memory = root.join("memory");
    save_json_pretty(
        &memory_path(
            memory.to_str().unwrap(),
            &["task_outcomes", "task-doc.json"],
        ),
        &outcome("task-doc", "document behavior", true),
    )
    .expect("outcome");
    save_json_pretty(
        &memory_path(
            memory.to_str().unwrap(),
            &["task_outcomes", "task-broad.json"],
        ),
        &outcome("task-broad", "do everything", false),
    )
    .expect("outcome");
    let patterns = build_agent_patterns(memory.to_str().unwrap()).expect("patterns");
    assert!(patterns
        .patterns
        .iter()
        .any(|p| p.pattern_id == "successful_docs_task"));
    assert!(patterns
        .patterns
        .iter()
        .any(|p| p.pattern_id == "failed_broad_task"));
    let memory_state = build_task_strategy_memory(memory.to_str().unwrap()).expect("strategy");
    assert!(memory_state
        .strategies
        .iter()
        .any(|s| s.recommended_strategy == "docs_only"));
    fs::remove_dir_all(root).expect("cleanup");
}
