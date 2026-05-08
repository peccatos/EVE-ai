use std::fs;
use std::path::{Path, PathBuf};

#[path = "evolution_test_support.rs"]
mod evolution_test_support;

use eva_runtime_with_task_validator::contracts::EvolutionStatus;
use eva_runtime_with_task_validator::evolution::{CandidateSummary, ReplayResult};
use eva_runtime_with_task_validator::{
    approve_candidate, build_external_patch_package, build_pr_package, build_self_review_package,
    print_future_phases, print_operator_console, print_ops_json, print_proof_json,
    print_proof_report, run_demo, EvolutionReport, MutationContract, MutationKind,
};
use serde_json::Value;

#[test]
fn ops_status_prints_combined_operations_state() {
    let root = temp_runtime_root("phase130-ops-status");
    let output = evolution_test_support::eva_command(&root)
        .args(["--ops-status"])
        .output()
        .expect("run ops-status");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ops_status:"));
    assert!(stdout.contains("preflight="));
    assert!(stdout.contains("auto_promote=false"));
    assert!(!root.join("sandboxes").exists());
    evolution_test_support::remove_root(&root);
}

#[test]
fn ops_json_is_deterministic_rebuildable_enough() {
    let root = temp_runtime_root("phase130-ops-json");
    let first = print_ops_json(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("ops json");
    let second = print_ops_json(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("ops json");
    let mut first_value: Value = serde_json::from_str(&first).expect("parse first");
    let mut second_value: Value = serde_json::from_str(&second).expect("parse second");
    first_value["generated_at"] = Value::from(0_u64);
    second_value["generated_at"] = Value::from(0_u64);
    assert_eq!(first_value, second_value);
    evolution_test_support::remove_root(&root);
}

#[test]
fn pr_package_writes_metadata_only_package_and_does_not_push_merge() {
    let root = temp_runtime_root("phase130-pr-package");
    seed_candidate(
        &root,
        CandidateFixture::useful("pr-candidate", 2_100_100_000),
    );
    let memory = root.join("memory");
    approve_candidate(
        root.to_str().unwrap(),
        memory.to_str().unwrap(),
        "pr-candidate",
        "approved for package",
    )
    .expect("approve");

    let package =
        build_pr_package(root.to_str().unwrap(), memory.to_str().unwrap()).expect("pr package");
    assert!(package.metadata_only);
    assert!(package.operator_approval_required);
    assert!(!package.auto_promote);
    assert!(package.no_network);
    assert!(package.no_push);
    assert!(package.no_merge);
    assert_eq!(package.status, "ready_for_export");
    assert!(memory
        .join("operations/pr_packages")
        .join(format!("{}.json", package.package_id))
        .exists());
    assert!(memory
        .join("operations/pr_packages")
        .join(format!("{}.ru.md", package.package_id))
        .exists());
    evolution_test_support::remove_root(&root);
}

#[test]
fn pr_package_works_in_draft_mode_when_no_release_candidate_exists() {
    let root = temp_runtime_root("phase130-pr-draft");
    let package = build_pr_package(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("draft package");
    assert_eq!(package.status, "draft_no_release_candidate");
    assert!(package.approved_candidate_ids.is_empty());
    evolution_test_support::remove_root(&root);
}

#[test]
fn external_patch_package_rejects_network_url() {
    let root = temp_runtime_root("phase130-ext-network");
    let error = build_external_patch_package(
        root.join("memory").to_str().unwrap(),
        "https://example.com/repo.git",
    )
    .expect_err("network path must fail");
    assert!(error.contains("network"));
    evolution_test_support::remove_root(&root);
}

#[test]
fn external_patch_package_rejects_missing_path() {
    let root = temp_runtime_root("phase130-ext-missing");
    let error = build_external_patch_package(
        root.join("memory").to_str().unwrap(),
        root.join(".eva-operations-tests/missing").to_str().unwrap(),
    )
    .expect_err("missing path must fail");
    assert!(error.contains("does not exist"));
    evolution_test_support::remove_root(&root);
}

#[test]
fn external_patch_package_does_not_mutate_external_repo() {
    let root = temp_runtime_root("phase130-ext-safe");
    let external = root.join(".eva-operations-tests").join("external-repo");
    fs::create_dir_all(external.join("src")).expect("external src");
    fs::write(
        external.join("Cargo.toml"),
        "[package]\nname = \"external_repo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("cargo");
    fs::write(
        external.join("src/lib.rs"),
        "pub fn ext() -> bool { true }\n",
    )
    .expect("lib");
    fs::create_dir_all(external.join(".git")).expect("git");
    let before = fs::read_to_string(external.join("src/lib.rs")).expect("before");

    let package = build_external_patch_package(
        root.join("memory").to_str().unwrap(),
        external.to_str().unwrap(),
    )
    .expect("external package");
    let after = fs::read_to_string(external.join("src/lib.rs")).expect("after");
    assert_eq!(before, after);
    assert!(package.metadata_only);
    assert!(!package.source_mutated);
    evolution_test_support::remove_root(&root);
}

#[test]
fn self_review_package_is_conservative_when_no_approved_release_candidate_exists() {
    let root = temp_runtime_root("phase130-self-review");
    let package = build_self_review_package(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("self review");
    assert!(!package.self_modification_allowed_now);
    assert!(package.self_modification_reason_ru.contains("консерватив"));
    evolution_test_support::remove_root(&root);
}

#[test]
fn self_review_package_forbids_auto_promote_self_apply_network_push_merge() {
    let root = temp_runtime_root("phase130-self-review-forbidden");
    let package = build_self_review_package(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("self review");
    for item in ["auto_promote", "self_apply", "network", "push", "merge"] {
        assert!(package.forbidden_actions.contains(&item.to_string()));
    }
    evolution_test_support::remove_root(&root);
}

#[test]
fn operator_console_prints_next_commands_and_safety_state() {
    let root = temp_runtime_root("phase130-console");
    let console = print_operator_console(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("console");
    assert!(console.contains("Operator Console"));
    assert!(console.contains("Next commands"));
    assert!(console.contains("auto_promote=false"));
    evolution_test_support::remove_root(&root);
}

#[test]
fn proof_report_includes_phase130_capability_flags() {
    let root = temp_runtime_root("phase130-proof");
    let report = print_proof_report(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("proof report");
    assert!(report.contains("operations_runtime_support=true"));
    assert!(report.contains("pr_package_support=true"));
    assert!(report.contains("external_patch_package_support=true"));
    assert!(report.contains("self_review_package_support=true"));
    assert!(report.contains("operator_console_support=true"));
    evolution_test_support::remove_root(&root);
}

#[test]
fn proof_json_includes_phase130_capability_flags() {
    let root = temp_runtime_root("phase130-proof-json");
    let json = print_proof_json(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("proof json");
    let value: Value = serde_json::from_str(&json).expect("parse proof");
    assert_eq!(value["operations_runtime_support"].as_bool(), Some(true));
    assert_eq!(value["pr_package_support"].as_bool(), Some(true));
    assert_eq!(
        value["external_patch_package_support"].as_bool(),
        Some(true)
    );
    assert_eq!(value["self_review_package_support"].as_bool(), Some(true));
    assert_eq!(value["operator_console_support"].as_bool(), Some(true));
    evolution_test_support::remove_root(&root);
}

#[test]
fn demo_includes_operations_section() {
    let root = temp_runtime_root("phase130-demo");
    let output = run_demo(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("demo");
    assert!(output.contains("operations_status:"));
    assert!(output.contains("Operator Console"));
    assert!(!root.join("sandboxes").exists());
    evolution_test_support::remove_root(&root);
}

#[test]
fn future_phase_registry_no_longer_reports_old_phases_as_independently_planned() {
    let output = print_future_phases();
    assert!(
        output.contains("Phase 10.0: CI/PR Integration Runtime status=completed_by_phase_13_0x")
    );
    assert!(output.contains(
        "Phase 13.0: Controlled Self-Modification Review Runtime status=completed_by_phase_13_0x"
    ));
    assert!(output
        .contains("Phase 14.0: Trust + Workspace Recovery Gate status=completed_by_phase_14_0x"));
    assert!(output.contains("Phase 15.0: EVA Runtime v1.0 Candidate status=planned"));
    assert!(!output.contains("Phase 10.0: CI/PR Integration Runtime status=planned"));
    assert!(
        !output.contains("Phase 13.0: Controlled Self-Modification Review Runtime status=planned")
    );
    assert!(!output.contains("Stable Local Release Candidate Flow"));
    assert!(!output.contains("Local CI Runner / Matrix Validation"));
    assert!(!output.contains("External Repo Patch Dry-Run Runtime"));
    assert!(!output.contains("Governance-backed PR Export"));
    assert!(!output.contains("Controlled Daemon Mode"));
}

#[test]
fn no_sandbox_leaks_are_created() {
    let root = temp_runtime_root("phase130-no-leaks");
    let _ = build_self_review_package(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("self review");
    let _ = build_pr_package(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("pr package");
    assert!(!root.join("sandboxes").exists());
    evolution_test_support::remove_root(&root);
}

#[derive(Clone)]
struct CandidateFixture {
    run_id: String,
    kind: MutationKind,
    mutation_class: String,
    useful_change: bool,
    replay_status: String,
    timestamp_unix: u64,
}

impl CandidateFixture {
    fn useful(run_id: &str, timestamp_unix: u64) -> Self {
        Self {
            run_id: run_id.to_string(),
            kind: MutationKind::AddUnitTest,
            mutation_class: "useful".to_string(),
            useful_change: true,
            replay_status: "ok".to_string(),
            timestamp_unix,
        }
    }
}

fn temp_runtime_root(name: &str) -> PathBuf {
    let root = evolution_test_support::unique_evolution_root(name);
    fs::create_dir_all(root.join("src")).expect("src");
    fs::create_dir_all(root.join("tests")).expect("tests");
    fs::create_dir_all(root.join("memory")).expect("memory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"phase130_temp\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\ndoctest = false\n",
    )
    .expect("cargo");
    fs::write(root.join("src/main.rs"), "fn main() {}\n").expect("main");
    fs::write(root.join("src/lib.rs"), "pub fn probe() -> bool { true }\n").expect("lib");
    fs::write(
        root.join("tests/evolution_generated_tests.rs"),
        "#[test]\nfn existing_test() { assert!(true); }\n",
    )
    .expect("tests");
    seed_autonomy_memory(&root);
    root
}

fn seed_candidate(root: &Path, fixture: CandidateFixture) {
    fs::create_dir_all(root.join("memory/candidates")).expect("candidates");
    fs::create_dir_all(root.join("memory/reports")).expect("reports");
    fs::create_dir_all(root.join("memory/replays")).expect("replays");
    fs::write(root.join("memory/regressions.json"), "[]").expect("regressions");
    fs::write(root.join("memory/success_patterns.json"), "[]").expect("success");
    let mutation_kind = format!("{:?}", fixture.kind).to_ascii_lowercase();
    let summary = CandidateSummary {
        run_id: fixture.run_id.clone(),
        mutation_id: format!("mutation-{}", fixture.run_id),
        mutation_digest: format!("digest-{}", fixture.run_id),
        status: EvolutionStatus::Candidate,
        target_file: "tests/evolution_generated_tests.rs".to_string(),
        mutation_kind: mutation_kind.clone(),
        risk: 0.10,
        score: 9.5,
        useful_change: fixture.useful_change,
        non_candidate_reason: None,
        duplicate_rejected: false,
        regression_penalty: 0.0,
        success_bonus: 0.0,
        cargo_check_ok: true,
        cargo_test_ok: true,
        cargo_run_ok: true,
        stdout_digest: String::new(),
        stderr_digest: String::new(),
        stderr_tail: String::new(),
        timestamp_unix: fixture.timestamp_unix,
    };
    let mutation = MutationContract {
        id: summary.mutation_id.clone(),
        kind: fixture.kind,
        target_file: summary.target_file.clone(),
        search: None,
        replace: None,
        append: Some(
            "#[test]\nfn eva_generated_phase130_fixture() { assert!(true); }\n".to_string(),
        ),
        reason: "fixture".to_string(),
        expected_gain: 0.5,
        risk: summary.risk,
    };
    let report = EvolutionReport {
        run_id: fixture.run_id.clone(),
        status: EvolutionStatus::Candidate,
        goal_ru: "fixture".to_string(),
        selected_plan_ru: "fixture".to_string(),
        mutation_ru: "fixture".to_string(),
        target_file: summary.target_file.clone(),
        mutation_kind,
        hypothesis_id: None,
        source_patterns: Vec::new(),
        avoided_risks: Vec::new(),
        recombination_reason_ru: None,
        portfolio_reason_ru: None,
        selected_strategy: Some("ReplaySafety".to_string()),
        policy_reason_ru: Some("fixture".to_string()),
        mutation_class: fixture.mutation_class.clone(),
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
        sandbox_ru: "ok".to_string(),
        checks_ru: "ok".to_string(),
        score_ru: "ok".to_string(),
        candidate_ru: "ok".to_string(),
        replay_ru: "ok".to_string(),
        replay_status: fixture.replay_status.clone(),
        replay_checked_at: Some(fixture.timestamp_unix),
        risk_ru: "ok".to_string(),
        next_step_ru: "ok".to_string(),
    };
    let replay = ReplayResult {
        run_id: fixture.run_id.clone(),
        replay_status: EvolutionStatus::Candidate,
        score: 9.5,
        matches_stored_summary: true,
        cargo_check_ok: true,
        cargo_test_ok: true,
        cargo_run_ok: true,
        stdout_digest: String::new(),
        stderr_digest: String::new(),
        stderr_tail: String::new(),
        sandbox_destroyed: true,
        timestamp_unix: fixture.timestamp_unix,
    };
    fs::write(
        root.join("memory/candidates")
            .join(format!("{}.summary.json", fixture.run_id)),
        serde_json::to_string_pretty(&summary).expect("summary"),
    )
    .expect("summary");
    fs::write(
        root.join("memory/candidates")
            .join(format!("{}.mutation.json", fixture.run_id)),
        serde_json::to_string_pretty(&mutation).expect("mutation"),
    )
    .expect("mutation");
    fs::write(
        root.join("memory/reports")
            .join(format!("{}.report.json", fixture.run_id)),
        serde_json::to_string_pretty(&report).expect("report"),
    )
    .expect("report");
    fs::write(
        root.join("memory/reports")
            .join(format!("{}.ru.md", fixture.run_id)),
        "fixture report",
    )
    .expect("report md");
    fs::write(
        root.join("memory/replays")
            .join(format!("{}.json", fixture.run_id)),
        serde_json::to_string_pretty(&replay).expect("replay"),
    )
    .expect("replay");
}

fn seed_autonomy_memory(root: &Path) {
    fs::create_dir_all(root.join("memory/replays")).expect("replays");
    fs::write(root.join("memory/regressions.json"), "[]").expect("regressions");
    fs::write(root.join("memory/success_patterns.json"), "[]").expect("success");
    let mut lines = Vec::new();
    for index in 0..12 {
        lines.push(format!(
            "{{\"run_id\":\"seed-{index}\",\"plan_id\":null,\"hypothesis_id\":null,\"objective\":\"ImproveTests\",\"graph_evidence\":[],\"recombined_source_patterns\":[],\"recombined_avoided_risks\":[],\"recombination_reason_ru\":null,\"portfolio_reason_ru\":null,\"selected_strategy\":null,\"policy_reason_ru\":null,\"mutation_class\":\"useful\",\"hygiene_warning_ru\":null,\"diversity_bonus\":0.0,\"saturation_penalty\":0.0,\"repeated_target_penalty\":0.0,\"final_recombination_score\":0.0,\"strategy_bonus\":0.0,\"strategy_saturation_penalty\":0.0,\"quality_bonus\":0.0,\"novelty_score\":0.0,\"useful_delta_score\":0.0,\"duplicate_suppression_score\":0.0,\"regression_avoidance_score\":0.0,\"coverage_proxy_score\":0.0,\"quality_score\":0.9,\"final_strategy_score\":0.9,\"mutation_id\":\"m-{index}\",\"mutation_digest\":\"d-{index}\",\"status\":\"candidate\",\"target_file\":\"tests/evolution_generated_tests.rs\",\"mutation_kind\":\"addunittest\",\"risk\":0.10,\"score\":8.50,\"useful_change\":true,\"non_candidate_reason\":null,\"duplicate_rejected\":false,\"regression_penalty\":0.0,\"success_bonus\":0.0,\"cargo_check_ok\":true,\"cargo_test_ok\":true,\"cargo_run_ok\":true,\"retained_in_core\":false,\"sandbox_destroyed\":true,\"stdout_digest\":\"\",\"stderr_digest\":\"\",\"stderr_tail\":\"\",\"timestamp_unix\":{}}}",
            index + 1
        ));
    }
    fs::write(root.join("memory/evolution.jsonl"), lines.join("\n") + "\n").expect("evolution");
    for index in 0..4 {
        let replay = ReplayResult {
            run_id: format!("seed-{index}"),
            replay_status: EvolutionStatus::Candidate,
            score: 8.5,
            matches_stored_summary: true,
            cargo_check_ok: true,
            cargo_test_ok: true,
            cargo_run_ok: true,
            stdout_digest: String::new(),
            stderr_digest: String::new(),
            stderr_tail: String::new(),
            sandbox_destroyed: true,
            timestamp_unix: index + 1,
        };
        fs::write(
            root.join("memory/replays")
                .join(format!("seed-{index}.json")),
            serde_json::to_string_pretty(&replay).expect("replay"),
        )
        .expect("seed replay");
    }
    fs::write(
        root.join("memory/metrics.json"),
        r#"{"total_runs":12,"passed_runs":12,"failed_runs":0,"candidate_count":12,"replay_passed":4,"promoted_count":0,"average_score":8.5,"last_run_id":"seed-11"}"#,
    )
    .expect("metrics");
}
