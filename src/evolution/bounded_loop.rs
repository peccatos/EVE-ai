use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::evolution::{
    adjust_task_from_campaign, autonomy_status, print_evolution_policy, refresh_evolution_policy,
    refresh_portfolio, refresh_strategy_portfolio, update_policy_feedback, validate_task_contract,
};
use crate::promotion::{replay_candidate, review_candidate};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundedRunSummary {
    pub bounded_run_id: String,
    pub task_id: String,
    pub task_path: String,
    pub started_at: u64,
    pub finished_at: u64,
    pub requested_cycles: usize,
    pub executed_cycles: usize,
    pub stopped_early: bool,
    pub stop_reason: String,
    pub campaign_ids: Vec<String>,
    pub candidate_run_ids: Vec<String>,
    pub replayed_run_ids: Vec<String>,
    pub reviewed_run_ids: Vec<String>,
    pub promotion_ready_run_ids: Vec<String>,
    pub zero_yield_campaign_ids: Vec<String>,
    pub adjusted_task_paths: Vec<String>,
    pub policy_feedback_path: String,
    pub forbidden_mutations: u64,
    pub sandbox_leaks: u64,
    pub auto_promote: bool,
    pub final_status: String,
}

pub fn run_bounded_evolution(
    project_root: &str,
    memory_root: &str,
    task_path: &str,
    requested_cycles: usize,
) -> Result<BoundedRunSummary, String> {
    let task = crate::evolution::load_task_contract(Path::new(task_path))?;
    validate_task_contract(&task)?;
    let autonomy = autonomy_status(project_root, memory_root)?;
    if !autonomy.campaign_mode_allowed {
        return Err("campaign mode is blocked by autonomy gate".to_string());
    }
    let cycles = requested_cycles
        .clamp(1, 10)
        .min(autonomy.max_campaign_cycles);
    let started_at = crate::evolution::memory::now_unix();
    let bounded_run_id = format!("bounded-{}-{}", task.task_id, started_at);
    let mut summary = BoundedRunSummary {
        bounded_run_id: bounded_run_id.clone(),
        task_id: task.task_id.clone(),
        task_path: task_path.to_string(),
        started_at,
        finished_at: started_at,
        requested_cycles,
        executed_cycles: 0,
        stopped_early: false,
        stop_reason: String::new(),
        campaign_ids: Vec::new(),
        candidate_run_ids: Vec::new(),
        replayed_run_ids: Vec::new(),
        reviewed_run_ids: Vec::new(),
        promotion_ready_run_ids: Vec::new(),
        zero_yield_campaign_ids: Vec::new(),
        adjusted_task_paths: Vec::new(),
        policy_feedback_path: Path::new(memory_root)
            .join("policy_feedback.json")
            .display()
            .to_string(),
        forbidden_mutations: 0,
        sandbox_leaks: 0,
        auto_promote: false,
        final_status: "running".to_string(),
    };

    for index in 0..cycles {
        refresh_portfolio(memory_root)?;
        refresh_strategy_portfolio(memory_root)?;
        refresh_evolution_policy(project_root, memory_root, Some(&task))?;

        let mut cycle_task = task.clone();
        cycle_task.cycles = 1;
        cycle_task.created_at = task.created_at.max(index as u64 + 1);
        crate::evolution::store_task_contract(memory_root, &cycle_task)?;
        let campaign =
            crate::evolution::run_stored_campaign(project_root, memory_root, &cycle_task.task_id)?;
        summary.executed_cycles += 1;
        summary.campaign_ids.push(campaign.campaign_id.clone());
        summary.forbidden_mutations += campaign.forbidden_mutations;
        summary.sandbox_leaks += campaign.sandbox_leaks;
        if campaign.useful_candidates == 0 {
            summary
                .zero_yield_campaign_ids
                .push(campaign.campaign_id.clone());
            if let Ok(adjustment) = adjust_task_from_campaign(memory_root, &campaign.campaign_id) {
                if !adjustment.adjusted_task_path.is_empty() {
                    summary
                        .adjusted_task_paths
                        .push(adjustment.adjusted_task_path);
                }
            }
        }
        for run_id in &campaign.candidate_run_ids {
            summary.candidate_run_ids.push(run_id.clone());
            if task.require_replay {
                let _ = replay_candidate(project_root, memory_root, run_id);
                summary.replayed_run_ids.push(run_id.clone());
            }
            let review = review_candidate(project_root, memory_root, run_id)?;
            summary.reviewed_run_ids.push(run_id.clone());
            if review.promotion_allowed {
                summary.promotion_ready_run_ids.push(run_id.clone());
            }
        }
        let _ = update_policy_feedback(memory_root, &campaign);

        if !summary.promotion_ready_run_ids.is_empty() {
            summary.stopped_early = true;
            summary.stop_reason = "promotion_ready_candidate".to_string();
            break;
        }
        if campaign.forbidden_mutations > 0 {
            summary.stopped_early = true;
            summary.stop_reason = "forbidden_mutation_detected".to_string();
            break;
        }
        if campaign.sandbox_leaks > 0 {
            summary.stopped_early = true;
            summary.stop_reason = "sandbox_leak_detected".to_string();
            break;
        }
    }

    summary.finished_at = crate::evolution::memory::now_unix();
    summary.final_status = if !summary.promotion_ready_run_ids.is_empty() {
        "promotion_ready_candidate_found".to_string()
    } else if !summary.zero_yield_campaign_ids.is_empty() && summary.candidate_run_ids.is_empty() {
        "zero_yield_with_adjustments".to_string()
    } else {
        "completed_without_auto_promotion".to_string()
    };
    write_bounded_run(memory_root, &summary)?;
    Ok(summary)
}

pub fn print_last_bounded_run(memory_root: &str) -> Result<String, String> {
    let summary =
        latest_bounded_run(memory_root)?.ok_or_else(|| "no bounded runs available".to_string())?;
    print_bounded_run_report(memory_root, &summary.bounded_run_id)
}

pub fn print_bounded_run_report(memory_root: &str, bounded_run_id: &str) -> Result<String, String> {
    let dir = Path::new(memory_root).join("bounded_runs");
    let path = dir.join(format!("{bounded_run_id}.ru.md"));
    if path.exists() {
        return fs::read_to_string(path)
            .map_err(|error| format!("failed to read bounded run report: {error}"));
    }
    let json_path = dir.join(format!("{bounded_run_id}.json"));
    let contents = fs::read_to_string(&json_path)
        .map_err(|error| format!("failed to read bounded run json: {error}"))?;
    let summary: BoundedRunSummary = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse bounded run json: {error}"))?;
    write_bounded_run(memory_root, &summary)?;
    fs::read_to_string(path)
        .map_err(|error| format!("failed to read rebuilt bounded report: {error}"))
}

pub fn list_bounded_runs(memory_root: &str) -> Result<Vec<String>, String> {
    let dir = Path::new(memory_root).join("bounded_runs");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut ids = fs::read_dir(dir)
        .map_err(|error| format!("failed to read bounded runs dir: {error}"))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .path()
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_string)
        })
        .filter_map(|name| name.strip_suffix(".json").map(str::to_string))
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    Ok(ids)
}

fn write_bounded_run(memory_root: &str, summary: &BoundedRunSummary) -> Result<(), String> {
    let dir = Path::new(memory_root).join("bounded_runs");
    fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create bounded runs dir: {error}"))?;
    crate::evolution::memory::write_json(
        dir.join(format!("{}.json", summary.bounded_run_id)),
        summary,
    )?;
    fs::write(
        dir.join(format!("{}.ru.md", summary.bounded_run_id)),
        render_bounded_markdown(memory_root, summary),
    )
    .map_err(|error| format!("failed to write bounded run markdown: {error}"))
}

fn render_bounded_markdown(memory_root: &str, summary: &BoundedRunSummary) -> String {
    let policy =
        print_evolution_policy(".", memory_root, None).unwrap_or_else(|_| "{}".to_string());
    let next_command = if let Some(path) = summary.adjusted_task_paths.last() {
        format!("cargo run -- --run-task {path}")
    } else if let Some(run_id) = summary.promotion_ready_run_ids.first() {
        format!("cargo run -- --review-candidate {run_id}")
    } else {
        "cargo run -- --recombine-patterns".to_string()
    };
    format!(
        "# Bounded EVA Run\n\n## Task\n{}\n{}\n\n## Policy\n{}\n\n## Campaign cycles\ncampaign_ids={:?}\nrequested_cycles={}\nexecuted_cycles={}\n\n## Recombination fallback\nzero_yield_campaign_ids={:?}\n\n## Candidate recovery\ncandidate_run_ids={:?}\npromotion_ready_run_ids={:?}\n\n## Replay/review\nreplayed_run_ids={:?}\nreviewed_run_ids={:?}\n\n## Feedback\npolicy_feedback_path={}\n\n## Adjusted task drafts\n{:?}\n\n## Safety\nforbidden_mutations={}\nsandbox_leaks={}\nauto_promote={}\n\n## Final decision\nfinal_status={}\nstopped_early={}\nstop_reason={}\n\n## Next manual command\n`{}`\n",
        summary.task_id,
        summary.task_path,
        policy,
        summary.campaign_ids,
        summary.requested_cycles,
        summary.executed_cycles,
        summary.zero_yield_campaign_ids,
        summary.candidate_run_ids,
        summary.promotion_ready_run_ids,
        summary.replayed_run_ids,
        summary.reviewed_run_ids,
        summary.policy_feedback_path,
        summary.adjusted_task_paths,
        summary.forbidden_mutations,
        summary.sandbox_leaks,
        summary.auto_promote,
        summary.final_status,
        summary.stopped_early,
        summary.stop_reason,
        next_command
    )
}

fn latest_bounded_run(memory_root: &str) -> Result<Option<BoundedRunSummary>, String> {
    let dir = Path::new(memory_root).join("bounded_runs");
    if !dir.exists() {
        return Ok(None);
    }
    let mut runs = fs::read_dir(&dir)
        .map_err(|error| format!("failed to read bounded runs dir: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .filter_map(|path| {
            fs::read_to_string(path)
                .ok()
                .and_then(|contents| serde_json::from_str::<BoundedRunSummary>(&contents).ok())
        })
        .collect::<Vec<_>>();
    runs.sort_by(|left, right| {
        right
            .finished_at
            .cmp(&left.finished_at)
            .then_with(|| right.bounded_run_id.cmp(&left.bounded_run_id))
    });
    Ok(runs.into_iter().next())
}
