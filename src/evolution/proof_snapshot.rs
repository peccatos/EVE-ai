use std::fs;
use std::path::Path;

use crate::contracts::ProofSnapshot;
use crate::evolution::{
    build_proof_report, governance_status, latest_supervised_run_id, memory, release_proposal_count,
};

pub fn build_proof_snapshot(
    project_root: &str,
    memory_root: &str,
) -> Result<ProofSnapshot, String> {
    let proof = build_proof_report(project_root, memory_root)?;
    let governance = governance_status(project_root, memory_root)?;
    let snapshot = ProofSnapshot {
        snapshot_id: format!("snapshot-{}", memory::now_unix()),
        total_runs: proof.total_runs,
        candidate_count: proof.candidate_count,
        replay_passed: proof.replay_passed_candidates,
        promoted_count: proof.promoted_candidates,
        promotion_queue_ready: proof.ready_candidates,
        promotion_queue_blocked: proof.blocked_candidates,
        approved_count: governance.approved_count,
        rejected_count: governance.rejected_count,
        deferred_count: governance.deferred_count,
        release_proposal_count: release_proposal_count(memory_root)?,
        latest_bounded_run_id: proof.latest_bounded_run_id.clone(),
        latest_supervised_run_id: latest_supervised_run_id(memory_root)?,
        auto_promote: false,
        operator_approval_required: true,
        created_at: memory::now_unix(),
    };
    write_proof_snapshot(memory_root, &snapshot)?;
    Ok(snapshot)
}

pub fn print_proof_snapshot(project_root: &str, memory_root: &str) -> Result<String, String> {
    let snapshot = build_proof_snapshot(project_root, memory_root)?;
    Ok(render_proof_snapshot_markdown(&snapshot))
}

pub fn print_proof_snapshot_json(project_root: &str, memory_root: &str) -> Result<String, String> {
    let snapshot = build_proof_snapshot(project_root, memory_root)?;
    serde_json::to_string_pretty(&snapshot)
        .map_err(|error| format!("failed to serialize proof snapshot: {error}"))
}

pub fn latest_proof_snapshot_id(memory_root: &str) -> Result<Option<String>, String> {
    let dir = Path::new(memory_root)
        .join("governance")
        .join("proof_snapshots");
    if !dir.exists() {
        return Ok(None);
    }
    let mut ids = fs::read_dir(dir)
        .map_err(|error| format!("failed to read proof snapshots: {error}"))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .path()
                .file_stem()
                .and_then(|name| name.to_str())
                .map(str::to_string)
        })
        .collect::<Vec<_>>();
    ids.sort();
    Ok(ids.pop())
}

fn write_proof_snapshot(memory_root: &str, snapshot: &ProofSnapshot) -> Result<(), String> {
    let dir = Path::new(memory_root)
        .join("governance")
        .join("proof_snapshots");
    fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create proof snapshot dir: {error}"))?;
    memory::write_json(dir.join(format!("{}.json", snapshot.snapshot_id)), snapshot)?;
    fs::write(
        dir.join(format!("{}.ru.md", snapshot.snapshot_id)),
        render_proof_snapshot_markdown(snapshot),
    )
    .map_err(|error| format!("failed to write proof snapshot markdown: {error}"))
}

fn render_proof_snapshot_markdown(snapshot: &ProofSnapshot) -> String {
    format!(
        "# Governance Proof Snapshot EVA\n\nsnapshot_id={}\ntotal_runs={}\ncandidate_count={}\nreplay_passed={}\npromoted_count={}\npromotion_queue_ready={}\npromotion_queue_blocked={}\napproved_count={}\nrejected_count={}\ndeferred_count={}\nrelease_proposal_count={}\nlatest_bounded_run_id={}\nlatest_supervised_run_id={}\nauto_promote={}\noperator_approval_required={}\n",
        snapshot.snapshot_id,
        snapshot.total_runs,
        snapshot.candidate_count,
        snapshot.replay_passed,
        snapshot.promoted_count,
        snapshot.promotion_queue_ready,
        snapshot.promotion_queue_blocked,
        snapshot.approved_count,
        snapshot.rejected_count,
        snapshot.deferred_count,
        snapshot.release_proposal_count,
        snapshot.latest_bounded_run_id.as_deref().unwrap_or("none"),
        snapshot.latest_supervised_run_id.as_deref().unwrap_or("none"),
        snapshot.auto_promote,
        snapshot.operator_approval_required,
    )
}
