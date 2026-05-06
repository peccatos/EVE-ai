use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::contracts::{sha256_digest, CommandResult};
use crate::evolution::{
    ensure_portfolio, ensure_strategy_portfolio, mutation_class_label,
    templates::normalized_generated_test_name_for_seed, MutationClass,
};

const GENERATED_TEST_PATH: &str = "tests/evolution_generated_tests.rs";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HygieneReport {
    pub cosmetic_mutation_count: u64,
    pub unsafe_mutation_count: u64,
    pub legacy_mutation_count: u64,
    #[serde(default)]
    pub long_generated_test_names: Vec<String>,
    #[serde(default)]
    pub duplicated_generated_test_names: Vec<String>,
    #[serde(default)]
    pub portfolio_contamination_summary: Vec<String>,
    #[serde(default)]
    pub recommended_safe_cleanup_actions: Vec<String>,
}

pub fn run_evolution_hygiene(
    project_root: &str,
    memory_root: &str,
) -> Result<HygieneReport, String> {
    let portfolio = ensure_portfolio(memory_root)?;
    let strategy_portfolio = ensure_strategy_portfolio(memory_root)?;
    let test_file = Path::new(project_root).join(GENERATED_TEST_PATH);
    let test_names = load_generated_test_names(&test_file)?;
    let long_generated_test_names = test_names
        .iter()
        .filter(|name| name.len() > 80)
        .cloned()
        .collect::<Vec<_>>();
    let duplicated_generated_test_names = duplicate_names(&test_names);

    let cosmetic_mutation_count = portfolio
        .kinds
        .iter()
        .map(|entry| entry.cosmetic_count)
        .sum();
    let unsafe_mutation_count = portfolio.kinds.iter().map(|entry| entry.unsafe_count).sum();
    let legacy_mutation_count = strategy_portfolio
        .strategies
        .iter()
        .map(|entry| entry.legacy_count)
        .sum();

    let mut portfolio_contamination_summary = portfolio
        .kinds
        .iter()
        .filter_map(|entry| {
            if entry.cosmetic_count > 0
                || entry.unsafe_count > 0
                || entry.mutation_class == MutationClass::Legacy
            {
                Some(format!(
                    "{} class={} cosmetic={} unsafe={}",
                    entry.mutation_kind,
                    mutation_class_label(entry.mutation_class),
                    entry.cosmetic_count,
                    entry.unsafe_count
                ))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    portfolio_contamination_summary.extend(
        strategy_portfolio
            .strategies
            .iter()
            .filter(|entry| {
                entry.cosmetic_count > 0 || entry.unsafe_count > 0 || entry.legacy_count > 0
            })
            .map(|entry| {
                format!(
                    "strategy:{} cosmetic={} unsafe={} legacy={}",
                    entry.strategy, entry.cosmetic_count, entry.unsafe_count, entry.legacy_count
                )
            }),
    );
    portfolio_contamination_summary.sort();

    let mut recommended_safe_cleanup_actions = Vec::new();
    if cosmetic_mutation_count > 0 {
        recommended_safe_cleanup_actions.push(
            "Ignore cosmetic AppendComment history during policy and recombination.".to_string(),
        );
    }
    if unsafe_mutation_count > 0 {
        recommended_safe_cleanup_actions.push(
            "Keep unsafe historical kinds visible in hygiene only; never select them.".to_string(),
        );
    }
    if !long_generated_test_names.is_empty() || !duplicated_generated_test_names.is_empty() {
        recommended_safe_cleanup_actions.push(
            "Run --hygiene-fix-generated-tests to normalize only long eva_generated_* test names."
                .to_string(),
        );
    }
    if recommended_safe_cleanup_actions.is_empty() {
        recommended_safe_cleanup_actions.push("No cleanup required.".to_string());
    }

    let report = HygieneReport {
        cosmetic_mutation_count,
        unsafe_mutation_count,
        legacy_mutation_count,
        long_generated_test_names,
        duplicated_generated_test_names,
        portfolio_contamination_summary,
        recommended_safe_cleanup_actions,
    };
    persist_hygiene_report(memory_root, &report)?;
    Ok(report)
}

pub fn print_hygiene_report(project_root: &str, memory_root: &str) -> Result<String, String> {
    let report = run_evolution_hygiene(project_root, memory_root)?;
    Ok(render_hygiene_markdown(&report))
}

pub fn print_hygiene_plan(project_root: &str, memory_root: &str) -> Result<String, String> {
    let report = run_evolution_hygiene(project_root, memory_root)?;
    Ok(report.recommended_safe_cleanup_actions.join("\n"))
}

pub fn fix_generated_test_names(project_root: &str) -> Result<String, String> {
    let path = Path::new(project_root).join(GENERATED_TEST_PATH);
    if !path.exists() {
        return Ok("generated test file not found".to_string());
    }
    let original = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read generated tests: {error}"))?;
    let mut updated = original.clone();
    let mut changed = false;
    for name in load_generated_test_names(&path)? {
        if !name.starts_with("eva_generated_") || name.len() <= 80 {
            continue;
        }
        let new_name = normalized_generated_test_name_for_seed(
            &format!("cleanup:{}:{}", name, sha256_digest(&name)),
            "deterministic",
        );
        if new_name == name {
            continue;
        }
        updated = updated.replace(&format!("fn {name}"), &format!("fn {new_name}"));
        changed = true;
    }
    if !changed {
        return Ok("no long generated test names found".to_string());
    }

    fs::write(&path, &updated)
        .map_err(|error| format!("failed to write generated tests: {error}"))?;
    let fmt = run_host_command(project_root, "cargo", &["fmt"])?;
    let check = run_host_command(project_root, "cargo", &["check"])?;
    let test = run_host_command(project_root, "cargo", &["test"])?;
    if !fmt.success || !check.success || !test.success {
        fs::write(&path, original)
            .map_err(|error| format!("validation failed and rollback write failed: {error}"))?;
        return Err(format!(
            "generated test hygiene validation failed: fmt={} check={} test={}",
            fmt.success, check.success, test.success
        ));
    }
    Ok("generated test names normalized".to_string())
}

fn persist_hygiene_report(memory_root: &str, report: &HygieneReport) -> Result<(), String> {
    let dir = Path::new(memory_root).join("hygiene");
    fs::create_dir_all(&dir).map_err(|error| format!("failed to create hygiene dir: {error}"))?;
    crate::evolution::memory::write_json(dir.join("latest_hygiene.json"), report)?;
    fs::write(
        dir.join("latest_hygiene.ru.md"),
        render_hygiene_markdown(report),
    )
    .map_err(|error| format!("failed to write hygiene markdown: {error}"))
}

fn render_hygiene_markdown(report: &HygieneReport) -> String {
    format!(
        "# Hygiene EVA\n\ncosmetic mutation count: {}\nunsafe mutation count: {}\nlegacy mutation count: {}\nlong generated test names: {}\nduplicated generated test names: {}\nportfolio contamination summary: {}\nrecommended safe cleanup actions: {}\n",
        report.cosmetic_mutation_count,
        report.unsafe_mutation_count,
        report.legacy_mutation_count,
        if report.long_generated_test_names.is_empty() {
            "(none)".to_string()
        } else {
            report.long_generated_test_names.join(", ")
        },
        if report.duplicated_generated_test_names.is_empty() {
            "(none)".to_string()
        } else {
            report.duplicated_generated_test_names.join(", ")
        },
        if report.portfolio_contamination_summary.is_empty() {
            "(none)".to_string()
        } else {
            report.portfolio_contamination_summary.join("; ")
        },
        report.recommended_safe_cleanup_actions.join("; ")
    )
}

fn load_generated_test_names(path: &Path) -> Result<Vec<String>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read generated test file: {error}"))?;
    Ok(contents.lines().filter_map(extract_test_fn_name).collect())
}

fn duplicate_names(names: &[String]) -> Vec<String> {
    let mut counts = BTreeMap::new();
    for name in names {
        *counts.entry(name.clone()).or_insert(0_u64) += 1;
    }
    counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(name, _)| name)
        .collect()
}

fn extract_test_fn_name(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let remainder = trimmed.strip_prefix("fn ")?;
    let name = remainder
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>();
    if name.starts_with("eva_generated_") {
        Some(name)
    } else {
        None
    }
}

fn run_host_command(project_root: &str, bin: &str, args: &[&str]) -> Result<CommandResult, String> {
    let start = std::time::Instant::now();
    let output = Command::new(bin)
        .args(args)
        .current_dir(project_root)
        .output()
        .map_err(|error| format!("failed to run {bin}: {error}"))?;
    Ok(CommandResult {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        duration_ms: start.elapsed().as_millis(),
    })
}
