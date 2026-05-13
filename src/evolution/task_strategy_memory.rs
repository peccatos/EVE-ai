use crate::agent::outcome::list_task_outcomes;
use crate::agent::storage::{memory_path, now_unix, save_json_pretty};
use crate::contracts::{TaskStrategyMemory, TaskStrategyPattern};

pub fn build_task_strategy_memory(memory_root: &str) -> Result<TaskStrategyMemory, String> {
    let outcomes = list_task_outcomes(memory_root)?;
    let kinds = ["docs", "tests", "code", "broad", "unsafe"];
    let mut strategies = Vec::new();
    for kind in kinds {
        let success = outcomes
            .iter()
            .filter(|outcome| classify_goal(&outcome.goal) == kind && outcome.success)
            .count() as u64;
        let failure = outcomes
            .iter()
            .filter(|outcome| classify_goal(&outcome.goal) == kind && !outcome.success)
            .count() as u64;
        if success > 0 || failure > 0 || matches!(kind, "broad" | "unsafe") {
            strategies.push(TaskStrategyPattern {
                task_kind: kind.to_string(),
                recommended_strategy: recommended_strategy(kind, success, failure),
                success_count: success,
                failure_count: failure,
                confidence: confidence(success, failure),
                reason: format!("derived from {success} successes and {failure} failures"),
            });
        }
    }
    let memory = TaskStrategyMemory {
        generated_at: now_unix(),
        strategies,
        warnings: if outcomes.is_empty() {
            vec!["no_task_outcomes".to_string()]
        } else {
            Vec::new()
        },
        blockers: Vec::new(),
    };
    save_json_pretty(
        &memory_path(
            memory_root,
            &["strategy_memory", "task_strategy_memory.json"],
        ),
        &memory,
    )?;
    Ok(memory)
}

pub fn print_strategy_memory(memory_root: &str) -> Result<String, String> {
    let memory = build_task_strategy_memory(memory_root)?;
    Ok(format!(
        "EVA Task Strategy Memory\nstrategies={}\nwarnings={}",
        memory.strategies.len(),
        if memory.warnings.is_empty() {
            "none".to_string()
        } else {
            memory.warnings.join(",")
        }
    ))
}

pub fn classify_goal(goal: &str) -> String {
    let lower = goal.to_ascii_lowercase();
    if lower.contains(".git")
        || lower.contains("memory/")
        || lower.contains("/etc")
        || lower.contains("sudo")
    {
        "unsafe".to_string()
    } else if lower.contains("doc")
        || lower.contains("readme")
        || lower.contains("док")
        || lower.contains("опис")
    {
        "docs".to_string()
    } else if lower.contains("test") || lower.contains("провер") {
        "tests".to_string()
    } else if lower.contains("refactor")
        || lower.contains("implement")
        || lower.contains("code")
        || lower.contains("src/")
    {
        "code".to_string()
    } else {
        "broad".to_string()
    }
}

pub fn recommended_strategy(kind: &str, success: u64, failure: u64) -> String {
    match kind {
        "docs" if success >= failure => "docs_only".to_string(),
        "tests" if success >= failure => "tests_first".to_string(),
        "code" if success > failure => "small_code_patch".to_string(),
        "unsafe" => "refuse_unsafe".to_string(),
        "broad" => "manual_decomposition_required".to_string(),
        _ => "manual_decomposition_required".to_string(),
    }
}

fn confidence(success: u64, failure: u64) -> f32 {
    let total = success + failure;
    if total == 0 {
        0.0
    } else {
        success.max(failure) as f32 / total as f32
    }
}
