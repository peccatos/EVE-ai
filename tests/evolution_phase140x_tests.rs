use std::fs;
use std::path::{Path, PathBuf};

#[path = "evolution_test_support.rs"]
mod evolution_test_support;

use eva_runtime_with_task_validator::{
    build_capability_policy, build_evidence_bundle, build_preflight_gate_v3,
    build_recovery_manifest, build_trust_decision, build_workspace_snapshot, print_future_phases,
    print_operator_console, print_proof_json, print_proof_report, print_trust_proof_report,
    run_demo,
};
use serde_json::Value;

#[test]
fn capability_policy_denies_push_merge_auto_promote_self_apply_external_mutation() {
    let policy = build_capability_policy();
    assert!(!policy.auto_promote_allowed);
    assert!(!policy.network_push_allowed);
    assert!(!policy.merge_allowed);
    assert!(!policy.external_repo_mutation_allowed);
    assert!(!policy.self_apply_allowed);
    assert!(!policy.source_mutation_without_approval_allowed);
    assert!(policy.metadata_generation_allowed);
    assert!(policy.local_read_only_inspection_allowed);
    assert!(policy.sandboxed_validation_allowed_when_isolated);
}

#[test]
fn trust_decision_denies_unsafe_states_and_warns_when_no_approved_release_candidate_exists() {
    let root = temp_runtime_root("phase140-trust");
    let trust = build_trust_decision(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("trust");
    assert_eq!(trust.trust_decision, "warn");
    assert!(trust
        .warnings
        .contains(&"no_approved_release_candidate".to_string()));
    assert!(!trust.auto_promote);
    assert!(trust.operator_approval_required);
    evolution_test_support::remove_root(&root);
}

#[test]
fn evidence_bundle_is_metadata_only_and_does_not_include_full_source_content() {
    let root = temp_runtime_root("phase140-evidence");
    let bundle = build_evidence_bundle(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("bundle");
    let json = serde_json::to_string_pretty(&bundle).expect("serialize");
    assert!(!json.contains("pub fn"));
    assert!(!json.contains("fn main()"));
    assert!(root
        .join("memory/evidence")
        .join(format!("{}.json", bundle.bundle_id))
        .exists());
    evolution_test_support::remove_root(&root);
}

#[test]
fn workspace_snapshot_records_branch_head_status_counts_without_source_content() {
    let root = temp_runtime_root("phase140-workspace");
    let snapshot = build_workspace_snapshot(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("snapshot");
    let json = serde_json::to_string_pretty(&snapshot).expect("serialize");
    assert!(!snapshot.git_branch.is_empty());
    assert!(!snapshot.git_head.is_empty());
    assert!(!json.contains("pub fn probe"));
    assert!(snapshot.tracked_count >= 1);
    evolution_test_support::remove_root(&root);
}

#[test]
fn recovery_manifest_is_metadata_only_and_contains_manual_recovery_instructions() {
    let root = temp_runtime_root("phase140-recovery");
    let _ = build_workspace_snapshot(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("snapshot");
    let manifest = build_recovery_manifest(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("manifest");
    let json = serde_json::to_string_pretty(&manifest).expect("serialize");
    assert!(manifest
        .recovery_steps
        .iter()
        .any(|step| step.contains("git status --short")));
    assert!(manifest
        .prohibited_automatic_actions
        .contains(&"push".to_string()));
    assert!(!json.contains("pub fn"));
    evolution_test_support::remove_root(&root);
}

#[test]
fn preflight_gate_v3_composes_policy_trust_evidence_snapshot_recovery() {
    let root = temp_runtime_root("phase140-gate-v3");
    let gate = build_preflight_gate_v3(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("gate");
    assert_eq!(gate.status, "warn");
    assert!(gate
        .warnings
        .contains(&"no_approved_release_candidate".to_string()));
    assert!(gate
        .next_actions
        .iter()
        .any(|item| item.contains("trust-proof-report")));
    evolution_test_support::remove_root(&root);
}

#[test]
fn proof_report_exposes_phase14_capability_flags() {
    let root = temp_runtime_root("phase140-proof");
    let report = print_proof_report(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("report");
    assert!(report.contains("capability_policy_support=true"));
    assert!(report.contains("trust_decision_support=true"));
    assert!(report.contains("evidence_bundle_support=true"));
    assert!(report.contains("workspace_snapshot_support=true"));
    assert!(report.contains("recovery_manifest_support=true"));
    assert!(report.contains("preflight_gate_v3_support=true"));
    assert!(report.contains("trust_proof_report_support=true"));
    let json = print_proof_json(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("json");
    let value: Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(value["capability_policy_support"].as_bool(), Some(true));
    evolution_test_support::remove_root(&root);
}

#[test]
fn demo_and_operator_console_include_phase14_status() {
    let root = temp_runtime_root("phase140-demo");
    let demo = run_demo(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("demo");
    let console = print_operator_console(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("console");
    let trust_report = print_trust_proof_report(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("trust proof");
    assert!(demo.contains("trust_decision:"));
    assert!(demo.contains("preflight_gate_v3:"));
    assert!(demo.contains("Phase 14.0: Trust + Workspace Recovery Gate"));
    assert!(demo.contains("status=completed_by_phase_14_0x"));
    assert!(demo.contains("Phase 15.0: EVA Runtime v1.0 Candidate status=completed_by_phase_15_0x"));
    assert!(console.contains("capability_policy:"));
    assert!(console.contains("preflight_gate_v3:"));
    assert!(console.contains("runtime_candidate: status="));
    assert!(console.contains("Phase 14.0: Trust + Workspace Recovery Gate"));
    assert!(console.contains("Phase 15.0: EVA Runtime v1.0 Candidate"));
    assert!(trust_report.contains("Trust Proof Report"));
    evolution_test_support::remove_root(&root);
}

#[test]
fn future_phase_registry_marks_phase15_completed_with_no_remaining_planned_tail() {
    let output = print_future_phases();
    assert!(output.contains(
        "Phase 13.0: Controlled Self-Modification Review Runtime status=completed_by_phase_13_0x"
    ));
    assert!(output
        .contains("Phase 14.0: Trust + Workspace Recovery Gate status=completed_by_phase_14_0x"));
    assert!(
        output.contains("Phase 15.0: EVA Runtime v1.0 Candidate status=completed_by_phase_15_0x")
    );
    assert!(!output.contains("Phase 14.0: Trust + Workspace Recovery Gate status=planned"));
    assert!(!output.contains("Phase 15.0: EVA Runtime v1.0 Candidate status=planned"));
    assert!(!output.contains("Stable Local Release Candidate Flow"));
    assert!(!output.contains("Local CI Runner / Matrix Validation"));
    assert!(!output.contains("External Repo Patch Dry-Run Runtime"));
    assert!(!output.contains("Governance-backed PR Export"));
    assert!(!output.contains("Controlled Daemon Mode"));
}

#[test]
fn no_sandbox_leaks_and_no_source_mutation_caused_by_metadata_commands() {
    let root = temp_runtime_root("phase140-safe");
    let before = fs::read_to_string(root.join("src/lib.rs")).expect("before");
    let _ = build_workspace_snapshot(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("snapshot");
    let _ = build_evidence_bundle(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("bundle");
    let _ = build_recovery_manifest(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("manifest");
    let after = fs::read_to_string(root.join("src/lib.rs")).expect("after");
    assert_eq!(before, after);
    assert!(!root.join("sandboxes").exists());
    evolution_test_support::remove_root(&root);
}

fn temp_runtime_root(name: &str) -> PathBuf {
    let root = evolution_test_support::unique_evolution_root(name);
    fs::create_dir_all(root.join("src")).expect("src");
    fs::create_dir_all(root.join("tests")).expect("tests");
    fs::create_dir_all(root.join("memory")).expect("memory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"phase140_temp\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\ndoctest = false\n",
    )
    .expect("cargo");
    fs::write(root.join("src/main.rs"), "fn main() {}\n").expect("main");
    fs::write(root.join("src/lib.rs"), "pub fn probe() -> bool { true }\n").expect("lib");
    fs::write(
        root.join("tests/evolution_generated_tests.rs"),
        "#[test]\nfn existing_test() { assert!(true); }\n",
    )
    .expect("tests");
    fs::write(root.join("memory/regressions.json"), "[]").expect("regressions");
    fs::write(root.join("memory/success_patterns.json"), "[]").expect("success");
    fs::write(
        root.join("memory/evolution.jsonl"),
        "{\"run_id\":\"seed-1\",\"plan_id\":null,\"hypothesis_id\":null,\"objective\":\"ImproveTests\",\"graph_evidence\":[],\"recombined_source_patterns\":[],\"recombined_avoided_risks\":[],\"recombination_reason_ru\":null,\"portfolio_reason_ru\":null,\"selected_strategy\":null,\"policy_reason_ru\":null,\"mutation_class\":\"useful\",\"hygiene_warning_ru\":null,\"diversity_bonus\":0.0,\"saturation_penalty\":0.0,\"repeated_target_penalty\":0.0,\"final_recombination_score\":0.0,\"strategy_bonus\":0.0,\"strategy_saturation_penalty\":0.0,\"quality_bonus\":0.0,\"novelty_score\":0.0,\"useful_delta_score\":0.0,\"duplicate_suppression_score\":0.0,\"regression_avoidance_score\":0.0,\"coverage_proxy_score\":0.0,\"quality_score\":0.9,\"final_strategy_score\":0.9,\"mutation_id\":\"m-1\",\"mutation_digest\":\"d-1\",\"status\":\"candidate\",\"target_file\":\"tests/evolution_generated_tests.rs\",\"mutation_kind\":\"addunittest\",\"risk\":0.10,\"score\":8.50,\"useful_change\":true,\"non_candidate_reason\":null,\"duplicate_rejected\":false,\"regression_penalty\":0.0,\"success_bonus\":0.0,\"cargo_check_ok\":true,\"cargo_test_ok\":true,\"cargo_run_ok\":true,\"retained_in_core\":false,\"sandbox_destroyed\":true,\"stdout_digest\":\"\",\"stderr_digest\":\"\",\"stderr_tail\":\"\",\"timestamp_unix\":1}\n",
    )
    .expect("evolution");
    fs::write(
        root.join("memory/metrics.json"),
        r#"{"total_runs":1,"passed_runs":1,"failed_runs":0,"candidate_count":1,"replay_passed":0,"promoted_count":0,"average_score":8.5,"last_run_id":"seed-1"}"#,
    )
    .expect("metrics");
    init_git_repo(&root);
    root
}

fn init_git_repo(root: &Path) {
    std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(root)
        .status()
        .expect("git init");
    std::process::Command::new("git")
        .args(["config", "user.email", "eva@example.local"])
        .current_dir(root)
        .status()
        .expect("git email");
    std::process::Command::new("git")
        .args(["config", "user.name", "EVA Test"])
        .current_dir(root)
        .status()
        .expect("git name");
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(root)
        .status()
        .expect("git add");
    std::process::Command::new("git")
        .args(["commit", "-qm", "init"])
        .current_dir(root)
        .status()
        .expect("git commit");
}
