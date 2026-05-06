use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::contracts::{EvolutionLogEntry, EvolutionStatus};
use crate::evolution::memory::ReplayResult;

pub const DEFAULT_METRICS_PATH: &str = "memory/metrics.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EvolutionMetrics {
    pub total_runs: u64,
    pub passed_runs: u64,
    pub failed_runs: u64,
    pub candidate_count: u64,
    pub replay_passed: u64,
    pub promoted_count: u64,
    pub average_score: f32,
    pub last_run_id: Option<String>,
}

pub fn load_metrics(memory_root: &str) -> Result<EvolutionMetrics, String> {
    let path = Path::new(memory_root).join("metrics.json");
    if !path.exists() {
        return Ok(EvolutionMetrics::default());
    }
    let contents =
        fs::read_to_string(&path).map_err(|error| format!("failed to read metrics: {error}"))?;
    serde_json::from_str(&contents).map_err(|error| format!("failed to parse metrics: {error}"))
}

pub fn update_metrics_after_log(
    memory_root: &str,
    entry: &EvolutionLogEntry,
) -> Result<EvolutionMetrics, String> {
    let mut metrics = load_metrics(memory_root)?;
    let previous_total = metrics.total_runs;
    metrics.total_runs += 1;
    match entry.status {
        EvolutionStatus::Failed => metrics.failed_runs += 1,
        EvolutionStatus::Candidate => {
            metrics.passed_runs += 1;
            metrics.candidate_count += 1;
        }
        EvolutionStatus::Promoted => {
            metrics.passed_runs += 1;
            metrics.promoted_count += 1;
        }
        EvolutionStatus::Passed => metrics.passed_runs += 1,
    }
    metrics.average_score =
        ((metrics.average_score * previous_total as f32) + entry.score) / metrics.total_runs as f32;
    metrics.last_run_id = Some(entry.run_id.clone());
    write_metrics(memory_root, &metrics)?;
    Ok(metrics)
}

pub fn update_metrics_after_replay(
    memory_root: &str,
    replay: &ReplayResult,
) -> Result<EvolutionMetrics, String> {
    let mut metrics = load_metrics(memory_root)?;
    if replay.matches_stored_summary && replay.cargo_check_ok && replay.cargo_test_ok {
        metrics.replay_passed += 1;
    }
    write_metrics(memory_root, &metrics)?;
    Ok(metrics)
}

pub fn write_metrics(memory_root: &str, metrics: &EvolutionMetrics) -> Result<(), String> {
    let path = Path::new(memory_root).join("metrics.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create metrics directory: {error}"))?;
    }
    let contents = serde_json::to_string_pretty(metrics)
        .map_err(|error| format!("failed to serialize metrics: {error}"))?;
    fs::write(path, contents).map_err(|error| format!("failed to write metrics: {error}"))
}
