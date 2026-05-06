use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::contracts::{EvolutionLogEntry, MutationKind};
use crate::evolution::{
    classify_mutation_kind, classify_mutation_kind_label, memory, mutation_class_label,
    MutationClass, ReplayResult,
};

pub const DEFAULT_PORTFOLIO_PATH: &str = "memory/portfolio.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MutationPortfolio {
    #[serde(default)]
    pub kinds: Vec<MutationPortfolioEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutationPortfolioEntry {
    pub mutation_kind: String,
    #[serde(default)]
    pub mutation_class: MutationClass,
    pub seen_count: u64,
    #[serde(default, alias = "success_count")]
    pub useful_success_count: u64,
    #[serde(default)]
    pub cosmetic_count: u64,
    #[serde(default)]
    pub unsafe_count: u64,
    pub candidate_count: u64,
    pub replay_passed_count: u64,
    pub promoted_count: u64,
    pub average_score: f32,
    pub saturation_score: f32,
    pub last_used_at: u64,
}

impl Default for MutationPortfolioEntry {
    fn default() -> Self {
        Self {
            mutation_kind: String::new(),
            mutation_class: MutationClass::Legacy,
            seen_count: 0,
            useful_success_count: 0,
            cosmetic_count: 0,
            unsafe_count: 0,
            candidate_count: 0,
            replay_passed_count: 0,
            promoted_count: 0,
            average_score: 0.0,
            saturation_score: 0.0,
            last_used_at: 0,
        }
    }
}

pub fn load_portfolio(memory_root: &str) -> Result<MutationPortfolio, String> {
    let path = Path::new(memory_root).join("portfolio.json");
    if !path.exists() {
        return Ok(MutationPortfolio::default());
    }
    let contents =
        fs::read_to_string(path).map_err(|error| format!("failed to read portfolio: {error}"))?;
    let mut portfolio: MutationPortfolio = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse portfolio: {error}"))?;
    normalize_portfolio_classes(&mut portfolio);
    Ok(portfolio)
}

pub fn print_portfolio(memory_root: &str) -> Result<String, String> {
    let portfolio = ensure_portfolio(memory_root)?;
    if portfolio.kinds.is_empty() {
        return Ok("(none)".to_string());
    }
    Ok(portfolio
        .kinds
        .iter()
        .map(|entry| {
            format!(
                "{} class={} seen={} useful_success={} cosmetic={} unsafe={} candidates={} replay_passed={} promoted={} avg_score={:.2} saturation={:.2} last_used_at={}",
                entry.mutation_kind,
                mutation_class_label(entry.mutation_class),
                entry.seen_count,
                entry.useful_success_count,
                entry.cosmetic_count,
                entry.unsafe_count,
                entry.candidate_count,
                entry.replay_passed_count,
                entry.promoted_count,
                entry.average_score,
                entry.saturation_score,
                entry.last_used_at
            )
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

pub fn ensure_portfolio(memory_root: &str) -> Result<MutationPortfolio, String> {
    let portfolio = load_portfolio(memory_root)?;
    if portfolio.kinds.is_empty() {
        return refresh_portfolio(memory_root);
    }
    Ok(portfolio)
}

pub fn refresh_portfolio(memory_root: &str) -> Result<MutationPortfolio, String> {
    let mut portfolio = MutationPortfolio::default();
    let logs = load_logs(memory_root)?;
    for entry in &logs {
        let kind = entry.mutation_kind.to_ascii_lowercase();
        let class = classify_mutation_kind_label(&kind, entry.useful_change);
        let slot = upsert_entry(&mut portfolio, &kind);
        slot.mutation_class = merge_class(slot.mutation_class, class);
        let previous_seen = slot.seen_count;
        slot.seen_count += 1;
        if class == MutationClass::Useful && entry.cargo_check_ok && entry.cargo_test_ok {
            slot.useful_success_count += 1;
        }
        if class == MutationClass::Cosmetic {
            slot.cosmetic_count += 1;
        }
        if class == MutationClass::Unsafe {
            slot.unsafe_count += 1;
        }
        if class == MutationClass::Useful
            && entry.status == crate::contracts::EvolutionStatus::Candidate
        {
            slot.candidate_count += 1;
        }
        if class == MutationClass::Useful
            && (entry.status == crate::contracts::EvolutionStatus::Promoted
                || entry.retained_in_core)
        {
            slot.promoted_count += 1;
        }
        slot.average_score = if previous_seen == 0 {
            entry.score
        } else {
            ((slot.average_score * previous_seen as f32) + entry.score) / slot.seen_count as f32
        };
        slot.last_used_at = slot.last_used_at.max(entry.timestamp_unix);
    }

    let replay_dir = Path::new(memory_root).join("replays");
    if replay_dir.exists() {
        let mut replay_paths = fs::read_dir(&replay_dir)
            .map_err(|error| format!("failed to read replays: {error}"))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        replay_paths.sort();
        for path in replay_paths {
            let contents = fs::read_to_string(&path)
                .map_err(|error| format!("failed to read replay file: {error}"))?;
            let replay: ReplayResult = serde_json::from_str(&contents)
                .map_err(|error| format!("failed to parse replay file: {error}"))?;
            if replay.matches_stored_summary
                && replay.cargo_check_ok
                && replay.cargo_test_ok
                && replay.cargo_run_ok
                && replay.replay_status != crate::contracts::EvolutionStatus::Failed
            {
                let run_id = path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default();
                if let Ok(mutation) = memory::load_candidate(memory_root, run_id) {
                    let kind = kind_label(mutation.kind);
                    let class = classify_mutation_kind(mutation.kind, true);
                    let slot = upsert_entry(&mut portfolio, &kind);
                    slot.mutation_class = merge_class(slot.mutation_class, class);
                    if class == MutationClass::Useful {
                        slot.replay_passed_count += 1;
                    }
                    slot.last_used_at = slot.last_used_at.max(replay.timestamp_unix);
                }
            }
        }
    }

    refresh_saturation_scores(&mut portfolio);
    write_portfolio(memory_root, &portfolio)?;
    Ok(portfolio)
}

pub fn update_portfolio_after_log(
    memory_root: &str,
    entry: &EvolutionLogEntry,
) -> Result<MutationPortfolio, String> {
    let mut portfolio = load_portfolio(memory_root)?;
    let kind = entry.mutation_kind.to_ascii_lowercase();
    let class = classify_mutation_kind_label(&kind, entry.useful_change);
    let slot = upsert_entry(&mut portfolio, &kind);
    slot.mutation_class = merge_class(slot.mutation_class, class);
    let previous_seen = slot.seen_count;
    slot.seen_count += 1;
    if class == MutationClass::Useful && entry.cargo_check_ok && entry.cargo_test_ok {
        slot.useful_success_count += 1;
    }
    if class == MutationClass::Cosmetic {
        slot.cosmetic_count += 1;
    }
    if class == MutationClass::Unsafe {
        slot.unsafe_count += 1;
    }
    if class == MutationClass::Useful
        && entry.status == crate::contracts::EvolutionStatus::Candidate
    {
        slot.candidate_count += 1;
    }
    if class == MutationClass::Useful && entry.status == crate::contracts::EvolutionStatus::Promoted
    {
        slot.promoted_count += 1;
    }
    slot.average_score = if previous_seen == 0 {
        entry.score
    } else {
        ((slot.average_score * previous_seen as f32) + entry.score) / slot.seen_count as f32
    };
    slot.last_used_at = entry.timestamp_unix;
    refresh_saturation_scores(&mut portfolio);
    write_portfolio(memory_root, &portfolio)?;
    Ok(portfolio)
}

pub fn update_portfolio_after_replay(
    memory_root: &str,
    mutation_kind: MutationKind,
    replay: &ReplayResult,
) -> Result<MutationPortfolio, String> {
    let mut portfolio = load_portfolio(memory_root)?;
    if replay.matches_stored_summary
        && replay.cargo_check_ok
        && replay.cargo_test_ok
        && replay.cargo_run_ok
        && replay.replay_status != crate::contracts::EvolutionStatus::Failed
    {
        let kind = kind_label(mutation_kind);
        let class = classify_mutation_kind(mutation_kind, true);
        let slot = upsert_entry(&mut portfolio, &kind);
        slot.mutation_class = merge_class(slot.mutation_class, class);
        if class == MutationClass::Useful {
            slot.replay_passed_count += 1;
        }
        slot.last_used_at = replay.timestamp_unix;
    }
    refresh_saturation_scores(&mut portfolio);
    write_portfolio(memory_root, &portfolio)?;
    Ok(portfolio)
}

fn write_portfolio(memory_root: &str, portfolio: &MutationPortfolio) -> Result<(), String> {
    let path = Path::new(memory_root).join("portfolio.json");
    memory::write_json(path, portfolio)
}

fn load_logs(memory_root: &str) -> Result<Vec<EvolutionLogEntry>, String> {
    let path = Path::new(memory_root).join("evolution.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read evolution log: {error}"))?;
    Ok(contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<EvolutionLogEntry>(line).ok())
        .collect())
}

fn upsert_entry<'a>(
    portfolio: &'a mut MutationPortfolio,
    mutation_kind: &str,
) -> &'a mut MutationPortfolioEntry {
    if let Some(index) = portfolio
        .kinds
        .iter()
        .position(|entry| entry.mutation_kind == mutation_kind)
    {
        return &mut portfolio.kinds[index];
    }
    portfolio.kinds.push(MutationPortfolioEntry {
        mutation_kind: mutation_kind.to_string(),
        ..MutationPortfolioEntry::default()
    });
    portfolio
        .kinds
        .sort_by(|left, right| left.mutation_kind.cmp(&right.mutation_kind));
    let index = portfolio
        .kinds
        .iter()
        .position(|entry| entry.mutation_kind == mutation_kind)
        .expect("portfolio entry present");
    &mut portfolio.kinds[index]
}

fn refresh_saturation_scores(portfolio: &mut MutationPortfolio) {
    let total_candidates = portfolio
        .kinds
        .iter()
        .map(|entry| entry.candidate_count)
        .sum::<u64>()
        .max(1);
    for entry in &mut portfolio.kinds {
        let share = entry.candidate_count as f32 / total_candidates as f32;
        entry.saturation_score = if share > 0.6 {
            (share - 0.6).clamp(0.0, 0.4)
        } else {
            0.0
        };
    }
}

pub fn kind_label(kind: MutationKind) -> String {
    format!("{kind:?}").to_ascii_lowercase()
}

fn normalize_portfolio_classes(portfolio: &mut MutationPortfolio) {
    for entry in &mut portfolio.kinds {
        if entry.mutation_class == MutationClass::Legacy {
            entry.mutation_class = classify_mutation_kind_label(
                &entry.mutation_kind,
                entry.useful_success_count > 0
                    || entry.candidate_count > 0
                    || entry.promoted_count > 0,
            );
        }
    }
}

fn merge_class(current: MutationClass, next: MutationClass) -> MutationClass {
    match (current, next) {
        (MutationClass::Unsafe, _) | (_, MutationClass::Unsafe) => MutationClass::Unsafe,
        (MutationClass::Cosmetic, _) | (_, MutationClass::Cosmetic) => MutationClass::Cosmetic,
        (MutationClass::Useful, _) | (_, MutationClass::Useful) => MutationClass::Useful,
        _ => MutationClass::Legacy,
    }
}
