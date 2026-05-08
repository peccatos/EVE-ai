use crate::contracts::PreflightGateV3Report;
use crate::evolution::{
    build_capability_policy, build_evidence_bundle, build_preflight_gate, build_recovery_manifest,
    build_trust_decision, build_workspace_snapshot, memory,
};

pub fn build_preflight_gate_v3(
    project_root: &str,
    memory_root: &str,
) -> Result<PreflightGateV3Report, String> {
    let gate_v2 = build_preflight_gate(project_root, memory_root)?;
    let policy = build_capability_policy();
    let trust = build_trust_decision(project_root, memory_root)?;
    let snapshot = build_workspace_snapshot(project_root, memory_root)?;
    let evidence = build_evidence_bundle(project_root, memory_root)?;
    let recovery = build_recovery_manifest(project_root, memory_root)?;

    let mut blockers = gate_v2.blockers.clone();
    let mut warnings = gate_v2.warnings.clone();
    blockers.extend(trust.blockers.clone());
    warnings.extend(trust.warnings.clone());
    if policy.auto_promote_allowed {
        blockers.push("capability_policy_auto_promote_allowed".to_string());
    }
    if snapshot.sandbox_leak_count > 0 {
        blockers.push("workspace_snapshot_sandbox_leaks".to_string());
    }
    if evidence.bundle_id.is_empty() {
        blockers.push("evidence_bundle_missing".to_string());
    }
    if recovery.manifest_id.is_empty() {
        blockers.push("recovery_manifest_missing".to_string());
    }
    blockers.sort();
    blockers.dedup();
    warnings.sort();
    warnings.dedup();
    let status = if !blockers.is_empty() || trust.trust_decision == "deny" {
        "fail"
    } else if !warnings.is_empty()
        || trust.trust_decision == "warn"
        || gate_v2.gate_status == "warn"
    {
        "warn"
    } else {
        "pass"
    }
    .to_string();
    let mut next_actions = trust.next_actions.clone();
    next_actions.push("cargo run -- --trust-proof-report".to_string());
    next_actions.sort();
    next_actions.dedup();
    Ok(PreflightGateV3Report {
        generated_at: memory::now_unix(),
        status,
        blockers,
        warnings,
        next_actions,
        auto_promote: false,
        operator_approval_required: true,
    })
}

pub fn print_preflight_gate_v3(project_root: &str, memory_root: &str) -> Result<String, String> {
    let report = build_preflight_gate_v3(project_root, memory_root)?;
    Ok(format!(
        "preflight_gate_v3: status={} auto_promote={} approval_required={} blockers={} warnings={}\nnext_actions={}",
        report.status,
        report.auto_promote,
        report.operator_approval_required,
        if report.blockers.is_empty() { "none".to_string() } else { report.blockers.join(",") },
        if report.warnings.is_empty() { "none".to_string() } else { report.warnings.join(",") },
        report.next_actions.join("; ")
    ))
}
