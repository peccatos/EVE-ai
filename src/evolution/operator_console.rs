use crate::contracts::OperatorConsoleReport;
use crate::evolution::{
    build_artifact_audit, build_determinism_audit, build_future_phase_registry,
    build_operations_report, build_preflight_gate, build_release_health, governance_status, memory,
    print_eva_status, print_operator_runbook, print_release_status,
};

pub fn build_operator_console_report(
    project_root: &str,
    memory_root: &str,
) -> Result<OperatorConsoleReport, String> {
    let governance = governance_status(project_root, memory_root)?;
    let release_status = print_release_status(memory_root)?;
    let health = build_release_health(project_root, memory_root)?;
    let gate = build_preflight_gate(project_root, memory_root)?;
    let artifact = build_artifact_audit(project_root)?;
    let determinism = build_determinism_audit(project_root, memory_root)?;
    let operations = build_operations_report(project_root, memory_root)?;
    let future = build_future_phase_registry();
    Ok(OperatorConsoleReport {
        generated_at: memory::now_unix(),
        status_lines: vec![
            print_eva_status(project_root, memory_root)?,
            format!(
                "governance_status: approved={} rejected={} deferred={} ready_approved={} auto_promote={}",
                governance.approved_count,
                governance.rejected_count,
                governance.deferred_count,
                governance.promotion_ready_approved_count,
                governance.auto_promote
            ),
            format!("release_status: {release_status}"),
            format!(
                "release_health: grade={} score={}",
                health.health_grade, health.health_score
            ),
            format!("preflight_gate: status={}", gate.gate_status),
            format!(
                "artifact_audit: status={} sandbox_leaks={}",
                if artifact.should_fail_release { "fail" } else { "pass" },
                artifact.sandbox_leaks.len()
            ),
            format!(
                "determinism_audit: status={}",
                if determinism.deterministic_enough { "pass" } else { "fail" }
            ),
            format!(
                "operations_status: next={} future_allowed_now={}",
                operations.next_safe_operator_action, operations.future_phases_allowed_now
            ),
            format!(
                "future_phases: {}",
                future
                    .entries
                    .iter()
                    .map(|entry| format!("{}={}", entry.phase, entry.status))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ],
        next_commands: vec![
            operations.next_safe_operator_action,
            "cargo run -- --proof-report".to_string(),
            "cargo run -- --operator-runbook".to_string(),
        ],
    })
}

pub fn print_operator_console(project_root: &str, memory_root: &str) -> Result<String, String> {
    let report = build_operator_console_report(project_root, memory_root)?;
    let runbook = print_operator_runbook(project_root, memory_root)?;
    Ok(format!(
        "# EVA Operator Console\n\n{}\n\n## Next commands\n{}\n\n{}\n",
        report.status_lines.join("\n"),
        report
            .next_commands
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n"),
        runbook
    ))
}
