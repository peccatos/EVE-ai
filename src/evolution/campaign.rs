use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::contracts::{EvolutionLogEntry, EvolutionStatus, MutationKind, TaskContract};
use crate::evolution::autonomy::autonomy_status;
use crate::evolution::benchmark::count_sandbox_leaks;
use crate::evolution::campaign_recombination::{
    select_task_compatible_hypothesis, CampaignRecombinationDiagnostics,
};
use crate::evolution::memory;
use crate::evolution::task_validator::{
    load_stored_task_contract, load_task_contract, matches_target_patterns, store_task_contract,
    validate_task_contract,
};
use crate::evolution::update_policy_feedback;
use crate::graph::analyzer::propose_mutation_plans;
use crate::promotion::review::review_candidate;
use crate::runtime::{
    run_planned_evolution_cycle_for_task, run_recombined_evolution_cycle_for_hypothesis,
};

const DYNAMIC_REVIEW_BLOCKERS: &[&str] = &[
    "already_promoted",
    "appendcomment_cosmetic",
    "autonomy_blocked",
    "candidate_missing",
    "duplicate_test_function_name",
    "forbidden_target",
    "mutation_missing",
    "promotion_gate_blocked",
    "quality_score_low",
    "report_missing",
    "replay_not_ok",
    "score_below_threshold",
    "target_already_contains_payload",
    "useful_change_false",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvolutionCampaign {
    pub campaign_id: String,
    pub task_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_corpus_id: Option<String>,
    #[serde(default)]
    pub source_task_id: String,
    #[serde(default)]
    pub corpus_derived: bool,
    pub total_cycles: u64,
    pub passed_cycles: u64,
    pub failed_cycles: u64,
    pub useful_candidates: u64,
    pub replay_attempted: u64,
    pub replay_passed: u64,
    pub replay_failed: u64,
    pub duplicate_rejections: u64,
    pub regression_patterns_added: u64,
    pub success_patterns_added: u64,
    pub promotion_ready_candidates: u64,
    pub promoted_candidates: u64,
    pub forbidden_mutations: u64,
    pub sandbox_leaks: u64,
    pub average_score: f32,
    pub started_at: u64,
    pub finished_at: u64,
    pub blocker_counts: Vec<CampaignBlockerCount>,
    pub candidate_run_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zero_candidate_reason: Option<String>,
    #[serde(default)]
    pub rejected_plan_count: usize,
    #[serde(default)]
    pub duplicate_rejection_count: usize,
    #[serde(default)]
    pub below_score_count: usize,
    #[serde(default)]
    pub filtered_by_task_count: usize,
    #[serde(default)]
    pub no_valid_plan_count: usize,
    #[serde(default)]
    pub allowed_target_miss_count: usize,
    #[serde(default)]
    pub allowed_kind_miss_count: usize,
    #[serde(default)]
    pub already_promoted_count: usize,
    #[serde(default)]
    pub repeated_target_penalty_count: usize,
    #[serde(default)]
    pub generated_plan_count: usize,
    #[serde(default)]
    pub accepted_plan_count: usize,
    #[serde(default)]
    pub candidate_generated_count: usize,
    #[serde(default)]
    pub candidate_useful_count: usize,
    #[serde(default)]
    pub candidate_rejected_count: usize,
    #[serde(default)]
    pub candidate_rejected_below_min_score: usize,
    #[serde(default)]
    pub candidate_rejected_duplicate_payload: usize,
    #[serde(default)]
    pub candidate_rejected_failed_validator: usize,
    #[serde(default)]
    pub candidate_rejected_failed_replay: usize,
    #[serde(default)]
    pub candidate_rejected_not_useful: usize,
    #[serde(default)]
    pub candidate_rejected_already_promoted: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_recovery_reason: Option<String>,
    #[serde(default)]
    pub recombination_fallback_attempted: bool,
    #[serde(default)]
    pub recombination_fallback_used: bool,
    #[serde(default)]
    pub recombination_candidates_seen: usize,
    #[serde(default)]
    pub recombination_accepted: usize,
    #[serde(default)]
    pub recombination_rejected_by_target: usize,
    #[serde(default)]
    pub recombination_rejected_by_kind: usize,
    #[serde(default)]
    pub recombination_rejected_by_risk: usize,
    #[serde(default)]
    pub recombination_rejected_by_forbidden_target: usize,
    #[serde(default)]
    pub recombination_rejected_by_class: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_hypothesis_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_risk: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recombination_fallback_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CampaignBlockerCount {
    pub blocker: String,
    pub count: u64,
}

#[derive(Debug, Clone, Default)]
struct CycleDiagnostics {
    generated_plan_count: usize,
    accepted_plan_count: usize,
    filtered_by_task_count: usize,
    no_valid_plan_count: usize,
    allowed_target_miss_count: usize,
    allowed_kind_miss_count: usize,
    repeated_target_penalty_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CampaignFeedback {
    task_id: String,
    source_corpus_id: String,
    last_campaign_id: String,
    zero_candidate_reason: String,
    recommended_adjustments: Vec<String>,
    created_at: u64,
}

pub fn run_task_from_path(
    project_root: &str,
    memory_root: &str,
    task_path: &str,
) -> Result<EvolutionCampaign, String> {
    let task = load_task_contract(Path::new(task_path))?;
    validate_task_contract(&task)?;
    store_task_contract(memory_root, &task)?;
    run_campaign(project_root, memory_root, &task)
}

pub fn run_stored_campaign(
    project_root: &str,
    memory_root: &str,
    task_id: &str,
) -> Result<EvolutionCampaign, String> {
    let task = load_stored_task_contract(memory_root, task_id)?;
    validate_task_contract(&task)?;
    run_campaign(project_root, memory_root, &task)
}

pub fn print_last_campaign_report(memory_root: &str) -> Result<String, String> {
    let campaign =
        latest_campaign(memory_root)?.ok_or_else(|| "no campaign reports available".to_string())?;
    print_campaign_report(memory_root, &campaign.campaign_id)
}

pub fn print_campaign_report(memory_root: &str, campaign_id: &str) -> Result<String, String> {
    let dir = Path::new(memory_root).join("campaigns");
    let markdown_path = dir.join(format!("{campaign_id}.ru.md"));
    if markdown_path.exists() {
        return fs::read_to_string(markdown_path)
            .map_err(|error| format!("failed to read campaign report: {error}"));
    }

    let campaign = load_campaign(memory_root, campaign_id)?
        .ok_or_else(|| format!("campaign json not found for {campaign_id}"))?;
    let task = load_stored_task_contract(memory_root, &campaign.task_id)
        .map_err(|error| format!("campaign report missing and rebuild failed: {error}"))?;
    write_campaign(memory_root, &task, &campaign)?;
    fs::read_to_string(dir.join(format!("{campaign_id}.ru.md")))
        .map_err(|error| format!("failed to read rebuilt campaign report: {error}"))
}

pub fn print_campaign(campaign: &EvolutionCampaign) -> String {
    serde_json::to_string_pretty(campaign).unwrap_or_else(|_| "{}".to_string())
}

pub(crate) fn reconcile_campaign_candidate_state(
    project_root: &str,
    memory_root: &str,
    campaign_id: &str,
) -> Result<EvolutionCampaign, String> {
    let mut campaign = load_campaign(memory_root, campaign_id)?
        .ok_or_else(|| format!("campaign json not found for {campaign_id}"))?;
    if campaign.candidate_run_ids.is_empty() {
        return Ok(campaign);
    }

    let task = load_stored_task_contract(memory_root, &campaign.task_id)?;
    let preserved_rejected = campaign
        .candidate_rejected_count
        .saturating_sub(campaign.candidate_rejected_failed_replay)
        .saturating_sub(campaign.candidate_rejected_already_promoted);
    let preserved_blockers = campaign
        .blocker_counts
        .iter()
        .filter(|item| !DYNAMIC_REVIEW_BLOCKERS.contains(&item.blocker.as_str()))
        .map(|item| (item.blocker.clone(), item.count))
        .collect::<BTreeMap<_, _>>();

    let mut blocker_counts = preserved_blockers;
    let mut replay_passed = 0_u64;
    let mut replay_failed = 0_u64;
    let mut promotion_ready_candidates = 0_u64;
    let mut already_promoted_count = 0_usize;

    for run_id in &campaign.candidate_run_ids {
        let review = review_candidate(project_root, memory_root, run_id)?;
        if review.replay_status == "ok" {
            replay_passed += 1;
        } else {
            replay_failed += 1;
        }
        if review.promotion_allowed {
            promotion_ready_candidates += 1;
        }
        let already_promoted = review
            .promotion_blockers
            .iter()
            .any(|blocker| blocker == "already_promoted");
        if already_promoted {
            already_promoted_count += 1;
        }
        for blocker in &review.promotion_blockers {
            *blocker_counts.entry(blocker.clone()).or_insert(0) += 1;
        }
    }

    campaign.replay_attempted = if task.require_replay {
        campaign.candidate_run_ids.len() as u64
    } else {
        0
    };
    campaign.replay_passed = replay_passed;
    campaign.replay_failed = replay_failed;
    campaign.promotion_ready_candidates = promotion_ready_candidates;
    campaign.already_promoted_count = already_promoted_count;
    campaign.candidate_rejected_failed_replay = replay_failed as usize;
    campaign.candidate_rejected_already_promoted = already_promoted_count;
    campaign.candidate_rejected_count = preserved_rejected
        + campaign.candidate_rejected_failed_replay
        + campaign.candidate_rejected_already_promoted;
    campaign.candidate_recovery_reason =
        if campaign.promotion_ready_candidates > 0 || campaign.useful_candidates > 0 {
            if already_promoted_count > 0 {
                Some("candidate_rejected_already_promoted".to_string())
            } else if replay_failed > 0 {
                Some("candidate_rejected_failed_replay".to_string())
            } else {
                None
            }
        } else {
            campaign.candidate_recovery_reason.clone()
        };
    campaign.blocker_counts = blocker_counts
        .into_iter()
        .map(|(blocker, count)| CampaignBlockerCount { blocker, count })
        .collect();

    write_campaign(memory_root, &task, &campaign)?;
    Ok(campaign)
}

fn run_campaign(
    project_root: &str,
    memory_root: &str,
    task: &TaskContract,
) -> Result<EvolutionCampaign, String> {
    validate_task_contract(task)?;
    let autonomy = autonomy_status(project_root, memory_root)?;
    if autonomy.current_level < 3 || !autonomy.campaign_mode_allowed {
        return Err("campaign mode is blocked by autonomy gate".to_string());
    }
    if task.cycles > autonomy.max_campaign_cycles {
        return Err(format!(
            "campaign cycles exceed autonomy limit: {} > {}",
            task.cycles, autonomy.max_campaign_cycles
        ));
    }
    if task.require_benchmark && !has_benchmark_history(memory_root)? {
        return Err("task requires benchmark history before campaign run".to_string());
    }

    let started_at = memory::now_unix();
    let campaign_id = format!("campaign-{}-{}", task.task_id, started_at);
    let before_regressions = crate::evolution::load_regressions(memory_root)?.len() as u64;
    let before_successes = crate::evolution::load_success_patterns(memory_root)?.len() as u64;
    let before_promotions = count_promotions(memory_root)? as u64;
    let mut campaign = EvolutionCampaign {
        campaign_id: campaign_id.clone(),
        task_id: task.task_id.clone(),
        source_corpus_id: task.source_corpus_id.clone(),
        source_task_id: task.task_id.clone(),
        corpus_derived: task.source_corpus_id.is_some(),
        total_cycles: 0,
        passed_cycles: 0,
        failed_cycles: 0,
        useful_candidates: 0,
        replay_attempted: 0,
        replay_passed: 0,
        replay_failed: 0,
        duplicate_rejections: 0,
        regression_patterns_added: 0,
        success_patterns_added: 0,
        promotion_ready_candidates: 0,
        promoted_candidates: 0,
        forbidden_mutations: 0,
        sandbox_leaks: 0,
        average_score: 0.0,
        started_at,
        finished_at: started_at,
        blocker_counts: Vec::new(),
        candidate_run_ids: Vec::new(),
        zero_candidate_reason: None,
        rejected_plan_count: 0,
        duplicate_rejection_count: 0,
        below_score_count: 0,
        filtered_by_task_count: 0,
        no_valid_plan_count: 0,
        allowed_target_miss_count: 0,
        allowed_kind_miss_count: 0,
        already_promoted_count: 0,
        repeated_target_penalty_count: 0,
        generated_plan_count: 0,
        accepted_plan_count: 0,
        candidate_generated_count: 0,
        candidate_useful_count: 0,
        candidate_rejected_count: 0,
        candidate_rejected_below_min_score: 0,
        candidate_rejected_duplicate_payload: 0,
        candidate_rejected_failed_validator: 0,
        candidate_rejected_failed_replay: 0,
        candidate_rejected_not_useful: 0,
        candidate_rejected_already_promoted: 0,
        candidate_recovery_reason: None,
        recombination_fallback_attempted: false,
        recombination_fallback_used: false,
        recombination_candidates_seen: 0,
        recombination_accepted: 0,
        recombination_rejected_by_target: 0,
        recombination_rejected_by_kind: 0,
        recombination_rejected_by_risk: 0,
        recombination_rejected_by_forbidden_target: 0,
        recombination_rejected_by_class: 0,
        selected_hypothesis_id: None,
        selected_target: None,
        selected_kind: None,
        selected_risk: None,
        recombination_fallback_reason: None,
    };
    let mut blocker_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut total_score = 0.0_f32;

    for _ in 0..task.cycles {
        let diagnostics = collect_cycle_diagnostics(memory_root, task)?;
        campaign.generated_plan_count += diagnostics.generated_plan_count;
        campaign.accepted_plan_count += diagnostics.accepted_plan_count;
        campaign.filtered_by_task_count += diagnostics.filtered_by_task_count;
        campaign.no_valid_plan_count += diagnostics.no_valid_plan_count;
        campaign.allowed_target_miss_count += diagnostics.allowed_target_miss_count;
        campaign.allowed_kind_miss_count += diagnostics.allowed_kind_miss_count;
        campaign.repeated_target_penalty_count += diagnostics.repeated_target_penalty_count;
        if diagnostics.accepted_plan_count == 0 {
            campaign.rejected_plan_count += diagnostics.generated_plan_count.max(1);
        }

        let latest_before = latest_log_entry(memory_root)?.map(|entry| entry.run_id);
        let run_result = if diagnostics.accepted_plan_count == 0 {
            let (hypothesis, recombination_diagnostics) =
                select_task_compatible_hypothesis(memory_root, task)?;
            merge_recombination_diagnostics(&mut campaign, &recombination_diagnostics);
            if let Some(hypothesis) = hypothesis {
                run_recombined_evolution_cycle_for_hypothesis(
                    project_root,
                    memory_root,
                    &hypothesis,
                )
            } else {
                Err(recombination_diagnostics
                    .recombination_fallback_reason
                    .unwrap_or_else(|| "no recombination fallback available".to_string()))
            }
        } else {
            run_planned_evolution_cycle_for_task(project_root, memory_root, Some(task))
        };

        match run_result {
            Ok(()) => {}
            Err(error) => {
                if let Some(entry) = latest_log_entry(memory_root)? {
                    if latest_before.as_deref() != Some(entry.run_id.as_str())
                        && entry.duplicate_rejected
                    {
                        campaign.total_cycles += 1;
                        total_score += entry.score;
                        campaign.failed_cycles += 1;
                        campaign.duplicate_rejections += 1;
                        campaign.duplicate_rejection_count += 1;
                        campaign.candidate_generated_count += 1;
                        campaign.candidate_rejected_count += 1;
                        campaign.candidate_rejected_duplicate_payload += 1;
                        campaign.candidate_recovery_reason =
                            Some("candidate_rejected_duplicate_payload".to_string());
                        campaign.rejected_plan_count += 1;
                        *blocker_counts
                            .entry("duplicate_bad_mutation".to_string())
                            .or_insert(0) += 1;
                        continue;
                    }
                }
                campaign.total_cycles += 1;
                campaign.failed_cycles += 1;
                campaign.candidate_rejected_count += 1;
                campaign.candidate_rejected_failed_validator += 1;
                campaign.candidate_recovery_reason =
                    Some("candidate_rejected_failed_validator".to_string());
                *blocker_counts
                    .entry(blocker_from_error(&error, &diagnostics))
                    .or_insert(0) += 1;
                continue;
            }
        }

        let entry = latest_log_entry(memory_root)?
            .ok_or_else(|| "campaign cycle completed without evolution log entry".to_string())?;
        campaign.total_cycles += 1;
        total_score += entry.score;
        campaign.candidate_generated_count += 1;

        if entry.status == EvolutionStatus::Failed {
            campaign.failed_cycles += 1;
        } else {
            campaign.passed_cycles += 1;
        }
        if entry.duplicate_rejected {
            campaign.duplicate_rejections += 1;
            campaign.duplicate_rejection_count += 1;
            campaign.candidate_rejected_count += 1;
            campaign.candidate_rejected_duplicate_payload += 1;
            campaign.candidate_recovery_reason =
                Some("candidate_rejected_duplicate_payload".to_string());
            campaign.rejected_plan_count += 1;
            *blocker_counts
                .entry("duplicate_bad_mutation".to_string())
                .or_insert(0) += 1;
            continue;
        }
        if is_forbidden_target(&entry.target_file) {
            campaign.forbidden_mutations += 1;
        }
        campaign.sandbox_leaks += count_sandbox_leaks(project_root)?;

        if !(entry.useful_change && entry.status == EvolutionStatus::Candidate) {
            if entry.non_candidate_reason.is_some() {
                campaign.rejected_plan_count += 1;
            }
            campaign.candidate_rejected_count += 1;
            campaign.candidate_rejected_not_useful += 1;
            campaign.candidate_recovery_reason = Some("candidate_rejected_not_useful".to_string());
            continue;
        }

        if entry.score < task.min_score {
            campaign.below_score_count += 1;
            campaign.candidate_rejected_count += 1;
            campaign.candidate_rejected_below_min_score += 1;
            campaign.candidate_recovery_reason =
                Some("candidate_rejected_below_min_score".to_string());
            campaign.rejected_plan_count += 1;
            *blocker_counts
                .entry("below_min_score".to_string())
                .or_insert(0) += 1;
            continue;
        }

        let review = review_candidate(project_root, memory_root, &entry.run_id)?;
        if task.require_replay {
            campaign.replay_attempted += 1;
            let replay_result =
                crate::promotion::replay_candidate(project_root, memory_root, &entry.run_id);
            if replay_result.is_ok() && review.replay_status == "ok" {
                campaign.replay_passed += 1;
            } else {
                campaign.replay_failed += 1;
                campaign.candidate_rejected_count += 1;
                campaign.candidate_rejected_failed_replay += 1;
                campaign.candidate_recovery_reason =
                    Some("candidate_rejected_failed_replay".to_string());
            }
        }

        let already_promoted = review
            .promotion_blockers
            .iter()
            .any(|blocker| blocker == "already_promoted");
        for blocker in &review.promotion_blockers {
            *blocker_counts.entry(blocker.clone()).or_insert(0) += 1;
        }
        if already_promoted {
            campaign.already_promoted_count += 1;
            campaign.candidate_rejected_count += 1;
            campaign.candidate_rejected_already_promoted += 1;
            campaign.candidate_recovery_reason =
                Some("candidate_rejected_already_promoted".to_string());
            campaign.rejected_plan_count += 1;
            continue;
        }

        campaign.useful_candidates += 1;
        campaign.candidate_useful_count += 1;
        campaign.candidate_run_ids.push(entry.run_id.clone());
        if review.promotion_allowed {
            campaign.promotion_ready_candidates += 1;
        }
    }

    campaign.finished_at = memory::now_unix();
    campaign.regression_patterns_added =
        crate::evolution::load_regressions(memory_root)?.len() as u64 - before_regressions;
    campaign.success_patterns_added =
        crate::evolution::load_success_patterns(memory_root)?.len() as u64 - before_successes;
    campaign.promoted_candidates = count_promotions(memory_root)? as u64 - before_promotions;
    campaign.sandbox_leaks += count_sandbox_leaks(project_root)?;
    campaign.average_score = if campaign.total_cycles == 0 {
        0.0
    } else {
        total_score / campaign.total_cycles as f32
    };

    if campaign.total_cycles > 0 && campaign.useful_candidates == 0 {
        let reason = infer_zero_candidate_reason(&campaign);
        campaign.zero_candidate_reason = Some(reason.clone());
        if campaign.candidate_recovery_reason.is_none() {
            campaign.candidate_recovery_reason = Some(reason.clone());
        }
        *blocker_counts.entry(reason.clone()).or_insert(0) += 1;
    }

    campaign.blocker_counts = blocker_counts
        .into_iter()
        .map(|(blocker, count)| CampaignBlockerCount { blocker, count })
        .collect();

    write_campaign(memory_root, task, &campaign)?;
    maybe_write_feedback(memory_root, &campaign)?;
    let _ = update_policy_feedback(memory_root, &campaign);
    Ok(campaign)
}

fn write_campaign(
    memory_root: &str,
    task: &TaskContract,
    campaign: &EvolutionCampaign,
) -> Result<(), String> {
    let dir = Path::new(memory_root).join("campaigns");
    fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create campaigns directory: {error}"))?;
    memory::write_json(dir.join(format!("{}.json", campaign.campaign_id)), campaign)?;
    fs::write(
        dir.join(format!("{}.ru.md", campaign.campaign_id)),
        render_campaign_markdown(task, campaign),
    )
    .map_err(|error| format!("failed to write campaign markdown: {error}"))
}

fn render_campaign_markdown(task: &TaskContract, campaign: &EvolutionCampaign) -> String {
    let blockers = if campaign.blocker_counts.is_empty() {
        "Нет явных blocker reason по кандидатам.".to_string()
    } else {
        campaign
            .blocker_counts
            .iter()
            .map(|item| format!("- {}: {}", item.blocker, item.count))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let ready = if campaign.candidate_run_ids.is_empty() {
        "Нет кандидатов для ручного promotion-review.".to_string()
    } else {
        campaign
            .candidate_run_ids
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    };
    let zero_reason = campaign
        .zero_candidate_reason
        .clone()
        .unwrap_or_else(|| "none".to_string());
    format!(
        "# Campaign EVA\n\n## Цель задачи\n{}\n\n## Ограничения\ncycles={} max_risk={:.2} min_score={:.2} require_replay={} auto_promote={}\nallowed_targets={:?}\nallowed_kinds={:?}\nsource_corpus_id={}\ncorpus_derived={}\n\n## Количество циклов\n{}\n\n## Найденные кандидаты\nПолезных кандидатов: {}\nГотовых к promotion-review: {}\n\n## Replay\nПопыток: {}\nПройдено: {}\nНе пройдено: {}\n\n## Причины отказа по кандидатам\n{}\n\n## Готовые к promotion кандидаты\n{}\n\n## Диагностика результата\nzero_candidate_reason={}\ngenerated_plan_count={}\naccepted_plan_count={}\nrejected_plan_count={}\nfiltered_by_task_count={}\nno_valid_plan_count={}\nallowed_target_miss_count={}\nallowed_kind_miss_count={}\nduplicate_rejection_count={}\nbelow_score_count={}\nalready_promoted_count={}\nrepeated_target_penalty_count={}\n\n## Recombination fallback\nrecombination_fallback_attempted={}\nrecombination_fallback_used={}\nrecombination_candidates_seen={}\nrecombination_accepted={}\nrecombination_rejected_by_target={}\nrecombination_rejected_by_kind={}\nrecombination_rejected_by_risk={}\nrecombination_rejected_by_forbidden_target={}\nrecombination_rejected_by_class={}\nselected_hypothesis_id={:?}\nselected_target={:?}\nselected_kind={:?}\nselected_risk={:?}\nrecombination_fallback_reason={:?}\n\n## Candidate recovery\ncandidate_generated_count={}\ncandidate_useful_count={}\ncandidate_rejected_count={}\ncandidate_rejected_below_min_score={}\ncandidate_rejected_duplicate_payload={}\ncandidate_rejected_failed_validator={}\ncandidate_rejected_failed_replay={}\ncandidate_rejected_not_useful={}\ncandidate_rejected_already_promoted={}\ncandidate_recovery_reason={:?}\n\n## Риски\nforbidden_mutations={} sandbox_leaks={} duplicate_rejections={}\n\n## Итог EVA\nЕва выполнила {} sandbox-циклов. Найдено {} полезных кандидата(ов), {} прошли replay. Promotion автоматически не выполнялся.\n\n## Рекомендация следующего шага\n{}",
        task.goal_ru,
        task.cycles,
        task.max_risk,
        task.min_score,
        task.require_replay,
        task.auto_promote,
        task.allowed_targets,
        task.allowed_mutation_kinds,
        task.source_corpus_id.as_deref().unwrap_or("нет"),
        campaign.corpus_derived,
        campaign.total_cycles,
        campaign.useful_candidates,
        campaign.promotion_ready_candidates,
        campaign.replay_attempted,
        campaign.replay_passed,
        campaign.replay_failed,
        blockers,
        ready,
        zero_reason,
        campaign.generated_plan_count,
        campaign.accepted_plan_count,
        campaign.rejected_plan_count,
        campaign.filtered_by_task_count,
        campaign.no_valid_plan_count,
        campaign.allowed_target_miss_count,
        campaign.allowed_kind_miss_count,
        campaign.duplicate_rejection_count,
        campaign.below_score_count,
        campaign.already_promoted_count,
        campaign.repeated_target_penalty_count,
        campaign.recombination_fallback_attempted,
        campaign.recombination_fallback_used,
        campaign.recombination_candidates_seen,
        campaign.recombination_accepted,
        campaign.recombination_rejected_by_target,
        campaign.recombination_rejected_by_kind,
        campaign.recombination_rejected_by_risk,
        campaign.recombination_rejected_by_forbidden_target,
        campaign.recombination_rejected_by_class,
        campaign.selected_hypothesis_id,
        campaign.selected_target,
        campaign.selected_kind,
        campaign.selected_risk,
        campaign.recombination_fallback_reason,
        campaign.candidate_generated_count,
        campaign.candidate_useful_count,
        campaign.candidate_rejected_count,
        campaign.candidate_rejected_below_min_score,
        campaign.candidate_rejected_duplicate_payload,
        campaign.candidate_rejected_failed_validator,
        campaign.candidate_rejected_failed_replay,
        campaign.candidate_rejected_not_useful,
        campaign.candidate_rejected_already_promoted,
        campaign.candidate_recovery_reason,
        campaign.forbidden_mutations,
        campaign.sandbox_leaks,
        campaign.duplicate_rejections,
        campaign.total_cycles,
        campaign.useful_candidates,
        campaign.replay_passed,
        if campaign.promotion_ready_candidates > 0 {
            "Рекомендуется вручную рассмотреть replay-подтверждённые кандидаты через --review-candidate."
        } else {
            "Если zero-yield повторится, изучите adjustment draft и bounded loop feedback."
        }
    )
}

fn collect_cycle_diagnostics(
    memory_root: &str,
    task: &TaskContract,
) -> Result<CycleDiagnostics, String> {
    let plans = propose_mutation_plans(memory_root)?;
    if plans.is_empty() {
        return Ok(CycleDiagnostics {
            no_valid_plan_count: 1,
            ..CycleDiagnostics::default()
        });
    }

    let repeated_target_overused = repeated_target_overused(memory_root)?;
    let mut diagnostics = CycleDiagnostics {
        generated_plan_count: plans.len(),
        ..CycleDiagnostics::default()
    };
    for plan in plans {
        let target_allowed = task.allowed_targets.is_empty()
            || matches_target_patterns(&plan.target_file, &task.allowed_targets);
        let target_forbidden = matches_target_patterns(&plan.target_file, &task.forbidden_targets);
        let objective_allowed = task.preferred_objectives.is_empty()
            || task.preferred_objectives.contains(&plan.objective);
        let kind_allowed = task.allowed_mutation_kinds.is_empty()
            || task.allowed_mutation_kinds.contains(&plan.mutation_kind);
        let risk_allowed = plan.estimated_risk <= task.max_risk;

        if !target_allowed || target_forbidden {
            diagnostics.allowed_target_miss_count += 1;
        }
        if !kind_allowed {
            diagnostics.allowed_kind_miss_count += 1;
        }
        if !target_allowed
            || target_forbidden
            || !objective_allowed
            || !kind_allowed
            || !risk_allowed
        {
            diagnostics.filtered_by_task_count += 1;
            continue;
        }
        if repeated_target_overused && plan.target_file == "tests/evolution_generated_tests.rs" {
            diagnostics.repeated_target_penalty_count += 1;
            continue;
        }
        diagnostics.accepted_plan_count += 1;
    }
    if diagnostics.accepted_plan_count == 0 {
        diagnostics.no_valid_plan_count = 1;
    }
    Ok(diagnostics)
}

fn infer_zero_candidate_reason(campaign: &EvolutionCampaign) -> String {
    if campaign.recombination_fallback_attempted
        && !campaign.recombination_fallback_used
        && campaign.accepted_plan_count == 0
        && campaign.filtered_by_task_count > 0
    {
        return "task_constraints_too_narrow".to_string();
    }
    if campaign.no_valid_plan_count > 0 && campaign.generated_plan_count == 0 {
        return "no_valid_plan".to_string();
    }
    if campaign.allowed_target_miss_count > 0
        && campaign.accepted_plan_count == 0
        && campaign.allowed_target_miss_count >= campaign.allowed_kind_miss_count
    {
        return "allowed_targets_filtered_all".to_string();
    }
    if campaign.allowed_kind_miss_count > 0 && campaign.accepted_plan_count == 0 {
        return "allowed_kinds_filtered_all".to_string();
    }
    if campaign.repeated_target_penalty_count > 0 && campaign.accepted_plan_count == 0 {
        return "repeated_target_pressure_too_high".to_string();
    }
    if campaign.duplicate_rejection_count > 0 {
        return "all_candidates_duplicate".to_string();
    }
    if campaign.below_score_count > 0 {
        return "all_candidates_below_min_score".to_string();
    }
    if campaign.already_promoted_count > 0 {
        return "all_candidates_already_promoted".to_string();
    }
    if campaign.filtered_by_task_count > 0 {
        return "task_constraints_too_narrow".to_string();
    }
    if campaign.no_valid_plan_count > 0 {
        return "no_valid_plan".to_string();
    }
    "unknown_zero_yield".to_string()
}

fn blocker_from_error(error: &str, diagnostics: &CycleDiagnostics) -> String {
    if diagnostics.accepted_plan_count == 0 && diagnostics.allowed_target_miss_count > 0 {
        "allowed_targets_filtered_all".to_string()
    } else if diagnostics.accepted_plan_count == 0 && diagnostics.allowed_kind_miss_count > 0 {
        "allowed_kinds_filtered_all".to_string()
    } else if error.contains("no graph-guided plans available") {
        "no_valid_plan".to_string()
    } else {
        "campaign_cycle_failed".to_string()
    }
}

fn maybe_write_feedback(memory_root: &str, campaign: &EvolutionCampaign) -> Result<(), String> {
    let Some(source_corpus_id) = campaign.source_corpus_id.clone() else {
        return Ok(());
    };
    let Some(reason) = campaign.zero_candidate_reason.clone() else {
        return Ok(());
    };

    let feedback = CampaignFeedback {
        task_id: campaign.task_id.clone(),
        source_corpus_id,
        last_campaign_id: campaign.campaign_id.clone(),
        zero_candidate_reason: reason.clone(),
        recommended_adjustments: recommended_adjustments(&reason),
        created_at: memory::now_unix(),
    };
    let path = Path::new(memory_root)
        .join("tasks")
        .join("feedback")
        .join(format!("{}.json", campaign.task_id));
    memory::write_json(path, &feedback)
}

fn recommended_adjustments(reason: &str) -> Vec<String> {
    let mut values = vec![
        "run --recombine-patterns before campaign".to_string(),
        "inspect --evolution-policy".to_string(),
    ];
    match reason {
        "allowed_targets_filtered_all" | "task_constraints_too_narrow" => {
            values.insert(0, "loosen allowed_targets".to_string());
            values.insert(
                1,
                "allow src/evolution/* for metrics/reporting tasks".to_string(),
            );
        }
        "allowed_kinds_filtered_all" => {
            values.insert(0, "add another allowed mutation kind".to_string());
        }
        "all_candidates_below_min_score" => {
            values.insert(0, "reduce min_score".to_string());
        }
        "repeated_target_pressure_too_high" => {
            values.insert(0, "loosen allowed_targets".to_string());
            values.insert(1, "add another allowed mutation kind".to_string());
        }
        _ => {}
    }
    values
}

fn latest_campaign(memory_root: &str) -> Result<Option<EvolutionCampaign>, String> {
    let dir = Path::new(memory_root).join("campaigns");
    if !dir.exists() {
        return Ok(None);
    }
    let mut campaigns = fs::read_dir(&dir)
        .map_err(|error| format!("failed to read campaign directory: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .filter_map(|path| {
            fs::read_to_string(path)
                .ok()
                .and_then(|contents| serde_json::from_str::<EvolutionCampaign>(&contents).ok())
        })
        .collect::<Vec<_>>();
    campaigns.sort_by(|left, right| {
        right
            .finished_at
            .cmp(&left.finished_at)
            .then_with(|| right.started_at.cmp(&left.started_at))
            .then_with(|| right.campaign_id.cmp(&left.campaign_id))
    });
    Ok(campaigns.into_iter().next())
}

fn load_campaign(
    memory_root: &str,
    campaign_id: &str,
) -> Result<Option<EvolutionCampaign>, String> {
    let path = Path::new(memory_root)
        .join("campaigns")
        .join(format!("{campaign_id}.json"));
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read campaign json: {error}"))?;
    let campaign = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse campaign json: {error}"))?;
    Ok(Some(campaign))
}

fn repeated_target_overused(memory_root: &str) -> Result<bool, String> {
    let mut summaries = memory::list_candidate_summaries(memory_root)?;
    summaries.sort_by(|left, right| right.timestamp_unix.cmp(&left.timestamp_unix));
    let recent = summaries.into_iter().take(10).collect::<Vec<_>>();
    if recent.is_empty() {
        return Ok(false);
    }
    let repeated = recent
        .iter()
        .filter(|summary| summary.target_file == "tests/evolution_generated_tests.rs")
        .count();
    Ok(repeated * 10 >= recent.len() * 6)
}

fn has_benchmark_history(memory_root: &str) -> Result<bool, String> {
    let dir = Path::new(memory_root).join("benchmarks");
    if !dir.exists() {
        return Ok(false);
    }
    Ok(fs::read_dir(dir)
        .map_err(|error| format!("failed to read benchmark directory: {error}"))?
        .filter_map(Result::ok)
        .any(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "json")
        }))
}

fn latest_log_entry(memory_root: &str) -> Result<Option<EvolutionLogEntry>, String> {
    let path = Path::new(memory_root).join("evolution.jsonl");
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read evolution log: {error}"))?;
    for line in contents.lines().rev() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<EvolutionLogEntry>(line) {
            return Ok(Some(entry));
        }
    }
    Ok(None)
}

fn count_promotions(memory_root: &str) -> Result<usize, String> {
    let path = Path::new(memory_root).join("evolution.jsonl");
    if !path.exists() {
        return Ok(0);
    }
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read evolution log: {error}"))?;
    Ok(contents
        .lines()
        .filter_map(|line| serde_json::from_str::<EvolutionLogEntry>(line).ok())
        .filter(|entry| entry.retained_in_core)
        .count())
}

fn is_forbidden_target(target_file: &str) -> bool {
    target_file.starts_with("src/core/")
        || target_file == "src/main.rs"
        || target_file == "src/lib.rs"
        || target_file == "Cargo.toml"
        || target_file.ends_with("/Cargo.toml")
}

fn merge_recombination_diagnostics(
    campaign: &mut EvolutionCampaign,
    diagnostics: &CampaignRecombinationDiagnostics,
) {
    campaign.recombination_fallback_attempted |= diagnostics.recombination_fallback_attempted;
    campaign.recombination_fallback_used |= diagnostics.recombination_fallback_used;
    campaign.recombination_candidates_seen += diagnostics.recombination_candidates_seen;
    campaign.recombination_accepted += diagnostics.recombination_accepted;
    campaign.recombination_rejected_by_target += diagnostics.recombination_rejected_by_target;
    campaign.recombination_rejected_by_kind += diagnostics.recombination_rejected_by_kind;
    campaign.recombination_rejected_by_risk += diagnostics.recombination_rejected_by_risk;
    campaign.recombination_rejected_by_forbidden_target +=
        diagnostics.recombination_rejected_by_forbidden_target;
    campaign.recombination_rejected_by_class += diagnostics.recombination_rejected_by_class;
    if diagnostics.selected_hypothesis_id.is_some() {
        campaign.selected_hypothesis_id = diagnostics.selected_hypothesis_id.clone();
        campaign.selected_target = diagnostics.selected_target.clone();
        campaign.selected_kind = diagnostics.selected_kind.clone();
        campaign.selected_risk = diagnostics.selected_risk;
    }
    if diagnostics.recombination_fallback_reason.is_some() {
        campaign.recombination_fallback_reason = diagnostics.recombination_fallback_reason.clone();
    }
}

#[allow(dead_code)]
fn _allowed_kind_name(kind: MutationKind) -> &'static str {
    match kind {
        MutationKind::AppendComment => "appendcomment",
        MutationKind::ReplaceText => "replacetext",
        MutationKind::ParameterTune => "parametertune",
        MutationKind::AddTestSkeleton => "addtestskeleton",
        MutationKind::AddMetricField => "addmetricfield",
        MutationKind::AddUnitTest => "addunittest",
        MutationKind::AddReplayAssertion => "addreplayassertion",
        MutationKind::AddLearningSummaryField => "addlearningsummaryfield",
        MutationKind::AddMetricUpdate => "addmetricupdate",
    }
}
