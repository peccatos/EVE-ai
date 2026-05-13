use std::fs;
use std::path::{Path, PathBuf};

#[path = "evolution_test_support.rs"]
mod evolution_test_support;

use eva_runtime_with_task_validator::{
    approve_release_candidate, build_release_candidate_state, build_runtime_validation,
    classify_run_outcome, load_metrics, load_tui_state, refresh_metrics, refresh_promotion_queue,
    run_tui, CandidateState, EvolutionLogEntry, EvolutionRunOutcome, EvolutionStatus,
};

#[test]
fn tui_state_loads_from_missing_files_without_panic() {
    let root = temp_root("phase151-tui-missing");
    let state = load_tui_state(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    );
    assert_eq!(state.dashboard.runtime_validation_status, "warn");
    assert_eq!(state.dashboard.last_replay_status, "missing");
    assert!(state.runs.is_empty());
    assert!(state.candidates.is_empty());
    let output = run_tui(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("tui snapshot");
    assert!(output.contains("EVA Operator TUI"));
    assert!(output.contains("Dashboard"));
    evolution_test_support::remove_root(&root);
}

#[test]
fn duplicate_rejection_is_safety_outcome_not_failed_run() {
    let root = temp_root("phase151-duplicate-metrics");
    write_log(
        &root,
        &log_entry(
            "dup-run",
            EvolutionStatus::Failed,
            false,
            true,
            Some("duplicate"),
        ),
    );
    let metrics = refresh_metrics(root.join("memory").to_str().unwrap()).expect("metrics");
    assert_eq!(metrics.total_runs, 1);
    assert_eq!(metrics.failed_runs, 0);
    assert_eq!(metrics.duplicate_rejected_runs, 1);
    assert_eq!(metrics.safety_rejected_runs, 1);
    let entry = log_entry(
        "dup-classify",
        EvolutionStatus::Failed,
        false,
        true,
        Some("duplicate"),
    );
    assert_eq!(
        classify_run_outcome(&entry),
        EvolutionRunOutcome::DuplicateSafetyRejection
    );
    evolution_test_support::remove_root(&root);
}

#[test]
fn real_cargo_failure_counts_as_failed_run() {
    let root = temp_root("phase151-cargo-failure");
    write_log(
        &root,
        &log_entry("cargo-fail", EvolutionStatus::Failed, false, false, None),
    );
    let metrics = refresh_metrics(root.join("memory").to_str().unwrap()).expect("metrics");
    assert_eq!(metrics.failed_runs, 1);
    assert_eq!(metrics.cargo_gate_failed_runs, 1);
    assert_eq!(metrics.safety_rejected_runs, 0);
    evolution_test_support::remove_root(&root);
}

#[test]
fn policy_rejection_does_not_count_as_runtime_failure() {
    let root = temp_root("phase151-policy-rejection");
    write_log(
        &root,
        &log_entry(
            "policy-reject",
            EvolutionStatus::Failed,
            true,
            false,
            Some("policy_rejection"),
        ),
    );
    let metrics = load_metrics(root.join("memory").to_str().unwrap()).expect("metrics");
    assert_eq!(metrics.failed_runs, 0);
    assert_eq!(metrics.policy_rejected_runs, 1);
    assert_eq!(metrics.safety_rejected_runs, 1);
    evolution_test_support::remove_root(&root);
}

#[test]
fn candidate_queue_marks_failed_replay_as_unreplayable() {
    let root = temp_root("phase151-queue-unreplayable");
    seed_candidate(&root, "bad-replay", "failed", true, true, false);
    let queue = refresh_promotion_queue(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("queue");
    let item = queue
        .items
        .iter()
        .find(|item| item.run_id == "bad-replay")
        .unwrap();
    assert_eq!(item.candidate_state, CandidateState::Unreplayable);
    assert_eq!(queue.summary.unreplayable_candidates, 1);
    evolution_test_support::remove_root(&root);
}

#[test]
fn candidate_queue_marks_missing_evidence_as_blocked_or_stale() {
    let root = temp_root("phase151-queue-stale");
    seed_candidate_summary_only(&root, "missing-evidence", true, true, false);
    let queue = refresh_promotion_queue(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("queue");
    let item = queue
        .items
        .iter()
        .find(|item| item.run_id == "missing-evidence")
        .unwrap();
    assert!(
        matches!(
            item.candidate_state,
            CandidateState::Stale | CandidateState::Blocked | CandidateState::Unreplayable
        ),
        "state={:?} reason={} blockers={:?}",
        item.candidate_state,
        item.candidate_state_reason,
        item.promotion_blockers
    );
    evolution_test_support::remove_root(&root);
}

#[test]
fn runtime_validation_warns_without_approved_release_candidate_and_blocks_sandbox_leak() {
    let root = temp_root("phase151-validation");
    let validation = build_runtime_validation(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("validation");
    assert_eq!(validation.status, "warn");
    assert!(validation
        .missing_green_conditions
        .contains(&"approved_release_candidate".to_string()));
    fs::create_dir_all(root.join("sandboxes/leak")).expect("leak");
    let blocked = build_runtime_validation(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("blocked validation");
    assert_eq!(blocked.status, "blocked");
    evolution_test_support::remove_root(&root);
}

#[test]
fn release_candidate_approval_refuses_missing_candidate() {
    let root = temp_root("phase151-release-missing");
    let err = approve_release_candidate(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
        "missing",
    )
    .expect_err("missing candidate must fail");
    assert!(err.contains("not found") || err.contains("missing"));
    let state = build_release_candidate_state(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("state");
    assert!(!state.operator_approved);
    evolution_test_support::remove_root(&root);
}

fn temp_root(name: &str) -> PathBuf {
    let root = evolution_test_support::unique_evolution_root(name);
    fs::create_dir_all(root.join("src")).expect("src");
    fs::create_dir_all(root.join("tests")).expect("tests");
    fs::create_dir_all(root.join("memory")).expect("memory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname=\"phase151_temp\"\nversion=\"0.1.0\"\nedition=\"2021\"\n\n[lib]\ndoctest=false\n",
    )
    .expect("cargo");
    fs::write(root.join("src/lib.rs"), "pub fn probe() -> bool { true }\n").expect("lib");
    fs::write(root.join("memory/regressions.json"), "[]").expect("regressions");
    fs::write(root.join("memory/success_patterns.json"), "[]").expect("success");
    root
}

fn write_log(root: &Path, entry: &EvolutionLogEntry) {
    fs::create_dir_all(root.join("memory")).expect("memory");
    fs::write(
        root.join("memory/evolution.jsonl"),
        format!("{}\n", serde_json::to_string(entry).expect("log")),
    )
    .expect("write log");
}

fn log_entry(
    run_id: &str,
    status: EvolutionStatus,
    cargo_ok: bool,
    duplicate: bool,
    reason: Option<&str>,
) -> EvolutionLogEntry {
    EvolutionLogEntry {
        run_id: run_id.to_string(),
        plan_id: None,
        hypothesis_id: None,
        objective: None,
        graph_evidence: Vec::new(),
        recombined_source_patterns: Vec::new(),
        recombined_avoided_risks: Vec::new(),
        recombination_reason_ru: None,
        portfolio_reason_ru: None,
        selected_strategy: None,
        policy_reason_ru: None,
        mutation_class: "useful".to_string(),
        hygiene_warning_ru: None,
        diversity_bonus: 0.0,
        saturation_penalty: 0.0,
        repeated_target_penalty: 0.0,
        final_recombination_score: 0.0,
        strategy_bonus: 0.0,
        strategy_saturation_penalty: 0.0,
        quality_bonus: 0.0,
        novelty_score: 0.0,
        useful_delta_score: 0.0,
        duplicate_suppression_score: 0.0,
        regression_avoidance_score: 0.0,
        coverage_proxy_score: 0.0,
        quality_score: 0.9,
        final_strategy_score: 0.9,
        mutation_id: format!("mutation-{run_id}"),
        mutation_digest: format!("digest-{run_id}"),
        status,
        target_file: "tests/evolution_generated_tests.rs".to_string(),
        mutation_kind: "addunittest".to_string(),
        risk: 0.1,
        score: 8.0,
        useful_change: true,
        non_candidate_reason: reason.map(str::to_string),
        duplicate_rejected: duplicate,
        regression_penalty: 0.0,
        success_bonus: 0.0,
        cargo_check_ok: cargo_ok,
        cargo_test_ok: cargo_ok,
        cargo_run_ok: cargo_ok,
        retained_in_core: false,
        sandbox_destroyed: true,
        stdout_digest: String::new(),
        stderr_digest: String::new(),
        stderr_tail: String::new(),
        timestamp_unix: 1,
    }
}

fn seed_candidate(
    root: &Path,
    run_id: &str,
    replay_status: &str,
    cargo_test_ok: bool,
    cargo_run_ok: bool,
    duplicate: bool,
) {
    seed_candidate_summary_only(root, run_id, cargo_test_ok, cargo_run_ok, duplicate);
    fs::write(
        root.join("memory/candidates").join(format!("{run_id}.mutation.json")),
        r##"{"id":"m","kind":"add_unit_test","target_file":"tests/evolution_generated_tests.rs","search":null,"replace":null,"append":"#[test]\nfn eva_generated_probe() { assert!(true); }\n","reason":"test fixture","expected_gain":0.5,"risk":0.1}"##,
    )
    .expect("mutation");
    fs::create_dir_all(root.join("memory/reports")).expect("reports");
    fs::write(
        root.join("memory/reports").join(format!("{run_id}.ru.md")),
        "report",
    )
    .expect("report md");
    fs::write(
        root.join("memory/reports").join(format!("{run_id}.report.json")),
        format!(
            r#"{{"run_id":"{run_id}","status":"candidate","goal_ru":"","selected_plan_ru":"","mutation_ru":"","target_file":"tests/evolution_generated_tests.rs","mutation_kind":"addunittest","mutation_class":"useful","sandbox_ru":"","checks_ru":"","score_ru":"","candidate_ru":"","replay_ru":"","replay_status":"{replay_status}","risk_ru":"","next_step_ru":"","quality_score":0.9,"novelty_score":0.9,"useful_delta_score":0.9,"regression_avoidance_score":0.9}}"#
        ),
    )
    .expect("report json");
    fs::create_dir_all(root.join("memory/replays")).expect("replays");
    fs::write(
        root.join("memory/replays").join(format!("{run_id}.json")),
        format!(
            r#"{{"run_id":"{run_id}","replay_status":"{}","score":8.0,"matches_stored_summary":{},"cargo_check_ok":{},"cargo_test_ok":{},"cargo_run_ok":{},"stdout_digest":"","stderr_digest":"","stderr_tail":"","sandbox_destroyed":true,"timestamp_unix":1}}"#,
            if replay_status == "ok" { "candidate" } else { "failed" },
            replay_status == "ok",
            cargo_test_ok,
            cargo_test_ok,
            cargo_run_ok
        ),
    )
    .expect("replay");
}

fn seed_candidate_summary_only(
    root: &Path,
    run_id: &str,
    cargo_test_ok: bool,
    cargo_run_ok: bool,
    duplicate: bool,
) {
    fs::create_dir_all(root.join("memory/candidates")).expect("candidates");
    fs::write(
        root.join("memory/candidates").join(format!("{run_id}.summary.json")),
        format!(
            r#"{{"run_id":"{run_id}","mutation_id":"m","mutation_digest":"d-{run_id}","status":"candidate","target_file":"tests/evolution_generated_tests.rs","mutation_kind":"addunittest","risk":0.1,"score":8.0,"useful_change":true,"duplicate_rejected":{},"regression_penalty":0.0,"success_bonus":0.0,"cargo_check_ok":true,"cargo_test_ok":{},"cargo_run_ok":{},"stdout_digest":"","stderr_digest":"","stderr_tail":"","timestamp_unix":1}}"#,
            duplicate, cargo_test_ok, cargo_run_ok
        ),
    )
    .expect("summary");
}
