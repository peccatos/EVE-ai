use std::collections::BTreeMap;

use crate::agent::outcome::list_task_outcomes;
use crate::agent::storage::{memory_path, now_unix, save_json_pretty};
use crate::contracts::{TaskOutcome, TaskOutcomeAnalysis};

pub fn build_task_outcome_analysis(memory_root: &str) -> Result<TaskOutcomeAnalysis, String> {
    let outcomes = list_task_outcomes(memory_root)?;
    let total = outcomes.len() as u64;
    let successful = outcomes.iter().filter(|outcome| outcome.success).count() as u64;
    let failed = total.saturating_sub(successful);
    let approved = outcomes.iter().filter(|outcome| outcome.approved).count() as u64;
    let applied = outcomes.iter().filter(|outcome| outcome.applied).count() as u64;
    let validation_passed = outcomes
        .iter()
        .filter(|outcome| outcome.validation_status == "passed")
        .count() as u64;
    let analysis = TaskOutcomeAnalysis {
        generated_at: now_unix(),
        total_tasks: total,
        successful_tasks: successful,
        failed_tasks: failed,
        validation_pass_rate: ratio(validation_passed, total),
        approval_rate: ratio(approved, total),
        apply_success_rate: ratio(applied, total),
        most_successful_task_types: top_kinds(&outcomes, true),
        most_failed_task_types: top_kinds(&outcomes, false),
        risky_paths: collect_paths(&outcomes, false),
        safe_paths: collect_paths(&outcomes, true),
        common_failure_reasons: common_failures(&outcomes),
        warnings: if total == 0 {
            vec!["no_task_outcomes".to_string()]
        } else {
            Vec::new()
        },
        blockers: Vec::new(),
    };
    save_json_pretty(
        &memory_path(memory_root, &["outcome_analysis", "latest.json"]),
        &analysis,
    )?;
    Ok(analysis)
}

pub fn print_outcome_analyze(memory_root: &str) -> Result<String, String> {
    let analysis = build_task_outcome_analysis(memory_root)?;
    Ok(format!(
        "EVA Outcome Analysis\ntotal_tasks={}\nsuccessful_tasks={}\nfailed_tasks={}\nvalidation_pass_rate={:.2}\napproval_rate={:.2}\napply_success_rate={:.2}\ncommon_failure_reasons={}",
        analysis.total_tasks,
        analysis.successful_tasks,
        analysis.failed_tasks,
        analysis.validation_pass_rate,
        analysis.approval_rate,
        analysis.apply_success_rate,
        if analysis.common_failure_reasons.is_empty() { "none".to_string() } else { analysis.common_failure_reasons.join(",") }
    ))
}

fn ratio(count: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        count as f32 / total as f32
    }
}

fn top_kinds(outcomes: &[TaskOutcome], success: bool) -> Vec<String> {
    let mut counts = BTreeMap::<String, u64>::new();
    for outcome in outcomes.iter().filter(|outcome| outcome.success == success) {
        *counts
            .entry(crate::evolution::task_strategy_memory::classify_goal(
                &outcome.goal,
            ))
            .or_default() += 1;
    }
    let mut items = counts.into_iter().collect::<Vec<_>>();
    items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    items.into_iter().map(|(kind, _)| kind).take(5).collect()
}

fn collect_paths(outcomes: &[TaskOutcome], success: bool) -> Vec<String> {
    let mut paths = outcomes
        .iter()
        .filter(|outcome| outcome.success == success)
        .flat_map(|outcome| outcome.files_changed.clone())
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    paths.into_iter().take(10).collect()
}

fn common_failures(outcomes: &[TaskOutcome]) -> Vec<String> {
    let mut failures = outcomes
        .iter()
        .filter_map(|outcome| outcome.failure_reason.clone())
        .collect::<Vec<_>>();
    failures.sort();
    failures.dedup();
    failures.into_iter().take(10).collect()
}
