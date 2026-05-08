use std::fs;
use std::path::{Path, PathBuf};

#[path = "evolution_test_support.rs"]
mod evolution_test_support;

use eva_runtime_with_task_validator::{
    build_artifact_audit, build_capability_policy, build_determinism_audit, build_final_rc_report,
    build_preflight_gate_v3, build_proof_report, build_release_health,
    build_runtime_candidate_manifest, build_runtime_cli_contract, build_runtime_service_metadata,
    build_runtime_validation, build_workspace_snapshot, evaluate_runtime_validation,
    governance_status, print_final_rc_report, print_future_phases, print_operator_console,
    print_proof_json, print_proof_report, run_demo,
};
use serde_json::Value;

#[test]
fn runtime_candidate_is_metadata_only_and_deterministic_enough() {
    let root = temp_runtime_root("phase150-runtime-candidate");
    let first = build_runtime_candidate_manifest(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("first candidate");
    let second = build_runtime_candidate_manifest(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("second candidate");
    let json = serde_json::to_string_pretty(&first).expect("serialize");
    assert_eq!(first.completed_phases, second.completed_phases);
    assert_eq!(first.planned_phases, second.planned_phases);
    assert_eq!(first.support_flags, second.support_flags);
    assert_eq!(first.rc_status, second.rc_status);
    assert!(!json.contains("pub fn"));
    assert!(first.planned_phases.is_empty());
    evolution_test_support::remove_root(&root);
}

#[test]
fn runtime_validation_returns_warn_when_no_approved_release_candidate_exists() {
    let root = temp_runtime_root("phase150-runtime-validation-warn");
    let validation = build_runtime_validation(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("validation");
    assert_eq!(validation.status, "warn");
    assert!(validation
        .warnings
        .contains(&"no_approved_release_candidate".to_string()));
    assert!(!validation.auto_promote);
    assert!(validation.operator_approval_required);
    evolution_test_support::remove_root(&root);
}

#[test]
fn runtime_validation_blocks_sandbox_leaks() {
    let root = temp_runtime_root("phase150-runtime-validation-leak");
    fs::create_dir_all(root.join("sandboxes/leak")).expect("leak");
    let validation = build_runtime_validation(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("validation");
    assert_eq!(validation.status, "blocked");
    assert!(validation
        .blockers
        .contains(&"sandbox_leaks_present".to_string()));
    evolution_test_support::remove_root(&root);
}

#[test]
fn runtime_validation_blocks_unsafe_capability_policy_states_when_simulated_safely() {
    let root = temp_runtime_root("phase150-runtime-validation-policy");
    let governance = governance_status(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("governance");
    let proof = build_proof_report(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("proof");
    let health = build_release_health(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("health");
    let artifact = build_artifact_audit(root.to_str().unwrap()).expect("artifact");
    let determinism = build_determinism_audit(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("determinism");
    let gate_v3 = build_preflight_gate_v3(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("gate");
    let snapshot = build_workspace_snapshot(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("snapshot");
    let mut policy = build_capability_policy();
    policy.merge_allowed = true;
    let validation = evaluate_runtime_validation(
        1,
        &policy,
        &governance,
        &proof,
        &health,
        &artifact,
        &determinism,
        &gate_v3,
        &snapshot,
    );
    assert_eq!(validation.status, "blocked");
    assert!(validation
        .blockers
        .contains(&"unsafe_capability_policy".to_string()));
    evolution_test_support::remove_root(&root);
}

#[test]
fn runtime_service_metadata_does_not_install_or_start_daemon() {
    let root = temp_runtime_root("phase150-runtime-service");
    let service =
        build_runtime_service_metadata(root.join("memory").to_str().unwrap()).expect("service");
    assert_eq!(service.service_name, "eva-runtime");
    assert!(!service.daemonized);
    assert!(service.attach_supported);
    assert_eq!(service.systemd_install, "not_performed");
    assert!(!root.join("sandboxes").exists());
    evolution_test_support::remove_root(&root);
}

#[test]
fn cli_contract_lists_required_commands() {
    let root = temp_runtime_root("phase150-cli-contract");
    let contract =
        build_runtime_cli_contract(root.join("memory").to_str().unwrap()).expect("contract");
    let commands = contract
        .commands
        .iter()
        .map(|entry| entry.command.as_str())
        .collect::<Vec<_>>();
    for required in [
        "--eva-status",
        "--operator-console",
        "--proof-report",
        "--proof-json",
        "--capability-policy",
        "--trust-decision",
        "--workspace-snapshot",
        "--evidence-bundle",
        "--recovery-manifest",
        "--preflight-gate-v3",
        "--trust-proof-report",
        "--release-status",
        "--release-health",
        "--artifact-audit",
        "--determinism-audit",
        "--ops-status",
        "--ops-json",
        "--runtime-candidate",
        "--runtime-validation",
        "--runtime-service",
        "--runtime-cli-contract",
        "--final-rc-report",
    ] {
        assert!(commands.contains(&required), "missing {required}");
    }
    evolution_test_support::remove_root(&root);
}

#[test]
fn final_rc_report_includes_auto_promote_false_and_operator_approval_required() {
    let root = temp_runtime_root("phase150-final-rc");
    let report = print_final_rc_report(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("report");
    let metadata = build_final_rc_report(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("metadata");
    assert!(report.contains("auto_promote=false"));
    assert!(report.contains("operator approval required=true"));
    assert!(Path::new(&metadata.report_path).exists());
    evolution_test_support::remove_root(&root);
}

#[test]
fn proof_report_and_json_include_phase15_flags() {
    let root = temp_runtime_root("phase150-proof");
    let report = print_proof_report(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("report");
    assert!(report.contains("runtime_candidate_support=true"));
    assert!(report.contains("runtime_validation_support=true"));
    assert!(report.contains("runtime_service_metadata_support=true"));
    assert!(report.contains("stable_cli_contract_support=true"));
    assert!(report.contains("final_rc_report_support=true"));
    let json = print_proof_json(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("json");
    let value: Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(value["runtime_candidate_support"].as_bool(), Some(true));
    assert_eq!(value["runtime_validation_support"].as_bool(), Some(true));
    assert_eq!(
        value["runtime_service_metadata_support"].as_bool(),
        Some(true)
    );
    assert_eq!(value["stable_cli_contract_support"].as_bool(), Some(true));
    assert_eq!(value["final_rc_report_support"].as_bool(), Some(true));
    evolution_test_support::remove_root(&root);
}

#[test]
fn operator_console_and_demo_include_phase15_runtime_candidate_status() {
    let root = temp_runtime_root("phase150-console-demo");
    let console = print_operator_console(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("console");
    let demo = run_demo(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("demo");
    assert!(console.contains("runtime_candidate: status="));
    assert!(console.contains("Phase 15.0: EVA Runtime v1.0 Candidate"));
    assert!(demo.contains("runtime_candidate: status="));
    assert!(demo.contains("Phase 15.0: EVA Runtime v1.0 Candidate status=completed_by_phase_15_0x"));
    evolution_test_support::remove_root(&root);
}

#[test]
fn future_phase_registry_marks_phase15_completed_after_implementation() {
    let output = print_future_phases();
    assert!(output.contains(
        "Phase 13.0: Controlled Self-Modification Review Runtime status=completed_by_phase_13_0x"
    ));
    assert!(output
        .contains("Phase 14.0: Trust + Workspace Recovery Gate status=completed_by_phase_14_0x"));
    assert!(
        output.contains("Phase 15.0: EVA Runtime v1.0 Candidate status=completed_by_phase_15_0x")
    );
    assert!(!output.contains("status=planned"));
}

#[test]
fn metadata_commands_do_not_mutate_source_files_and_no_sandbox_leaks_are_created() {
    let root = temp_runtime_root("phase150-safe");
    let before = fs::read_to_string(root.join("src/lib.rs")).expect("before");
    let _ = build_runtime_candidate_manifest(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("candidate");
    let _ = build_runtime_validation(
        root.to_str().unwrap(),
        root.join("memory").to_str().unwrap(),
    )
    .expect("validation");
    let _ = build_runtime_service_metadata(root.join("memory").to_str().unwrap()).expect("service");
    let _ = build_runtime_cli_contract(root.join("memory").to_str().unwrap()).expect("contract");
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
        root.join(".gitignore"),
        "memory/\nsandboxes/\n.eva-evolution-tests/\n.eva-runtime-tests/\n.eva-operations-tests/\ntarget/\n",
    )
    .expect("gitignore");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"phase150_temp\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\ndoctest = false\n",
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
