use crate::contracts::TrustProofReport;
use crate::evolution::{
    build_capability_policy, build_evidence_bundle, build_preflight_gate_v3,
    build_recovery_manifest, build_trust_decision, build_workspace_snapshot, memory,
};

pub fn build_trust_proof_report(
    project_root: &str,
    memory_root: &str,
) -> Result<TrustProofReport, String> {
    let policy = build_capability_policy();
    let trust = build_trust_decision(project_root, memory_root)?;
    let snapshot = build_workspace_snapshot(project_root, memory_root)?;
    let evidence = build_evidence_bundle(project_root, memory_root)?;
    let recovery = build_recovery_manifest(project_root, memory_root)?;
    let gate_v3 = build_preflight_gate_v3(project_root, memory_root)?;
    Ok(TrustProofReport {
        generated_at: memory::now_unix(),
        capability_policy_status: format!(
            "denied={} allowed={}",
            policy.denied_capabilities.len(),
            policy.allowed_capabilities.len()
        ),
        trust_decision: trust.trust_decision,
        workspace_snapshot_id: Some(snapshot.snapshot_id),
        evidence_bundle_id: Some(evidence.bundle_id),
        recovery_manifest_id: Some(recovery.manifest_id),
        preflight_gate_v3_status: gate_v3.status,
        next_operator_commands: gate_v3.next_actions,
    })
}

pub fn print_trust_proof_report(project_root: &str, memory_root: &str) -> Result<String, String> {
    let report = build_trust_proof_report(project_root, memory_root)?;
    Ok(format!(
        "# EVA Trust Proof Report\n\ncapability_policy_status={}\ntrust_decision={}\nworkspace_snapshot_id={}\nevidence_bundle_id={}\nrecovery_manifest_id={}\npreflight_gate_v3_status={}\n\nnext_commands:\n{}\n",
        report.capability_policy_status,
        report.trust_decision,
        report.workspace_snapshot_id.as_deref().unwrap_or("none"),
        report.evidence_bundle_id.as_deref().unwrap_or("none"),
        report.recovery_manifest_id.as_deref().unwrap_or("none"),
        report.preflight_gate_v3_status,
        report
            .next_operator_commands
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    ))
}
