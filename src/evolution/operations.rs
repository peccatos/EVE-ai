use crate::contracts::OperationsReport;
use crate::evolution::{
    build_artifact_audit, build_determinism_audit, build_future_phase_registry,
    build_preflight_gate, build_release_health, governance_status, load_or_refresh_promotion_queue,
    memory,
};

pub fn build_operations_report(
    project_root: &str,
    memory_root: &str,
) -> Result<OperationsReport, String> {
    let health = build_release_health(project_root, memory_root)?;
    let gate = build_preflight_gate(project_root, memory_root)?;
    let artifact = build_artifact_audit(project_root)?;
    let determinism = build_determinism_audit(project_root, memory_root)?;
    let governance = governance_status(project_root, memory_root)?;
    let queue = load_or_refresh_promotion_queue(project_root, memory_root)?;
    let future = build_future_phase_registry();
    let promotion_ready_count = queue
        .items
        .iter()
        .filter(|item| item.lifecycle_state == "ready")
        .count();
    let promotion_blocked_count = queue.items.len().saturating_sub(promotion_ready_count);
    let future_phases_allowed_now = future.entries.iter().any(|entry| entry.allowed_now);
    let next_safe_operator_action = if gate.gate_status == "pass" && promotion_ready_count > 0 {
        "cargo run -- --pr-package".to_string()
    } else if gate.gate_status == "warn" {
        "cargo run -- --promotion-ready-approved".to_string()
    } else {
        "cargo run -- --preflight-gate".to_string()
    };

    Ok(OperationsReport {
        generated_at: memory::now_unix(),
        release_health_grade: health.health_grade,
        release_health_score: health.health_score,
        preflight_gate_status: gate.gate_status,
        artifact_audit_status: if artifact.should_fail_release {
            "fail".to_string()
        } else {
            "pass".to_string()
        },
        determinism_audit_status: if determinism.deterministic_enough {
            "pass".to_string()
        } else {
            "fail".to_string()
        },
        governance_approved_count: governance.approved_count,
        governance_rejected_count: governance.rejected_count,
        governance_deferred_count: governance.deferred_count,
        promotion_ready_count,
        promotion_blocked_count,
        release_count: health.release_count,
        latest_release_id: health.latest_release_id,
        proof_support_capabilities: vec![
            "operations_runtime_support".to_string(),
            "pr_package_support".to_string(),
            "external_patch_package_support".to_string(),
            "self_review_package_support".to_string(),
            "operator_console_support".to_string(),
            "runtime_candidate_support".to_string(),
            "runtime_validation_support".to_string(),
            "runtime_service_metadata_support".to_string(),
            "stable_cli_contract_support".to_string(),
            "final_rc_report_support".to_string(),
        ],
        auto_promote: false,
        operator_approval_required: true,
        future_phases_allowed_now,
        next_safe_operator_action,
    })
}

pub fn print_ops_status(project_root: &str, memory_root: &str) -> Result<String, String> {
    let report = build_operations_report(project_root, memory_root)?;
    Ok(format!(
        "ops_status: health={} score={} preflight={} artifact={} determinism={} governance=approved:{} rejected:{} deferred:{} queue_ready={} queue_blocked={} releases={} latest={} auto_promote={} approval_required={} future_allowed_now={} next={}",
        report.release_health_grade,
        report.release_health_score,
        report.preflight_gate_status,
        report.artifact_audit_status,
        report.determinism_audit_status,
        report.governance_approved_count,
        report.governance_rejected_count,
        report.governance_deferred_count,
        report.promotion_ready_count,
        report.promotion_blocked_count,
        report.release_count,
        report.latest_release_id.as_deref().unwrap_or("none"),
        report.auto_promote,
        report.operator_approval_required,
        report.future_phases_allowed_now,
        report.next_safe_operator_action
    ))
}

pub fn print_ops_json(project_root: &str, memory_root: &str) -> Result<String, String> {
    serde_json::to_string_pretty(&build_operations_report(project_root, memory_root)?)
        .map_err(|error| format!("failed to serialize operations report: {error}"))
}
