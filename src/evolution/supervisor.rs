use std::fs;
use std::path::Path;

use crate::contracts::SupervisedRun;
use crate::evolution::{
    load_task_contract, memory, print_bounded_run_report, refresh_promotion_queue,
    run_bounded_evolution, validate_task_contract, BoundedRunSummary, EvolutionCampaign,
};
use crate::promotion::review::review_candidate;

pub fn supervise_task(
    project_root: &str,
    memory_root: &str,
    task_path: &str,
    max_rounds: usize,
) -> Result<SupervisedRun, String> {
    let task = load_task_contract(Path::new(task_path))?;
    validate_task_contract(&task)?;
    let started_at = memory::now_unix();
    let mut run = SupervisedRun {
        supervised_run_id: format!("supervised-{}-{}", task.task_id, started_at),
        initial_task_path: task_path.to_string(),
        current_task_path: task_path.to_string(),
        started_at,
        finished_at: started_at,
        max_rounds: max_rounds.clamp(1, 10),
        executed_rounds: 0,
        bounded_run_ids: Vec::new(),
        campaign_ids: Vec::new(),
        adjusted_task_paths: Vec::new(),
        ready_candidate_run_ids: Vec::new(),
        rejected_candidate_run_ids: Vec::new(),
        zero_yield_rounds: 0,
        replay_failed_rounds: 0,
        stop_reason: String::new(),
        final_status: "running".to_string(),
        auto_promote: false,
    };

    let mut current_task_path = task_path.to_string();
    for _ in 0..run.max_rounds {
        let round_task = load_task_contract(Path::new(&current_task_path))?;
        validate_task_contract(&round_task)?;
        let bounded = run_bounded_evolution(
            project_root,
            memory_root,
            &current_task_path,
            round_task.cycles,
        )?;
        run.executed_rounds += 1;
        run.current_task_path = current_task_path.clone();
        run.bounded_run_ids.push(bounded.bounded_run_id.clone());
        run.campaign_ids.extend(bounded.campaign_ids.clone());
        run.adjusted_task_paths
            .extend(bounded.adjusted_task_paths.clone());

        if bounded.sandbox_leaks > 0 || bounded.forbidden_mutations > 0 {
            run.stop_reason = if bounded.sandbox_leaks > 0 {
                "sandbox_leak_detected".to_string()
            } else {
                "forbidden_mutation_detected".to_string()
            };
            run.final_status = "safety_stop".to_string();
            break;
        }

        collect_candidate_outcomes(project_root, memory_root, &bounded, &mut run)?;
        let replay_failed = campaigns_for_bounded(memory_root, &bounded)?
            .iter()
            .any(|campaign| campaign.candidate_rejected_failed_replay > 0);
        if replay_failed {
            run.replay_failed_rounds += 1;
        }
        let zero_yield =
            !bounded.zero_yield_campaign_ids.is_empty() && bounded.candidate_run_ids.is_empty();
        if zero_yield {
            run.zero_yield_rounds += 1;
        }

        if !run.ready_candidate_run_ids.is_empty() {
            run.stop_reason = "promotion_ready_candidate".to_string();
            run.final_status = "promotion_ready_candidate_found".to_string();
            break;
        }

        if zero_yield {
            if run.zero_yield_rounds >= 2 {
                run.stop_reason = "repeated_zero_yield".to_string();
                run.final_status = "zero_yield_exhausted".to_string();
                break;
            }
            if let Some(path) = bounded.adjusted_task_paths.last() {
                current_task_path = path.clone();
                continue;
            }
            run.stop_reason = "zero_yield_without_adjustment".to_string();
            run.final_status = "no_progress".to_string();
            break;
        }

        if replay_failed {
            if run.replay_failed_rounds >= 2 {
                run.stop_reason = "replay_failed_repeated".to_string();
                run.final_status = "replay_failed_exhausted".to_string();
                break;
            }
            continue;
        }

        if bounded.adjusted_task_paths.is_empty() && bounded.candidate_run_ids.is_empty() {
            run.stop_reason = "no_progress_detected".to_string();
            run.final_status = "no_progress".to_string();
            break;
        }
    }

    if run.final_status == "running" {
        run.stop_reason = "max_rounds_reached".to_string();
        run.final_status = "max_rounds_reached".to_string();
    }
    run.finished_at = memory::now_unix();
    refresh_promotion_queue(project_root, memory_root)?;
    write_supervised_run(memory_root, &run)?;
    Ok(run)
}

pub fn print_last_supervised_run(memory_root: &str) -> Result<String, String> {
    let run = latest_supervised_run(memory_root)?
        .ok_or_else(|| "no supervised runs available".to_string())?;
    print_supervised_run_report(memory_root, &run.supervised_run_id)
}

pub fn print_supervised_run_report(
    memory_root: &str,
    supervised_run_id: &str,
) -> Result<String, String> {
    let path = Path::new(memory_root)
        .join("supervised_runs")
        .join(format!("{supervised_run_id}.ru.md"));
    if path.exists() {
        return fs::read_to_string(path)
            .map_err(|error| format!("failed to read supervised report: {error}"));
    }
    let json_path = Path::new(memory_root)
        .join("supervised_runs")
        .join(format!("{supervised_run_id}.json"));
    let contents = fs::read_to_string(&json_path)
        .map_err(|error| format!("failed to read supervised json: {error}"))?;
    let run: SupervisedRun = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse supervised json: {error}"))?;
    write_supervised_run(memory_root, &run)?;
    fs::read_to_string(path)
        .map_err(|error| format!("failed to read rebuilt supervised report: {error}"))
}

pub fn list_supervised_runs(memory_root: &str) -> Result<Vec<String>, String> {
    let dir = Path::new(memory_root).join("supervised_runs");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut ids = fs::read_dir(dir)
        .map_err(|error| format!("failed to read supervised runs dir: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .filter_map(|path| {
            path.file_stem()
                .and_then(|name| name.to_str())
                .map(str::to_string)
        })
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    Ok(ids)
}

pub fn latest_supervised_run_id(memory_root: &str) -> Result<Option<String>, String> {
    Ok(latest_supervised_run(memory_root)?.map(|run| run.supervised_run_id))
}

fn collect_candidate_outcomes(
    project_root: &str,
    memory_root: &str,
    bounded: &BoundedRunSummary,
    run: &mut SupervisedRun,
) -> Result<(), String> {
    for run_id in &bounded.candidate_run_ids {
        let review = review_candidate(project_root, memory_root, run_id)?;
        if review.promotion_allowed {
            if !run.ready_candidate_run_ids.contains(run_id) {
                run.ready_candidate_run_ids.push(run_id.clone());
            }
        } else if !run.rejected_candidate_run_ids.contains(run_id) {
            run.rejected_candidate_run_ids.push(run_id.clone());
        }
    }
    run.ready_candidate_run_ids.sort();
    run.ready_candidate_run_ids.dedup();
    run.rejected_candidate_run_ids.sort();
    run.rejected_candidate_run_ids.dedup();
    Ok(())
}

fn campaigns_for_bounded(
    memory_root: &str,
    bounded: &BoundedRunSummary,
) -> Result<Vec<EvolutionCampaign>, String> {
    bounded
        .campaign_ids
        .iter()
        .map(|campaign_id| load_campaign_json(memory_root, campaign_id))
        .collect()
}

fn load_campaign_json(memory_root: &str, campaign_id: &str) -> Result<EvolutionCampaign, String> {
    let path = Path::new(memory_root)
        .join("campaigns")
        .join(format!("{campaign_id}.json"));
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read campaign json: {error}"))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse campaign json: {error}"))
}

fn latest_supervised_run(memory_root: &str) -> Result<Option<SupervisedRun>, String> {
    let dir = Path::new(memory_root).join("supervised_runs");
    if !dir.exists() {
        return Ok(None);
    }
    let mut runs = fs::read_dir(dir)
        .map_err(|error| format!("failed to read supervised runs dir: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .filter_map(|path| {
            fs::read_to_string(path)
                .ok()
                .and_then(|contents| serde_json::from_str::<SupervisedRun>(&contents).ok())
        })
        .collect::<Vec<_>>();
    runs.sort_by(|left, right| {
        right
            .finished_at
            .cmp(&left.finished_at)
            .then_with(|| right.supervised_run_id.cmp(&left.supervised_run_id))
    });
    Ok(runs.into_iter().next())
}

fn write_supervised_run(memory_root: &str, run: &SupervisedRun) -> Result<(), String> {
    let dir = Path::new(memory_root).join("supervised_runs");
    fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create supervised runs dir: {error}"))?;
    memory::write_json(dir.join(format!("{}.json", run.supervised_run_id)), run)?;
    fs::write(
        dir.join(format!("{}.ru.md", run.supervised_run_id)),
        render_supervised_run_markdown(memory_root, run),
    )
    .map_err(|error| format!("failed to write supervised markdown: {error}"))
}

fn render_supervised_run_markdown(memory_root: &str, run: &SupervisedRun) -> String {
    let latest_bounded = run
        .bounded_run_ids
        .last()
        .and_then(|run_id| print_bounded_run_report(memory_root, run_id).ok())
        .unwrap_or_else(|| "(none)".to_string());
    format!(
        "# Supervised EVA Run\n\n## Task\ninitial_task_path={}\ncurrent_task_path={}\nmax_rounds={}\nexecuted_rounds={}\n\n## Campaigns\nbounded_run_ids={:?}\ncampaign_ids={:?}\n\n## Adjustments\n{:?}\n\n## Candidates\nready_candidate_run_ids={:?}\nrejected_candidate_run_ids={:?}\n\n## Diagnostics\nzero_yield_rounds={}\nreplay_failed_rounds={}\n\n## Safety\nauto_promote={}\n\n## Final decision\nfinal_status={}\nstop_reason={}\n\n## Latest bounded run\n{}\n",
        run.initial_task_path,
        run.current_task_path,
        run.max_rounds,
        run.executed_rounds,
        run.bounded_run_ids,
        run.campaign_ids,
        run.adjusted_task_paths,
        run.ready_candidate_run_ids,
        run.rejected_candidate_run_ids,
        run.zero_yield_rounds,
        run.replay_failed_rounds,
        run.auto_promote,
        run.final_status,
        run.stop_reason,
        latest_bounded
    )
}
