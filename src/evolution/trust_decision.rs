use std::path::Path;

use crate::contracts::TrustDecision;
use crate::evolution::{
    build_artifact_audit, build_capability_policy, build_determinism_audit, build_preflight_gate,
    build_release_health, build_workspace_snapshot, governance_status, memory,
};

pub fn build_trust_decision(
    project_root: &str,
    memory_root: &str,
) -> Result<TrustDecision, String> {
    let policy = build_capability_policy();
    let governance = governance_status(project_root, memory_root)?;
    let release_health = build_release_health(project_root, memory_root)?;
    let artifact = build_artifact_audit(project_root)?;
    let determinism = build_determinism_audit(project_root, memory_root)?;
    let gate_v2 = build_preflight_gate(project_root, memory_root)?;
    let snapshot = build_workspace_snapshot(project_root, memory_root)?;
    let rollback_available = latest_rollback_manifest_path(memory_root).is_some();

    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    if policy.auto_promote_allowed {
        blockers.push("auto_promote_allowed".to_string());
    }
    if policy.network_push_allowed {
        blockers.push("network_push_allowed".to_string());
    }
    if policy.merge_allowed {
        blockers.push("merge_allowed".to_string());
    }
    if policy.external_repo_mutation_allowed {
        blockers.push("external_repo_mutation_allowed".to_string());
    }
    if policy.self_apply_allowed {
        blockers.push("self_apply_allowed".to_string());
    }
    if policy.source_mutation_without_approval_allowed {
        blockers.push("source_mutation_without_approval_allowed".to_string());
    }
    if artifact.should_fail_release {
        blockers.push("artifact_audit_failed".to_string());
    }
    if !determinism.deterministic_enough {
        blockers.push("determinism_audit_failed".to_string());
    }
    if snapshot.sandbox_leak_count > 0 {
        blockers.push("sandbox_leaks_present".to_string());
    }
    if gate_v2.gate_status == "fail" {
        blockers.push("preflight_gate_v2_failed".to_string());
    }
    if release_health.health_grade == "red" {
        blockers.push("release_health_red".to_string());
    }
    if !governance.operator_approval_required {
        blockers.push("operator_approval_not_required".to_string());
    }

    if governance.promotion_ready_approved_count == 0 {
        warnings.push("no_approved_release_candidate".to_string());
    }
    if !rollback_available {
        warnings.push("rollback_manifest_missing".to_string());
    }
    if snapshot.modified_count > 0 || snapshot.untracked_count > 0 {
        warnings.push("workspace_not_clean".to_string());
    }
    if gate_v2.gate_status == "warn" {
        warnings.push("preflight_gate_v2_warn".to_string());
    }

    blockers.sort();
    blockers.dedup();
    warnings.sort();
    warnings.dedup();
    let trust_decision = if !blockers.is_empty() {
        "deny"
    } else if !warnings.is_empty() {
        "warn"
    } else {
        "allow"
    }
    .to_string();
    let next_actions = if trust_decision == "allow" {
        vec![
            "cargo run -- --preflight-gate-v3".to_string(),
            "cargo run -- --trust-proof-report".to_string(),
        ]
    } else if trust_decision == "warn" {
        vec![
            "cargo run -- --promotion-ready-approved".to_string(),
            "cargo run -- --workspace-snapshot".to_string(),
        ]
    } else {
        vec![
            "cargo run -- --artifact-audit".to_string(),
            "cargo run -- --operator-console".to_string(),
        ]
    };

    Ok(TrustDecision {
        generated_at: memory::now_unix(),
        trust_decision,
        blockers,
        warnings,
        next_actions,
        auto_promote: false,
        operator_approval_required: true,
    })
}

pub fn print_trust_decision(project_root: &str, memory_root: &str) -> Result<String, String> {
    serde_json::to_string_pretty(&build_trust_decision(project_root, memory_root)?)
        .map_err(|error| format!("failed to serialize trust decision: {error}"))
}

fn latest_rollback_manifest_path(memory_root: &str) -> Option<String> {
    let dir = Path::new(memory_root).join("releases").join("rollback");
    if !dir.exists() {
        return None;
    }
    let mut paths = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect::<Vec<_>>();
    paths.sort();
    paths.last().map(|path| path.display().to_string())
}
