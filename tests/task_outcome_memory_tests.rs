mod agent_v2_support;

use std::fs;

use agent_v2_support::temp_agent_root;
use eva_runtime_with_task_validator::agent::storage::{memory_path, save_json_pretty};
use eva_runtime_with_task_validator::{
    approve_proposal, build_agent_report, build_pr_summary_for_task, build_task_outcome,
    create_task, list_task_outcomes, plan_task, propose_task, AgentValidationStatus, ValidationRun,
};

#[test]
fn task_outcome_created_after_successful_report_and_pr_summary() {
    let root = temp_agent_root("outcome-success");
    let memory = root.join("memory");
    let task = create_task(memory.to_str().unwrap(), "document outcome").expect("task");
    plan_task(
        root.to_str().unwrap(),
        memory.to_str().unwrap(),
        &task.task_id,
    )
    .expect("plan");
    let proposal = propose_task(
        root.to_str().unwrap(),
        memory.to_str().unwrap(),
        &task.task_id,
    )
    .expect("proposal");
    approve_proposal(memory.to_str().unwrap(), &proposal.proposal_id).expect("approve");
    eva_runtime_with_task_validator::apply_proposal(
        root.to_str().unwrap(),
        memory.to_str().unwrap(),
        &proposal.proposal_id,
    )
    .expect("apply");
    let validation = ValidationRun {
        validation_id: "validation-passed".to_string(),
        task_id: Some(task.task_id.clone()),
        proposal_id: Some(proposal.proposal_id.clone()),
        status: AgentValidationStatus::Passed,
        started_at: 1,
        finished_at: 2,
        commands: Vec::new(),
        warnings: Vec::new(),
        blockers: Vec::new(),
    };
    save_json_pretty(
        &memory_path(
            memory.to_str().unwrap(),
            &["validations", "latest_validation.json"],
        ),
        &validation,
    )
    .expect("validation");
    build_agent_report(memory.to_str().unwrap(), &task.task_id).expect("report");
    build_pr_summary_for_task(memory.to_str().unwrap(), &task.task_id).expect("pr");
    let outcome = build_task_outcome(memory.to_str().unwrap(), &task.task_id).expect("outcome");
    assert!(outcome.success);
    assert!(outcome.approved);
    assert!(outcome.applied);
    assert_eq!(
        list_task_outcomes(memory.to_str().unwrap()).unwrap().len(),
        1
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn task_outcome_created_after_proposal_refusal_and_failed_validation() {
    let root = temp_agent_root("outcome-failure");
    let memory = root.join("memory");
    let task = create_task(memory.to_str().unwrap(), "do everything everywhere").expect("task");
    plan_task(
        root.to_str().unwrap(),
        memory.to_str().unwrap(),
        &task.task_id,
    )
    .expect("plan");
    let proposal = propose_task(
        root.to_str().unwrap(),
        memory.to_str().unwrap(),
        &task.task_id,
    )
    .expect("proposal");
    assert!(!proposal.blockers.is_empty());
    let outcome = build_task_outcome(memory.to_str().unwrap(), &task.task_id).expect("outcome");
    assert!(!outcome.success);
    assert_eq!(
        outcome.failure_reason.as_deref(),
        Some("task_requires_manual_decomposition")
    );

    let failed = ValidationRun {
        validation_id: "validation-failed".to_string(),
        task_id: Some(task.task_id.clone()),
        proposal_id: None,
        status: AgentValidationStatus::Failed,
        started_at: 1,
        finished_at: 2,
        commands: Vec::new(),
        warnings: Vec::new(),
        blockers: Vec::new(),
    };
    save_json_pretty(
        &memory_path(
            memory.to_str().unwrap(),
            &["validations", "latest_validation.json"],
        ),
        &failed,
    )
    .expect("validation");
    let outcome = build_task_outcome(memory.to_str().unwrap(), &task.task_id).expect("outcome");
    assert_eq!(outcome.validation_status, "failed");
    fs::remove_dir_all(root).expect("cleanup");
}
