use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use eva_runtime_with_task_validator::contracts::{EvolutionLogEntry, EvolutionStatus};
use eva_runtime_with_task_validator::evolution::memory::{
    load_candidate_summary, maybe_store_candidate, store_candidate, CandidateSummary,
};
use eva_runtime_with_task_validator::{
    check_promotion_gate, ingest_repo_patterns, replay_candidate, update_graph_for_evolution,
    MutationContract, MutationKind,
};

fn mutation(target_file: &str, risk: f32) -> MutationContract {
    MutationContract {
        id: "phase25-test-mutation".to_string(),
        kind: MutationKind::AppendComment,
        target_file: target_file.to_string(),
        search: None,
        replace: None,
        append: Some("// phase25 test mutation".to_string()),
        reason: "test phase 2.5 candidate flow".to_string(),
        expected_gain: 0.1,
        risk,
    }
}

fn log_entry(run_id: &str, score: f32, status: EvolutionStatus) -> EvolutionLogEntry {
    EvolutionLogEntry {
        run_id: run_id.to_string(),
        plan_id: None,
        hypothesis_id: None,
        objective: None,
        graph_evidence: Vec::new(),
        mutation_id: "phase25-test-mutation".to_string(),
        status,
        target_file: "src/probe.rs".to_string(),
        mutation_kind: "appendcomment".to_string(),
        risk: 0.1,
        score,
        cargo_check_ok: score >= 3.0,
        cargo_test_ok: score >= 7.0,
        cargo_run_ok: score >= 10.0,
        retained_in_core: false,
        sandbox_destroyed: true,
        stdout_digest: "stdout".to_string(),
        stderr_digest: "stderr".to_string(),
        stderr_tail: String::new(),
        timestamp_unix: 1,
    }
}

#[test]
fn candidate_stored_only_when_score_at_least_five() {
    let root = temp_dir("candidate-store");
    fs::create_dir_all(&root).expect("create temp memory");

    let accepted = log_entry("accepted", 5.0, EvolutionStatus::Candidate);
    assert!(maybe_store_candidate(
        root.to_str().unwrap(),
        &accepted,
        &mutation("src/probe.rs", 0.1)
    )
    .expect("store candidate"));
    assert!(root.join("candidates/accepted.mutation.json").exists());
    assert!(root.join("candidates/accepted.summary.json").exists());

    let failed = log_entry("failed", 4.9, EvolutionStatus::Failed);
    assert!(!maybe_store_candidate(
        root.to_str().unwrap(),
        &failed,
        &mutation("src/probe.rs", 0.1)
    )
    .expect("skip failed candidate"));
    assert!(!root.join("candidates/failed.mutation.json").exists());

    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn replay_reruns_candidate_in_fresh_sandbox() {
    let project = temp_crate("replay-project");
    let memory = temp_dir("replay-memory");
    fs::create_dir_all(&memory).expect("create memory");

    let entry = log_entry("replay-run", 10.0, EvolutionStatus::Candidate);
    store_candidate(
        memory.to_str().unwrap(),
        &entry,
        &mutation("src/probe.rs", 0.1),
    )
    .expect("store candidate");

    replay_candidate(
        project.to_str().unwrap(),
        memory.to_str().unwrap(),
        "replay-run",
    )
    .expect("replay candidate");

    let replay_path = memory.join("replays/replay-run.json");
    assert!(replay_path.exists());
    let replay = fs::read_to_string(replay_path).expect("read replay");
    assert!(replay.contains("\"matches_stored_summary\": true"));
    assert!(!project
        .join("src/probe.rs")
        .read_to_string_lossy()
        .contains("phase25"));

    fs::remove_dir_all(project).expect("cleanup project");
    fs::remove_dir_all(memory).expect("cleanup memory");
}

#[test]
fn promotion_rejects_high_risk_and_core_targets() {
    assert!(!check_promotion_gate(&mutation("src/probe.rs", 0.26), 10.0).allowed);
    assert!(!check_promotion_gate(&mutation("src/core/belief_state.rs", 0.1), 10.0).allowed);
    assert!(!check_promotion_gate(&mutation("src/main.rs", 0.1), 10.0).allowed);
    assert!(!check_promotion_gate(&mutation("src/lib.rs", 0.1), 10.0).allowed);
}

#[test]
fn graph_updates_after_successful_evolution() {
    let memory = temp_dir("graph-memory");
    let entry = log_entry("graph-run", 10.0, EvolutionStatus::Candidate);

    update_graph_for_evolution(memory.to_str().unwrap(), &entry).expect("update graph");

    let graph = fs::read_to_string(memory.join("graph.json")).expect("read graph");
    assert!(graph.contains("mutation:phase25-test-mutation"));
    assert!(graph.contains("file:src/probe.rs"));
    assert!(graph.contains("score_band:high"));

    fs::remove_dir_all(memory).expect("cleanup memory");
}

#[test]
fn repo_ingestion_does_not_mutate_source_repo() {
    let project = temp_crate("ingest-project");
    let memory = temp_dir("ingest-memory");
    let before = fs::read_to_string(project.join("src/probe.rs")).expect("read before");

    ingest_repo_patterns(project.to_str().unwrap(), memory.to_str().unwrap()).expect("ingest repo");

    let after = fs::read_to_string(project.join("src/probe.rs")).expect("read after");
    assert_eq!(before, after);
    let graph = fs::read_to_string(memory.join("graph.json")).expect("read graph");
    assert!(graph.contains("pattern:function:probe"));

    fs::remove_dir_all(project).expect("cleanup project");
    fs::remove_dir_all(memory).expect("cleanup memory");
}

#[test]
fn candidate_summary_round_trips() {
    let memory = temp_dir("summary-memory");
    let entry = log_entry("summary-run", 7.0, EvolutionStatus::Candidate);
    store_candidate(
        memory.to_str().unwrap(),
        &entry,
        &mutation("src/probe.rs", 0.1),
    )
    .expect("store candidate");

    let summary: CandidateSummary =
        load_candidate_summary(memory.to_str().unwrap(), "summary-run").expect("load summary");
    assert_eq!(summary.run_id, "summary-run");
    assert_eq!(summary.score, 7.0);

    fs::remove_dir_all(memory).expect("cleanup memory");
}

fn temp_crate(name: &str) -> PathBuf {
    let root = temp_dir(name);
    fs::create_dir_all(root.join("src")).expect("create crate src");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"eva_phase25_temp\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write cargo toml");
    fs::write(root.join("src/main.rs"), "fn main() {}\n").expect("write main");
    fs::write(root.join("src/probe.rs"), "pub fn probe() {}\n").expect("write probe");
    root
}

fn temp_dir(name: &str) -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_millis();
    std::env::temp_dir().join(format!("{name}-{}-{millis}", std::process::id()))
}

trait ReadToStringLossy {
    fn read_to_string_lossy(&self) -> String;
}

impl ReadToStringLossy for Path {
    fn read_to_string_lossy(&self) -> String {
        fs::read_to_string(self).unwrap_or_default()
    }
}
