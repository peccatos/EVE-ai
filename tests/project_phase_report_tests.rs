use eva_runtime_with_task_validator::{
    BenchmarkAggregateMetrics, ProjectPhaseReport, ProjectPhaseStatus, RuntimeAudit,
};

fn audit_with_benchmark(aggregate: Option<BenchmarkAggregateMetrics>) -> RuntimeAudit {
    RuntimeAudit {
        prediction_error: 0.42,
        learning_bias_applied: true,
        strategy_bonus_used: true,
        mutations_attempted: aggregate
            .as_ref()
            .map(|value| (value.mutation_attempt_rate * value.total_cases as f32).round() as u64)
            .unwrap_or(0),
        files_touched: aggregate
            .as_ref()
            .map(|value| value.avg_files_touched.round() as u64)
            .unwrap_or(0),
        rollback_count: aggregate
            .as_ref()
            .map(|value| (value.rollback_rate * value.total_cases as f32).round() as u64)
            .unwrap_or(0),
        benchmark: aggregate,
    }
}

#[test]
fn report_is_emitted_in_russian() {
    let report = ProjectPhaseReport::from_runtime_audit(&audit_with_benchmark(None));
    assert!(!report.summary_ru.is_empty());
    assert!(report.summary_ru.contains("Система") || report.summary_ru.contains("Ядро"));
    assert!(!report.current_blocker_ru.is_empty());
    assert!(!report.next_required_step_ru.is_empty());
}

#[test]
fn success_rate_zero_marks_repair_unproven() {
    let report = ProjectPhaseReport::from_runtime_audit(&audit_with_benchmark(Some(
        BenchmarkAggregateMetrics {
            total_cases: 4,
            reproducible_cases: 4,
            successful_fixes: 0,
            success_rate: 0.0,
            rollback_rate: 0.5,
            avg_files_touched: 1.0,
            avg_prediction_error_after: Some(0.3),
            github_context_usage_rate: 1.0,
            learning_active_rate: 1.0,
            mutation_attempt_rate: 0.5,
        },
    )));

    assert_eq!(report.project_phase, "benchmark_repair_activation");
    assert_eq!(report.phase_status, ProjectPhaseStatus::Partial);
    assert!(report
        .unproven_capabilities
        .iter()
        .any(|entry| entry.contains("успешный ремонт")));
}

#[test]
fn mutations_attempted_positive_changes_phase_classification() {
    let reproduction = ProjectPhaseReport::from_runtime_audit(&audit_with_benchmark(Some(
        BenchmarkAggregateMetrics {
            total_cases: 3,
            reproducible_cases: 3,
            successful_fixes: 0,
            success_rate: 0.0,
            rollback_rate: 0.0,
            avg_files_touched: 0.0,
            avg_prediction_error_after: None,
            github_context_usage_rate: 1.0,
            learning_active_rate: 1.0,
            mutation_attempt_rate: 0.0,
        },
    )));
    let activated = ProjectPhaseReport::from_runtime_audit(&audit_with_benchmark(Some(
        BenchmarkAggregateMetrics {
            total_cases: 3,
            reproducible_cases: 3,
            successful_fixes: 0,
            success_rate: 0.0,
            rollback_rate: 0.33,
            avg_files_touched: 1.0,
            avg_prediction_error_after: Some(0.2),
            github_context_usage_rate: 1.0,
            learning_active_rate: 1.0,
            mutation_attempt_rate: 0.66,
        },
    )));

    assert_eq!(reproduction.project_phase, "benchmark_reproduction");
    assert_eq!(activated.project_phase, "benchmark_repair_activation");
}
