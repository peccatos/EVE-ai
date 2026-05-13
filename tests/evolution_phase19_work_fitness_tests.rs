use eva_runtime_with_task_validator::{score_task_outcome, FitnessDecision, TaskOutcome};

fn outcome(success: bool, validation_status: &str, risk: &str, ops: usize) -> TaskOutcome {
    TaskOutcome {
        outcome_id: "outcome".to_string(),
        task_id: "task".to_string(),
        goal: "document behavior".to_string(),
        planner: "rule_based".to_string(),
        proposer: "rule_based".to_string(),
        llm_used: false,
        proposal_id: Some("proposal".to_string()),
        approval_id: Some("approval".to_string()),
        apply_id: Some("apply".to_string()),
        validation_id: Some("validation".to_string()),
        report_id: None,
        pr_summary_id: None,
        files_changed: vec!["docs/x.md".to_string()],
        patch_ops_count: ops,
        risk_level: risk.to_string(),
        approved: true,
        applied: true,
        validation_status: validation_status.to_string(),
        success,
        failure_reason: if success {
            None
        } else {
            Some("failed".to_string())
        },
        lessons: Vec::new(),
        warnings: Vec::new(),
        blockers: Vec::new(),
        created_at: 1,
    }
}

#[test]
fn fitness_scores_validation_passed_higher_than_failed() {
    let passed = score_task_outcome(&outcome(true, "passed", "low", 1));
    let failed = score_task_outcome(&outcome(false, "failed", "low", 1));
    assert!(passed.final_score > failed.final_score);
    assert_eq!(failed.decision, FitnessDecision::Reject);
}

#[test]
fn fitness_penalizes_high_risk_and_large_patch() {
    let low = score_task_outcome(&outcome(true, "passed", "low", 1));
    let high = score_task_outcome(&outcome(true, "passed", "high", 8));
    assert!(high.final_score < low.final_score);
    assert!(high.risk_penalty > 0.0);
    assert!(high.patch_size_penalty > 0.0);
}
