use std::fs;
use std::path::Path;

use crate::contracts::{sha256_digest, EvidenceBundle};
use crate::evolution::{
    build_artifact_audit, build_determinism_audit, build_operations_report, build_preflight_gate,
    build_proof_report, build_trust_decision, governance_status, latest_proof_snapshot_id,
    latest_release_id, latest_supervised_run_id, memory, print_release_status,
};

pub fn build_evidence_bundle(
    project_root: &str,
    memory_root: &str,
) -> Result<EvidenceBundle, String> {
    let proof = build_proof_report(project_root, memory_root)?;
    let governance = governance_status(project_root, memory_root)?;
    let release_summary = print_release_status(memory_root)?;
    let operations = build_operations_report(project_root, memory_root)?;
    let artifact = build_artifact_audit(project_root)?;
    let determinism = build_determinism_audit(project_root, memory_root)?;
    let preflight = build_preflight_gate(project_root, memory_root)?;
    let trust = build_trust_decision(project_root, memory_root)?;
    let generated_at = memory::now_unix();
    let seed = format!(
        "{}:{}:{}:{}",
        proof.release_count,
        governance.promotion_ready_approved_count,
        preflight.gate_status,
        trust.trust_decision
    );
    let bundle = EvidenceBundle {
        bundle_id: format!("evidence-{}", &sha256_digest(&seed)[..8]),
        generated_at,
        proof_report_summary: format!(
            "runs={} candidates={} promoted={} releases={}",
            proof.total_runs, proof.candidate_count, proof.promoted_candidates, proof.release_count
        ),
        governance_summary: format!(
            "approved={} rejected={} deferred={} ready_approved={}",
            governance.approved_count,
            governance.rejected_count,
            governance.deferred_count,
            governance.promotion_ready_approved_count
        ),
        release_summary,
        operations_summary: format!(
            "health={} preflight={} next={}",
            operations.release_health_grade,
            operations.preflight_gate_status,
            operations.next_safe_operator_action
        ),
        artifact_audit_summary: format!(
            "tracked={} untracked={} sandbox_leaks={} fail={}",
            artifact.tracked_runtime_artifacts.len(),
            artifact.untracked_runtime_artifacts.len(),
            artifact.sandbox_leaks.len(),
            artifact.should_fail_release
        ),
        determinism_audit_summary: format!(
            "checked={} deterministic_enough={} full_source_warnings={}",
            determinism.checked_documents.len(),
            determinism.deterministic_enough,
            determinism.full_source_content_warnings.len()
        ),
        preflight_summary: format!(
            "status={} blockers={} warnings={}",
            preflight.gate_status,
            preflight.blockers.len(),
            preflight.warnings.len()
        ),
        trust_decision_summary: format!(
            "decision={} blockers={} warnings={}",
            trust.trust_decision,
            trust.blockers.len(),
            trust.warnings.len()
        ),
        latest_release_id: latest_release_id(memory_root)?,
        latest_bounded_run_id: latest_json_id(memory_root, "bounded_runs")?,
        latest_supervised_run_id: latest_supervised_run_id(memory_root)?,
        latest_proof_snapshot_id: latest_proof_snapshot_id(memory_root)?,
    };
    write_evidence_bundle(memory_root, &bundle)?;
    Ok(bundle)
}

pub fn print_last_evidence_bundle(memory_root: &str) -> Result<String, String> {
    let bundle = latest_evidence_bundle(memory_root)?
        .ok_or_else(|| "no evidence bundles available".to_string())?;
    serde_json::to_string_pretty(&bundle)
        .map_err(|error| format!("failed to serialize evidence bundle: {error}"))
}

pub fn list_evidence_bundles(memory_root: &str) -> Result<Vec<String>, String> {
    let mut bundles = load_evidence_bundles(memory_root)?;
    bundles.sort_by(|left, right| {
        left.generated_at
            .cmp(&right.generated_at)
            .then_with(|| left.bundle_id.cmp(&right.bundle_id))
    });
    Ok(bundles.into_iter().map(|item| item.bundle_id).collect())
}

pub fn latest_evidence_bundle_id(memory_root: &str) -> Result<Option<String>, String> {
    Ok(latest_evidence_bundle(memory_root)?.map(|bundle| bundle.bundle_id))
}

fn write_evidence_bundle(memory_root: &str, bundle: &EvidenceBundle) -> Result<(), String> {
    let dir = Path::new(memory_root).join("evidence");
    fs::create_dir_all(&dir).map_err(|error| format!("failed to create evidence dir: {error}"))?;
    memory::write_json(dir.join(format!("{}.json", bundle.bundle_id)), bundle)
}

fn load_evidence_bundles(memory_root: &str) -> Result<Vec<EvidenceBundle>, String> {
    let dir = Path::new(memory_root).join("evidence");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut bundles = Vec::new();
    for entry in
        fs::read_dir(&dir).map_err(|error| format!("failed to read evidence dir: {error}"))?
    {
        let entry = entry.map_err(|error| format!("failed to read evidence entry: {error}"))?;
        let path = entry.path();
        if !path.extension().is_some_and(|ext| ext == "json") {
            continue;
        }
        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read evidence bundle: {error}"))?;
        let bundle: EvidenceBundle = serde_json::from_str(&contents)
            .map_err(|error| format!("failed to parse evidence bundle: {error}"))?;
        bundles.push(bundle);
    }
    Ok(bundles)
}

fn latest_evidence_bundle(memory_root: &str) -> Result<Option<EvidenceBundle>, String> {
    let mut bundles = load_evidence_bundles(memory_root)?;
    bundles.sort_by(|left, right| {
        left.generated_at
            .cmp(&right.generated_at)
            .then_with(|| left.bundle_id.cmp(&right.bundle_id))
    });
    Ok(bundles.pop())
}

fn latest_json_id(memory_root: &str, dir_name: &str) -> Result<Option<String>, String> {
    let dir = Path::new(memory_root).join(dir_name);
    if !dir.exists() {
        return Ok(None);
    }
    let mut entries = fs::read_dir(dir)
        .map_err(|error| format!("failed to read {dir_name}: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .filter_map(|path| {
            let modified = fs::metadata(&path).ok()?.modified().ok()?;
            let modified = modified
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_secs();
            let id = path.file_stem()?.to_str()?.to_string();
            Some((modified, id))
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1)));
    Ok(entries.into_iter().next().map(|(_, id)| id))
}
