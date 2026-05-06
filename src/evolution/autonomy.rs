use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::evolution::benchmark::count_sandbox_leaks;
use crate::evolution::load_metrics;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutonomyStatus {
    pub current_level: u8,
    pub allowed_next_level: u8,
    pub blockers: Vec<String>,
    pub required_metrics: Vec<String>,
}

pub fn autonomy_status(project_root: &str, memory_root: &str) -> Result<AutonomyStatus, String> {
    let metrics = load_metrics(memory_root)?;
    let sandbox_leaks = count_sandbox_leaks(project_root)?;
    let forbidden_mutations = count_forbidden_mutations(memory_root)?;
    let replay_passed = metrics.replay_passed;
    let recent_gates_pass = recent_gates_pass(memory_root)?;
    let useful_replay_candidates = replay_passed;
    let regression_rate = regression_rate(memory_root, metrics.total_runs)?;

    let mut blockers = Vec::new();
    if metrics.total_runs < 10 {
        blockers.push("нужно не менее 10 запусков".to_string());
    }
    if sandbox_leaks > 0 {
        blockers.push("обнаружены утечки sandbox".to_string());
    }
    if forbidden_mutations > 0 {
        blockers.push("обнаружены forbidden mutations".to_string());
    }
    if !recent_gates_pass {
        blockers.push("последние sandbox cargo-gates нестабильны".to_string());
    }

    let level2_ready = metrics.total_runs >= 10
        && sandbox_leaks == 0
        && forbidden_mutations == 0
        && recent_gates_pass;

    let mut current_level = if metrics.total_runs == 0 { 0 } else { 1 };
    if level2_ready {
        current_level = 2;
    }
    let level3_ready = level2_ready
        && useful_replay_candidates >= 1
        && replay_passed >= 3
        && regression_rate < 0.5;
    if level3_ready {
        current_level = 3;
    } else if current_level < 3 {
        if useful_replay_candidates < 1 {
            blockers.push("нет replay-подтверждённого полезного кандидата".to_string());
        }
        if replay_passed < 3 {
            blockers.push("нужно не менее 3 replay_passed".to_string());
        }
        if regression_rate >= 0.5 && metrics.total_runs > 0 {
            blockers.push("слишком высокий regression rate".to_string());
        }
    }

    Ok(AutonomyStatus {
        current_level,
        allowed_next_level: if current_level >= 3 {
            3
        } else {
            current_level + 1
        },
        blockers,
        required_metrics: vec![
            format!("total_runs={}", metrics.total_runs),
            format!("replay_passed={replay_passed}"),
            format!("sandbox_leaks={sandbox_leaks}"),
            format!("forbidden_mutations={forbidden_mutations}"),
            format!("regression_rate={regression_rate:.2}"),
        ],
    })
}

fn count_forbidden_mutations(memory_root: &str) -> Result<u64, String> {
    let path = Path::new(memory_root).join("evolution.jsonl");
    if !path.exists() {
        return Ok(0);
    }
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read evolution log: {error}"))?;
    Ok(contents
        .lines()
        .filter_map(|line| serde_json::from_str::<crate::EvolutionLogEntry>(line).ok())
        .filter(|entry| {
            entry.target_file.starts_with("src/core/")
                || entry.target_file == "src/main.rs"
                || entry.target_file == "src/lib.rs"
                || entry.target_file == "Cargo.toml"
        })
        .count() as u64)
}

fn recent_gates_pass(memory_root: &str) -> Result<bool, String> {
    let path = Path::new(memory_root).join("evolution.jsonl");
    if !path.exists() {
        return Ok(false);
    }
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read evolution log: {error}"))?;
    let recent = contents
        .lines()
        .filter_map(|line| serde_json::from_str::<crate::EvolutionLogEntry>(line).ok())
        .rev()
        .take(5)
        .collect::<Vec<_>>();
    if recent.is_empty() {
        return Ok(false);
    }
    Ok(recent
        .iter()
        .all(|entry| entry.cargo_check_ok && entry.cargo_test_ok && entry.cargo_run_ok))
}

fn regression_rate(memory_root: &str, total_runs: u64) -> Result<f32, String> {
    if total_runs == 0 {
        return Ok(0.0);
    }
    let entries = crate::evolution::load_regressions(memory_root)?;
    Ok(entries.len() as f32 / total_runs as f32)
}
