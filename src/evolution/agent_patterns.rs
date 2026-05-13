use std::fs;

use crate::agent::outcome::list_task_outcomes;
use crate::agent::storage::{memory_path, now_unix, save_json_pretty};
use crate::contracts::{AgentPattern, AgentPatternSummary};

pub fn build_agent_patterns(memory_root: &str) -> Result<AgentPatternSummary, String> {
    let outcomes = list_task_outcomes(memory_root)?;
    let mut patterns = Vec::new();
    for (task_kind, outcome_label, pattern_id) in [
        ("docs", "success", "successful_docs_task"),
        ("tests", "success", "successful_tests_task"),
        ("code", "success", "successful_small_code_edit"),
        ("broad", "failure", "failed_broad_task"),
        ("unsafe", "failure", "failed_forbidden_path"),
    ] {
        let evidence = outcomes
            .iter()
            .filter(|outcome| {
                crate::evolution::task_strategy_memory::classify_goal(&outcome.goal) == task_kind
            })
            .filter(|outcome| {
                (outcome.success && outcome_label == "success")
                    || (!outcome.success && outcome_label == "failure")
            })
            .map(|outcome| outcome.task_id.clone())
            .collect::<Vec<_>>();
        if !evidence.is_empty() {
            patterns.push(AgentPattern {
                pattern_id: pattern_id.to_string(),
                task_kind: task_kind.to_string(),
                outcome: outcome_label.to_string(),
                count: evidence.len() as u64,
                evidence_task_ids: evidence,
                reason: format!("{pattern_id} extracted from real task outcomes"),
            });
        }
    }
    let summary = AgentPatternSummary {
        generated_at: now_unix(),
        patterns,
        warnings: if outcomes.is_empty() {
            vec!["no_task_outcomes".to_string()]
        } else {
            Vec::new()
        },
        blockers: Vec::new(),
    };
    save_json_pretty(
        &memory_path(memory_root, &["patterns", "agent_patterns.json"]),
        &summary,
    )?;
    fs::write(
        memory_path(memory_root, &["patterns", "agent_patterns.md"]),
        render_patterns(&summary),
    )
    .map_err(|error| format!("write patterns report: {error}"))?;
    Ok(summary)
}

pub fn print_patterns(memory_root: &str) -> Result<String, String> {
    let summary = build_agent_patterns(memory_root)?;
    Ok(format!(
        "EVA Agent Patterns\npatterns={}\nwarnings={}",
        summary.patterns.len(),
        if summary.warnings.is_empty() {
            "none".to_string()
        } else {
            summary.warnings.join(",")
        }
    ))
}

fn render_patterns(summary: &AgentPatternSummary) -> String {
    let mut output = String::from("# EVA Agent Patterns\n\n");
    for pattern in &summary.patterns {
        output.push_str(&format!(
            "- {} kind={} outcome={} count={}\n",
            pattern.pattern_id, pattern.task_kind, pattern.outcome, pattern.count
        ));
    }
    output
}
