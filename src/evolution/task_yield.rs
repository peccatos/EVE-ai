use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::contracts::{DeniedMutationKind, MutationKind, TaskAdjustment, TaskContract};
use crate::evolution::campaign::EvolutionCampaign;
use crate::evolution::memory;
use crate::evolution::task_validator::{
    load_stored_task_contract, load_task_contract, validate_task_contract,
};

const SAFE_TARGETS: [&str; 4] = [
    "tests/*",
    "src/evolution/*",
    "src/promotion/*",
    "src/sandbox/*",
];
const SAFE_TARGETS_WITH_RUNTIME: [&str; 5] = [
    "tests/*",
    "src/evolution/*",
    "src/promotion/*",
    "src/sandbox/*",
    "src/runtime/*",
];
const SAFE_KINDS: [MutationKind; 4] = [
    MutationKind::AddUnitTest,
    MutationKind::AddReplayAssertion,
    MutationKind::AddLearningSummaryField,
    MutationKind::AddMetricUpdate,
];

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct TaskFeedback {
    #[serde(default)]
    task_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_corpus_id: Option<String>,
    #[serde(default)]
    last_campaign_id: String,
    #[serde(default)]
    zero_candidate_reason: String,
    #[serde(default)]
    recommended_adjustments: Vec<String>,
    #[serde(default)]
    created_at: u64,
}

pub fn adjust_task_from_campaign(
    memory_root: &str,
    campaign_id: &str,
) -> Result<TaskAdjustment, String> {
    let campaign = load_campaign(memory_root, campaign_id)?
        .ok_or_else(|| format!("campaign json not found for {campaign_id}"))?;
    if campaign.useful_candidates > 0 {
        return Err(
            "campaign already produced useful candidates; adjustment not needed".to_string(),
        );
    }
    let zero_reason = campaign
        .zero_candidate_reason
        .clone()
        .ok_or_else(|| "campaign is missing zero_candidate_reason".to_string())?;

    let (source_task, original_task_path) = resolve_source_task(memory_root, &campaign)?;
    let feedback = load_feedback(memory_root, &campaign.task_id).ok();
    let analysis = analyze_zero_yield(&campaign, &source_task, feedback.as_ref(), &zero_reason);
    let created_at = memory::now_unix();
    let adjustment_id = format!("adjustment-{}-{created_at}", campaign.task_id);
    let adjusted_dir = Path::new(memory_root).join("tasks").join("adjusted");
    fs::create_dir_all(&adjusted_dir)
        .map_err(|error| format!("failed to create adjusted task directory: {error}"))?;
    let adjusted_task_path = adjusted_dir.join(format!("{}.adjusted.task.json", campaign.task_id));
    let adjustment_path = adjusted_dir.join(format!("{}.adjustment.json", campaign.task_id));
    let markdown_path = adjusted_dir.join(format!("{}.ru.md", campaign.task_id));

    if let Some(task) = &analysis.adjusted_task {
        validate_adjusted_task(task, source_task.as_ref())?;
        memory::write_json(&adjusted_task_path, task)?;
    }

    let adjustment = TaskAdjustment {
        adjustment_id,
        source_task_id: campaign.task_id.clone(),
        source_campaign_id: campaign.campaign_id.clone(),
        source_corpus_id: campaign.source_corpus_id.clone(),
        zero_candidate_reason: zero_reason,
        diagnosis_ru: analysis.diagnosis_ru.clone(),
        recommended_changes: analysis.recommended_changes.clone(),
        original_task_path: original_task_path
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|| "unresolved".to_string()),
        adjusted_task_path: if analysis.adjusted_task.is_some() {
            adjusted_task_path.to_string_lossy().to_string()
        } else {
            String::new()
        },
        safety_notes: safety_notes(),
        created_at,
    };
    memory::write_json(&adjustment_path, &adjustment)?;
    fs::write(
        &markdown_path,
        render_adjustment_markdown(
            &adjustment,
            &campaign,
            source_task.as_ref(),
            analysis.adjusted_task.as_ref(),
            feedback.as_ref(),
        ),
    )
    .map_err(|error| format!("failed to write adjustment markdown: {error}"))?;
    Ok(adjustment)
}

pub fn print_last_task_adjustment(memory_root: &str) -> Result<String, String> {
    let adjustment = latest_adjustment(memory_root)?
        .ok_or_else(|| "no task adjustments available".to_string())?;
    let path = Path::new(memory_root)
        .join("tasks")
        .join("adjusted")
        .join(format!("{}.ru.md", adjustment.source_task_id));
    fs::read_to_string(&path)
        .map_err(|error| format!("failed to read last task adjustment: {error}"))
}

pub fn list_adjusted_tasks(memory_root: &str) -> Result<Vec<String>, String> {
    let dir = Path::new(memory_root).join("tasks").join("adjusted");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut ids = fs::read_dir(&dir)
        .map_err(|error| format!("failed to read adjusted task dir: {error}"))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .path()
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_string)
        })
        .filter_map(|name| name.strip_suffix(".adjusted.task.json").map(str::to_string))
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    Ok(ids)
}

#[derive(Debug, Clone)]
struct YieldAnalysis {
    diagnosis_ru: String,
    recommended_changes: Vec<String>,
    adjusted_task: Option<TaskContract>,
}

fn analyze_zero_yield(
    campaign: &EvolutionCampaign,
    source_task: &Option<TaskContract>,
    feedback: Option<&TaskFeedback>,
    zero_reason: &str,
) -> YieldAnalysis {
    let mut recommended_changes = Vec::new();
    if let Some(feedback) = feedback {
        recommended_changes.extend(feedback.recommended_adjustments.clone());
    }
    let Some(task) = source_task.clone() else {
        return YieldAnalysis {
            diagnosis_ru: format!(
                "Не удалось найти исходный task для кампании {}. Доступна только диагностика zero-yield.",
                campaign.campaign_id
            ),
            recommended_changes,
            adjusted_task: None,
        };
    };

    let mut adjusted = task.clone();
    adjusted.auto_promote = false;
    adjusted.require_russian_report = true;
    adjusted.require_replay = true;
    adjusted.max_risk = adjusted.max_risk.min(0.25);
    adjusted.min_score = adjusted.min_score.max(5.0);
    adjusted.cycles = (adjusted.cycles + 1).min((task.cycles + 2).min(5));
    adjusted.allowed_targets = sanitize_targets(&adjusted.allowed_targets);
    adjusted.allowed_mutation_kinds = sanitize_kinds(&adjusted.allowed_mutation_kinds);
    adjusted.denied_mutation_kinds = vec![
        DeniedMutationKind::DeleteCode,
        DeniedMutationKind::RewriteFunction,
        DeniedMutationKind::FreeDiff,
        DeniedMutationKind::DependencyAdd,
    ];

    let diagnosis_ru = match zero_reason {
        "task_constraints_too_narrow" => {
            adjusted.allowed_targets = broaden_targets(&adjusted.allowed_targets, true);
            adjusted.allowed_mutation_kinds = broaden_kinds(&adjusted.allowed_mutation_kinds);
            adjusted.min_score = (adjusted.min_score - 0.5).max(5.0);
            recommended_changes.extend([
                "safe target expansion applied".to_string(),
                "safe mutation kind expansion applied".to_string(),
                "min_score reduced slightly".to_string(),
            ]);
            "Кампания не дала кандидатов, потому что task отфильтровал все безопасные планы. Черновик расширяет только безопасные target families и полезные mutation kinds.".to_string()
        }
        "allowed_targets_filtered_all" => {
            adjusted.allowed_targets = broaden_targets(&adjusted.allowed_targets, true);
            recommended_changes.push("safe target expansion applied".to_string());
            "Все планы были отброшены по allowed_targets. Черновик расширяет target family только внутри tests/src/evolution/src/promotion/src/sandbox/src/runtime.".to_string()
        }
        "allowed_kinds_filtered_all" => {
            adjusted.allowed_mutation_kinds = broaden_kinds(&adjusted.allowed_mutation_kinds);
            recommended_changes.push("safe mutation kind expansion applied".to_string());
            "Все планы были отброшены по allowed_mutation_kinds. Черновик добавляет только полезные безопасные kinds.".to_string()
        }
        "all_candidates_below_min_score" => {
            adjusted.min_score = (adjusted.min_score - 1.0).max(5.0);
            adjusted.allowed_targets = broaden_targets(&adjusted.allowed_targets, false);
            recommended_changes.push("min_score reduced but kept >= 5.0".to_string());
            "Планы были полезными, но не дотянули до min_score. Черновик снижает порог, не опускаясь ниже 5.0.".to_string()
        }
        "all_candidates_duplicate" => {
            adjusted.allowed_mutation_kinds = diversify_kinds(&adjusted.allowed_mutation_kinds);
            adjusted.allowed_targets = broaden_targets(&adjusted.allowed_targets, false);
            recommended_changes.push("diversified kind/target family".to_string());
            "Планы упёрлись в duplicate history. Черновик смещает mutation portfolio в другие безопасные kind/target families.".to_string()
        }
        "all_candidates_already_promoted" => {
            adjusted.allowed_targets = broaden_targets(&adjusted.allowed_targets, false)
                .into_iter()
                .filter(|target| target != "tests/*")
                .collect::<Vec<_>>();
            if adjusted.allowed_targets.is_empty() {
                adjusted.allowed_targets =
                    vec!["src/evolution/*".to_string(), "src/promotion/*".to_string()];
            }
            recommended_changes.push("different safe target family selected".to_string());
            "Кандидаты уже были продвинуты ранее. Черновик уводит задачу в другую безопасную target family.".to_string()
        }
        unknown => {
            return YieldAnalysis {
                diagnosis_ru: format!(
                    "Причина zero-yield `{unknown}` не имеет безопасного автоматического шаблона. Сформирована только диагностика без нового task draft."
                ),
                recommended_changes,
                adjusted_task: None,
            };
        }
    };

    adjusted.allowed_targets = sanitize_targets(&adjusted.allowed_targets);
    adjusted.allowed_mutation_kinds = sanitize_kinds(&adjusted.allowed_mutation_kinds);
    adjusted.max_risk = adjusted.max_risk.min(0.25);
    adjusted.min_score = adjusted.min_score.max(5.0);
    adjusted.cycles = adjusted.cycles.min((task.cycles + 2).min(5));

    YieldAnalysis {
        diagnosis_ru,
        recommended_changes: dedup_strings(recommended_changes),
        adjusted_task: Some(adjusted),
    }
}

fn render_adjustment_markdown(
    adjustment: &TaskAdjustment,
    campaign: &EvolutionCampaign,
    source_task: Option<&TaskContract>,
    adjusted_task: Option<&TaskContract>,
    feedback: Option<&TaskFeedback>,
) -> String {
    let original_summary = source_task
        .map(|task| {
            format!(
                "task_id={} allowed_targets={:?} allowed_mutation_kinds={:?} cycles={} min_score={:.2} max_risk={:.2}",
                task.task_id, task.allowed_targets, task.allowed_mutation_kinds, task.cycles, task.min_score, task.max_risk
            )
        })
        .unwrap_or_else(|| "Исходный task не найден.".to_string());
    let feedback_block = feedback
        .map(|value| {
            if value.recommended_adjustments.is_empty() {
                "feedback: нет дополнительных рекомендаций".to_string()
            } else {
                format!("feedback: {}", value.recommended_adjustments.join(", "))
            }
        })
        .unwrap_or_else(|| "feedback: отсутствует".to_string());
    let safe_changes = if let Some(task) = adjusted_task {
        format!(
            "allowed_targets={:?}\nallowed_mutation_kinds={:?}\ncycles={}\nmin_score={:.2}\nmax_risk={:.2}\nauto_promote={}\nrequire_replay={}",
            task.allowed_targets,
            task.allowed_mutation_kinds,
            task.cycles,
            task.min_score,
            task.max_risk,
            task.auto_promote,
            task.require_replay
        )
    } else {
        "Новый task draft не был создан.".to_string()
    };
    let adjusted_json = adjusted_task
        .map(|task| serde_json::to_string_pretty(task).unwrap_or_else(|_| "{}".to_string()))
        .unwrap_or_else(|| "{}".to_string());
    let next_command = if adjustment.adjusted_task_path.is_empty() {
        "Новый task draft не создан; сначала изучите диагностику и feedback.".to_string()
    } else {
        format!("cargo run -- --run-task {}", adjustment.adjusted_task_path)
    };
    format!(
        "# Task Adjustment EVA\n\n## Исходная проблема\ncampaign_id={}\ntask_id={}\nzero_candidate_reason={}\nblocker_counts={:?}\n\n## Диагностика\n{}\n{}\n{}\n\n## Безопасные изменения\n{}\n\n## Что не было разрешено\n- src/core/*\n- src/main.rs\n- src/lib.rs\n- Cargo.toml\n- delete_code\n- rewrite_function\n- free_diff\n- dependency_add\n- append_comment\n- network\n- auto_promote=true\n\n## Новый task draft\n```json\n{}\n```\n\n## Следующая команда\n`{}`\n",
        campaign.campaign_id,
        campaign.task_id,
        adjustment.zero_candidate_reason,
        campaign.blocker_counts,
        adjustment.diagnosis_ru,
        original_summary,
        feedback_block,
        safe_changes,
        adjusted_json,
        next_command
    )
}

fn resolve_source_task(
    memory_root: &str,
    campaign: &EvolutionCampaign,
) -> Result<(Option<TaskContract>, Option<PathBuf>), String> {
    let candidate_paths = [
        Path::new(memory_root)
            .join("tasks")
            .join(format!("{}.task.json", campaign.task_id)),
        Path::new(memory_root)
            .join("tasks")
            .join("suggested")
            .join(format!("{}.task.json", campaign.task_id)),
        Path::new(memory_root)
            .join("tasks")
            .join("adjusted")
            .join(format!("{}.adjusted.task.json", campaign.task_id)),
    ];
    for path in candidate_paths {
        if path.exists() {
            let task = load_task_contract(&path)?;
            return Ok((Some(task), Some(path)));
        }
    }
    if let Ok(task) = load_stored_task_contract(memory_root, &campaign.task_id) {
        let path = Path::new(memory_root)
            .join("tasks")
            .join(format!("{}.task.json", campaign.task_id));
        return Ok((Some(task), Some(path)));
    }
    Ok((None, None))
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

fn load_feedback(memory_root: &str, task_id: &str) -> Result<TaskFeedback, String> {
    let path = Path::new(memory_root)
        .join("tasks")
        .join("feedback")
        .join(format!("{task_id}.json"));
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read task feedback: {error}"))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse task feedback: {error}"))
}

fn latest_adjustment(memory_root: &str) -> Result<Option<TaskAdjustment>, String> {
    let dir = Path::new(memory_root).join("tasks").join("adjusted");
    if !dir.exists() {
        return Ok(None);
    }
    let mut adjustments = fs::read_dir(&dir)
        .map_err(|error| format!("failed to read adjusted task dir: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .filter(|path| path.to_string_lossy().ends_with(".adjustment.json"))
        .filter_map(|path| {
            fs::read_to_string(path)
                .ok()
                .and_then(|contents| serde_json::from_str::<TaskAdjustment>(&contents).ok())
        })
        .collect::<Vec<_>>();
    adjustments.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.adjustment_id.cmp(&left.adjustment_id))
    });
    Ok(adjustments.into_iter().next())
}

fn validate_adjusted_task(
    task: &TaskContract,
    original: Option<&TaskContract>,
) -> Result<(), String> {
    validate_task_contract(task)?;
    if task.auto_promote {
        return Err("adjusted task must keep auto_promote=false".to_string());
    }
    if !task.require_russian_report {
        return Err("adjusted task must keep require_russian_report=true".to_string());
    }
    if !task.require_replay {
        return Err("adjusted task must keep require_replay=true".to_string());
    }
    if task.max_risk > 0.25 {
        return Err("adjusted task max_risk must stay <= 0.25".to_string());
    }
    if task.min_score < 5.0 {
        return Err("adjusted task min_score must stay >= 5.0".to_string());
    }
    if task
        .allowed_targets
        .iter()
        .any(|target| !SAFE_TARGETS_WITH_RUNTIME.contains(&target.as_str()))
    {
        return Err("adjusted task contains target outside safe families".to_string());
    }
    if task
        .allowed_mutation_kinds
        .iter()
        .any(|kind| !SAFE_KINDS.contains(kind))
    {
        return Err("adjusted task contains unsafe mutation kind".to_string());
    }
    if let Some(original) = original {
        let max_cycles = (original.cycles + 2).min(5);
        if task.cycles > max_cycles {
            return Err("adjusted task cycles exceed safe cap".to_string());
        }
    } else if task.cycles > 5 {
        return Err("adjusted task cycles exceed safe cap".to_string());
    }
    Ok(())
}

fn broaden_targets(existing: &[String], include_runtime: bool) -> Vec<String> {
    let mut targets = sanitize_targets(existing);
    let source = if include_runtime {
        SAFE_TARGETS_WITH_RUNTIME.as_slice()
    } else {
        SAFE_TARGETS.as_slice()
    };
    for target in source {
        if !targets.iter().any(|candidate| candidate == target) {
            targets.push((*target).to_string());
        }
    }
    targets
}

fn broaden_kinds(existing: &[MutationKind]) -> Vec<MutationKind> {
    let mut kinds = sanitize_kinds(existing);
    for kind in SAFE_KINDS {
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    kinds
}

fn diversify_kinds(existing: &[MutationKind]) -> Vec<MutationKind> {
    let mut kinds = sanitize_kinds(existing);
    if kinds == vec![MutationKind::AddUnitTest] {
        kinds = vec![
            MutationKind::AddReplayAssertion,
            MutationKind::AddMetricUpdate,
            MutationKind::AddLearningSummaryField,
        ];
    } else if !kinds.contains(&MutationKind::AddReplayAssertion) {
        kinds.push(MutationKind::AddReplayAssertion);
    }
    kinds
}

fn sanitize_targets(existing: &[String]) -> Vec<String> {
    let mut values = existing
        .iter()
        .filter(|target| SAFE_TARGETS_WITH_RUNTIME.contains(&target.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    if values.is_empty() {
        values.push("tests/*".to_string());
    }
    values
}

fn sanitize_kinds(existing: &[MutationKind]) -> Vec<MutationKind> {
    let mut values = existing
        .iter()
        .filter(|kind| SAFE_KINDS.contains(kind))
        .copied()
        .collect::<Vec<_>>();
    if values.is_empty() {
        values = vec![MutationKind::AddUnitTest, MutationKind::AddReplayAssertion];
    }
    values.sort_by_key(|kind| kind_label(*kind));
    values.dedup();
    values
}

fn kind_label(kind: MutationKind) -> &'static str {
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

fn dedup_strings(values: Vec<String>) -> Vec<String> {
    let mut values = values
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn safety_notes() -> Vec<String> {
    vec![
        "original task was not mutated".to_string(),
        "auto_promote remains false".to_string(),
        "only safe target families were allowed".to_string(),
        "unsafe mutation kinds were not allowed".to_string(),
        "network and LLM remain disabled".to_string(),
    ]
}
