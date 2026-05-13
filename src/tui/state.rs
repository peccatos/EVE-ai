use std::fs;
use std::path::Path;

use crate::contracts::{
    CandidateState, EvolutionLogEntry, EvolutionStatus, TuiCandidateRow, TuiDashboardState,
    TuiMetricsState, TuiReleaseState, TuiRunRow, TuiState,
};
use crate::evolution::{
    autonomy_status, build_preflight_gate_v3, build_release_candidate_state, build_release_health,
    build_runtime_validation, count_sandbox_leaks, load_metrics, load_or_refresh_promotion_queue,
    memory, print_release_status,
};

pub fn format_unknown(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_string()
}

pub fn load_tui_state(project_root: &str, memory_root: &str) -> TuiState {
    let metrics = load_metrics(memory_root).unwrap_or_default();
    let validation = build_runtime_validation(project_root, memory_root).unwrap_or_default();
    let autonomy = autonomy_status(project_root, memory_root).ok();
    let queue = load_or_refresh_promotion_queue(project_root, memory_root).unwrap_or_default();
    let health = build_release_health(project_root, memory_root).ok();
    let release_candidate = build_release_candidate_state(project_root, memory_root).ok();
    let preflight = build_preflight_gate_v3(project_root, memory_root).ok();
    let release_status =
        print_release_status(memory_root).unwrap_or_else(|_| "missing".to_string());
    let sandbox_leak_count = count_sandbox_leaks(project_root).unwrap_or(0) as usize;
    let runs = load_recent_runs(memory_root, 20);
    let last_replay_status = runs
        .iter()
        .find(|run| run.replay_status != "missing")
        .map(|run| run.replay_status.clone())
        .unwrap_or_else(|| "missing".to_string());

    let dashboard = TuiDashboardState {
        runtime_status: if validation.status.is_empty() {
            "unknown".to_string()
        } else {
            validation.status.clone()
        },
        runtime_validation_status: format_unknown(Some(&validation.status)),
        autonomy_level: autonomy
            .as_ref()
            .map(|value| value.current_level)
            .unwrap_or(0),
        allowed_next_autonomy_level: autonomy
            .as_ref()
            .map(|value| value.allowed_next_level)
            .unwrap_or(0),
        campaign_mode_allowed: autonomy
            .as_ref()
            .is_some_and(|value| value.campaign_mode_allowed),
        latest_run_id: metrics.last_run_id.clone(),
        last_replay_status,
        candidate_count: metrics.candidate_count,
        ready_candidates: queue.summary.ready_candidates,
        blocked_candidates: queue
            .items
            .len()
            .saturating_sub(queue.summary.ready_candidates),
        release_status,
        warnings: validation.warnings.clone(),
        blockers: validation.blockers.clone(),
        sandbox_leak_count,
    };

    let candidates = queue
        .items
        .iter()
        .map(|item| TuiCandidateRow {
            run_id: item.run_id.clone(),
            state: format!("{:?}", item.candidate_state),
            promotion_eligibility: item.promotion_state.clone(),
            replay_status: item.replay_status.clone(),
            block_reason: if item.candidate_state_reason.is_empty() {
                item.reason_ru.clone()
            } else {
                item.candidate_state_reason.clone()
            },
            updated_at: item.updated_at,
        })
        .collect::<Vec<_>>();

    let replay_total = metrics.replay_passed + metrics.replay_failed;
    let tui_metrics = TuiMetricsState {
        total_runs: metrics.total_runs,
        passed_runs: metrics.passed_runs,
        failed_runs: metrics.failed_runs,
        safety_rejected_runs: metrics.safety_rejected_runs,
        duplicate_rejected_runs: metrics.duplicate_rejected_runs,
        replay_passed: metrics.replay_passed,
        replay_failed: metrics.replay_failed,
        candidate_count: metrics.candidate_count,
        promoted_count: metrics.promoted_count,
        average_score: metrics.average_score,
        pass_ratio: metrics.pass_ratio,
        replay_pass_ratio: if replay_total == 0 {
            0.0
        } else {
            metrics.replay_passed as f32 / replay_total as f32
        },
    };

    let release = TuiReleaseState {
        approved_release_candidate_exists: release_candidate
            .as_ref()
            .is_some_and(|value| value.operator_approved),
        release_bundle_exists: release_candidate
            .as_ref()
            .is_some_and(|value| value.release_bundle_exists),
        latest_release_candidate: release_candidate
            .as_ref()
            .and_then(|value| value.approved_release_candidate.clone()),
        operator_approval_state: if release_candidate
            .as_ref()
            .is_some_and(|value| value.operator_approved)
        {
            "approved".to_string()
        } else {
            "missing".to_string()
        },
        preflight_gate_status: preflight
            .as_ref()
            .map(|value| value.status.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        release_health: health
            .as_ref()
            .map(|value| value.health_grade.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        green_gate_readiness: if validation.status == "green" {
            "green".to_string()
        } else {
            validation.status.clone()
        },
        missing_requirements: validation.warnings.clone(),
    };

    TuiState {
        dashboard,
        runs,
        candidates,
        metrics: tui_metrics,
        release,
        logs: recent_log_lines(memory_root, 20),
    }
}

fn load_recent_runs(memory_root: &str, limit: usize) -> Vec<TuiRunRow> {
    let path = Path::new(memory_root).join("evolution.jsonl");
    let Ok(contents) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut rows = contents
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<EvolutionLogEntry>(line).ok())
        .take(limit)
        .map(|entry| {
            let replay_status = load_replay_status(memory_root, &entry.run_id);
            TuiRunRow {
                run_id: entry.run_id,
                status: format!("{:?}", entry.status),
                replay_status,
                cargo_test_ok: Some(entry.cargo_test_ok),
                cargo_run_ok: Some(entry.cargo_run_ok),
                duplicate_rejected: entry.duplicate_rejected,
                candidate: entry.status == EvolutionStatus::Candidate,
                promoted: entry.retained_in_core,
                reason: entry
                    .non_candidate_reason
                    .unwrap_or_else(|| "none".to_string()),
            }
        })
        .collect::<Vec<_>>();
    rows.reverse();
    rows
}

fn load_replay_status(memory_root: &str, run_id: &str) -> String {
    let path = Path::new(memory_root)
        .join("replays")
        .join(format!("{run_id}.json"));
    let Ok(contents) = fs::read_to_string(path) else {
        return "missing".to_string();
    };
    let Ok(replay) = serde_json::from_str::<memory::ReplayResult>(&contents) else {
        return "unknown".to_string();
    };
    if replay.matches_stored_summary
        && replay.replay_status != EvolutionStatus::Failed
        && replay.cargo_check_ok
        && replay.cargo_test_ok
        && replay.cargo_run_ok
    {
        "passed".to_string()
    } else {
        "failed".to_string()
    }
}

fn recent_log_lines(memory_root: &str, limit: usize) -> Vec<String> {
    let path = Path::new(memory_root).join("evolution.jsonl");
    let Ok(contents) = fs::read_to_string(path) else {
        return vec!["missing evolution log".to_string()];
    };
    contents
        .lines()
        .rev()
        .take(limit)
        .map(|line| line.chars().take(180).collect::<String>())
        .collect()
}

#[allow(dead_code)]
fn _candidate_state_label(state: &CandidateState) -> &'static str {
    match state {
        CandidateState::Ready => "ready",
        CandidateState::Blocked => "blocked",
        CandidateState::Quarantined => "quarantined",
        CandidateState::Stale => "stale",
        CandidateState::Legacy => "legacy",
        CandidateState::Duplicate => "duplicate",
        CandidateState::Unreplayable => "unreplayable",
        CandidateState::AlreadyPromoted => "already_promoted",
        CandidateState::Unknown => "unknown",
    }
}
