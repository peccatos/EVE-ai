use std::fs;
use std::process::Command;

#[path = "evolution_test_support.rs"]
mod evolution_test_support;

use eva_runtime_with_task_validator::{
    print_repair_bench_gate, print_repair_bench_history, run_repair_bench, run_repair_bench_gate,
    run_repair_bench_history, RepairBenchBaseline, RepairBenchGateRequest, RepairBenchGateStatus,
    RepairBenchRequest,
};

#[test]
fn repair_bench_history_handles_empty_history() {
    let root = evolution_test_support::unique_evolution_root("repair-bench-history-empty");
    let output_dir = root.join("bench-output");
    let report = run_repair_bench_history(output_dir.clone()).expect("history report");
    assert_eq!(report.runs, 0);
    assert!(report.latest.is_none());
    let rendered = print_repair_bench_history(output_dir, false, None).expect("history text");
    assert!(rendered.contains("Runs: 0"));
    assert!(rendered.contains("Status: empty"));
    evolution_test_support::remove_root(&root);
}

#[test]
fn repair_bench_history_records_run() {
    let root = evolution_test_support::unique_evolution_root("repair-bench-history-run");
    let output_dir = root.join("bench-output");
    let _ = run_repair_bench(RepairBenchRequest {
        bench_id: "repair-bench-history-run".to_string(),
        suite: "phase21".to_string(),
        output_dir: output_dir.clone(),
        no_llm: true,
        json: false,
    })
    .expect("bench report");
    let report = run_repair_bench_history(output_dir.clone()).expect("history report");
    assert_eq!(report.runs, 1);
    let latest = report.latest.expect("latest");
    assert_eq!(latest.suite, "phase21");
    assert_eq!(latest.passed_cases, 4);
    assert_eq!(latest.partial_cases, 1);
    assert_eq!(latest.failed_cases, 0);
    assert!(output_dir.join("history.jsonl").exists());
    assert!(output_dir.join("latest.json").exists());
    evolution_test_support::remove_root(&root);
}

#[test]
fn repair_bench_gate_uses_latest_history_for_same_suite_only() {
    let root = evolution_test_support::unique_evolution_root("repair-bench-gate-pass");
    let output_dir = root.join("bench-output");
    let _ = run_repair_bench(eva_runtime_with_task_validator::RepairBenchRequest {
        bench_id: "repair-bench-gate-phase24x".to_string(),
        suite: "phase24x".to_string(),
        output_dir: output_dir.clone(),
        no_llm: true,
        json: false,
    })
    .expect("phase24x bench");
    let _ = run_repair_bench(eva_runtime_with_task_validator::RepairBenchRequest {
        bench_id: "repair-bench-gate-phase21".to_string(),
        suite: "phase21".to_string(),
        output_dir: output_dir.clone(),
        no_llm: true,
        json: false,
    })
    .expect("phase21 bench");
    let report = run_repair_bench_gate(RepairBenchGateRequest {
        suite: "phase21".to_string(),
        baseline: "latest".to_string(),
        baseline_file: None,
        output_dir: output_dir.clone(),
        json: false,
    })
    .expect("gate report");
    assert!(matches!(report.status, RepairBenchGateStatus::Passed));
    assert_eq!(report.baseline.suite, "phase21");
    assert_eq!(report.baseline.passed_cases, 4);
    assert_eq!(report.baseline.total_cases, 5);
    assert!(report.regressions.is_empty());
    assert_eq!(report.current_report.passed_cases, 4);
    assert!(report.output_dir.join("report.json").exists());
    assert!(report.output_dir.join("report.md").exists());
    evolution_test_support::remove_root(&root);
}

#[test]
fn repair_bench_gate_ignores_latest_history_from_different_suite() {
    let root = evolution_test_support::unique_evolution_root("repair-bench-gate-ignore-suite");
    let output_dir = root.join("bench-output");
    let _ = run_repair_bench(eva_runtime_with_task_validator::RepairBenchRequest {
        bench_id: "repair-bench-gate-phase24x-only".to_string(),
        suite: "phase24x".to_string(),
        output_dir: output_dir.clone(),
        no_llm: true,
        json: false,
    })
    .expect("phase24x bench");
    let report = run_repair_bench_gate(RepairBenchGateRequest {
        suite: "phase21".to_string(),
        baseline: "latest".to_string(),
        baseline_file: None,
        output_dir: output_dir.clone(),
        json: false,
    })
    .expect("gate report");
    assert!(matches!(report.status, RepairBenchGateStatus::Passed));
    assert_eq!(report.baseline.suite, "phase21");
    assert_eq!(report.baseline.passed_cases, 4);
    assert_eq!(report.baseline.total_cases, 5);
    evolution_test_support::remove_root(&root);
}

#[test]
fn repair_bench_gate_uses_builtin_phase21_baseline_when_no_phase21_history_exists() {
    let root = evolution_test_support::unique_evolution_root("repair-bench-gate-phase21-default");
    let output_dir = root.join("bench-output");
    let _ = run_repair_bench(eva_runtime_with_task_validator::RepairBenchRequest {
        bench_id: "repair-bench-gate-phase24x-history".to_string(),
        suite: "phase24x".to_string(),
        output_dir: output_dir.clone(),
        no_llm: true,
        json: false,
    })
    .expect("phase24x bench");
    let report = run_repair_bench_gate(RepairBenchGateRequest {
        suite: "phase21".to_string(),
        baseline: "latest".to_string(),
        baseline_file: None,
        output_dir: output_dir.clone(),
        json: false,
    })
    .expect("gate report");
    assert!(matches!(report.status, RepairBenchGateStatus::Passed));
    assert_eq!(report.baseline.suite, "phase21");
    assert_eq!(report.baseline.total_cases, 5);
    assert_eq!(report.baseline.passed_cases, 4);
    evolution_test_support::remove_root(&root);
}

#[test]
fn repair_bench_gate_uses_builtin_phase24x_baseline_when_no_phase24x_history_exists() {
    let root = evolution_test_support::unique_evolution_root("repair-bench-gate-phase24x-default");
    let output_dir = root.join("bench-output");
    let _ = run_repair_bench(eva_runtime_with_task_validator::RepairBenchRequest {
        bench_id: "repair-bench-gate-phase21-history".to_string(),
        suite: "phase21".to_string(),
        output_dir: output_dir.clone(),
        no_llm: true,
        json: false,
    })
    .expect("phase21 bench");
    let report = run_repair_bench_gate(RepairBenchGateRequest {
        suite: "phase24x".to_string(),
        baseline: "latest".to_string(),
        baseline_file: None,
        output_dir: output_dir.clone(),
        json: false,
    })
    .expect("gate report");
    assert!(matches!(report.status, RepairBenchGateStatus::Passed));
    assert_eq!(report.baseline.suite, "phase24x");
    assert_eq!(report.baseline.total_cases, 8);
    assert_eq!(report.baseline.actionable_cases, 7);
    assert_eq!(report.baseline.passed_cases, 7);
    assert_eq!(report.baseline.partial_cases, 1);
    assert_eq!(report.baseline.failed_cases, 0);
    evolution_test_support::remove_root(&root);
}

#[test]
fn repair_bench_gate_returns_failed_status_on_regression() {
    let root = evolution_test_support::unique_evolution_root("repair-bench-gate-regression");
    let output_dir = root.join("bench-output");
    let baseline = RepairBenchBaseline {
        suite: "phase24x".to_string(),
        total_cases: 8,
        actionable_cases: 7,
        passed_cases: 7,
        partial_cases: 1,
        failed_cases: 0,
        detection_success_rate: 1.0,
        repair_success_rate: 1.0,
        validation_success_rate: 1.0,
        evidence_success_rate: 1.0,
    };
    let baseline_file = output_dir.join("baseline.json");
    fs::create_dir_all(&output_dir).expect("create output dir");
    fs::write(
        &baseline_file,
        serde_json::to_string_pretty(&baseline).expect("serialize baseline"),
    )
    .expect("write baseline file");
    let report = run_repair_bench_gate(RepairBenchGateRequest {
        suite: "phase21".to_string(),
        baseline: "latest".to_string(),
        baseline_file: Some(baseline_file),
        output_dir: output_dir.clone(),
        json: false,
    })
    .expect("gate report");
    assert!(matches!(report.status, RepairBenchGateStatus::Failed));
    assert!(!report.regressions.is_empty());
    assert!(report
        .regressions
        .iter()
        .any(|regression| regression.field == "passed_cases decreased"));
    evolution_test_support::remove_root(&root);
}

#[test]
fn repair_bench_gate_cli_exits_nonzero_on_regression() {
    let root = evolution_test_support::unique_evolution_root("repair-bench-gate-cli-regression");
    let output_dir = root.join("bench-output");
    let baseline = RepairBenchBaseline {
        suite: "phase24x".to_string(),
        total_cases: 8,
        actionable_cases: 7,
        passed_cases: 7,
        partial_cases: 1,
        failed_cases: 0,
        detection_success_rate: 1.0,
        repair_success_rate: 1.0,
        validation_success_rate: 1.0,
        evidence_success_rate: 1.0,
    };
    let baseline_file = output_dir.join("baseline.json");
    fs::create_dir_all(&output_dir).expect("create output dir");
    fs::write(
        &baseline_file,
        serde_json::to_string_pretty(&baseline).expect("serialize baseline"),
    )
    .expect("write baseline file");
    let status = Command::new(env!("CARGO_BIN_EXE_eva_runtime_with_task_validator"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "repair-bench-gate",
            "--suite",
            "phase21",
            "--baseline-file",
            baseline_file.to_str().expect("baseline path"),
            "--output",
            output_dir.to_str().expect("output path"),
        ])
        .status()
        .expect("run repair-bench-gate cli");
    assert!(!status.success());
    evolution_test_support::remove_root(&root);
}

#[test]
fn repair_bench_gate_json_output_is_parseable() {
    let root = evolution_test_support::unique_evolution_root("repair-bench-gate-json");
    let output_dir = root.join("bench-output");
    let output = print_repair_bench_gate(RepairBenchGateRequest {
        suite: "phase21".to_string(),
        baseline: "latest".to_string(),
        baseline_file: None,
        output_dir,
        json: true,
    })
    .expect("gate text");
    let report: eva_runtime_with_task_validator::RepairBenchGateReport =
        serde_json::from_str(&output).expect("parse gate json");
    assert_eq!(report.suite, "phase21");
    assert!(matches!(report.status, RepairBenchGateStatus::Passed));
    evolution_test_support::remove_root(&root);
}

#[test]
fn repair_bench_gate_ignores_unknown_empty_project_partial() {
    let root = evolution_test_support::unique_evolution_root("repair-bench-gate-partial");
    let output_dir = root.join("bench-output");
    let report = run_repair_bench_gate(RepairBenchGateRequest {
        suite: "phase21".to_string(),
        baseline: "latest".to_string(),
        baseline_file: None,
        output_dir,
        json: false,
    })
    .expect("gate report");
    assert!(matches!(report.status, RepairBenchGateStatus::Passed));
    assert_eq!(report.current_report.partial_cases, 1);
    assert_eq!(report.current_report.failed_cases, 0);
    evolution_test_support::remove_root(&root);
}

#[test]
fn repair_bench_gate_does_not_mutate_source_tree() {
    let root = evolution_test_support::unique_evolution_root("repair-bench-gate-clean");
    let output_dir = root.join("bench-output");
    let before = git_status_short();
    let _ = run_repair_bench_gate(RepairBenchGateRequest {
        suite: "phase21".to_string(),
        baseline: "latest".to_string(),
        baseline_file: None,
        output_dir,
        json: false,
    })
    .expect("gate report");
    let after = git_status_short();
    assert_eq!(before, after);
    evolution_test_support::remove_root(&root);
}

fn git_status_short() -> Vec<String> {
    let output = Command::new("git")
        .args(["status", "--short"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("git status");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}
