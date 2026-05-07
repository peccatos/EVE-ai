use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::evolution::campaign::EvolutionCampaign;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PolicyFeedback {
    #[serde(default)]
    pub zero_yield_count: u64,
    #[serde(default)]
    pub repeated_task_constraints_too_narrow: u64,
    #[serde(default)]
    pub repeated_duplicate_payload: u64,
    #[serde(default)]
    pub repeated_below_min_score: u64,
    #[serde(default)]
    pub failing_strategy_counts: std::collections::BTreeMap<String, u64>,
    #[serde(default)]
    pub last_campaign_id: String,
    #[serde(default)]
    pub updated_at: u64,
}

pub fn load_policy_feedback(memory_root: &str) -> Result<PolicyFeedback, String> {
    let path = Path::new(memory_root).join("policy_feedback.json");
    if !path.exists() {
        return Ok(PolicyFeedback::default());
    }
    let contents = std::fs::read_to_string(&path)
        .map_err(|error| format!("failed to read policy feedback: {error}"))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse policy feedback: {error}"))
}

pub fn update_policy_feedback(
    memory_root: &str,
    campaign: &EvolutionCampaign,
) -> Result<PolicyFeedback, String> {
    let mut feedback = load_policy_feedback(memory_root)?;
    if campaign.useful_candidates == 0 {
        feedback.zero_yield_count += 1;
    }
    match campaign.zero_candidate_reason.as_deref() {
        Some("task_constraints_too_narrow") => feedback.repeated_task_constraints_too_narrow += 1,
        Some("all_candidates_duplicate") => feedback.repeated_duplicate_payload += 1,
        Some("all_candidates_below_min_score") => feedback.repeated_below_min_score += 1,
        _ => {}
    }
    if !campaign.task_id.is_empty() && campaign.useful_candidates == 0 {
        *feedback
            .failing_strategy_counts
            .entry(campaign.task_id.clone())
            .or_insert(0) += 1;
    }
    feedback.last_campaign_id = campaign.campaign_id.clone();
    feedback.updated_at = crate::evolution::memory::now_unix();
    crate::evolution::memory::write_json(
        Path::new(memory_root).join("policy_feedback.json"),
        &feedback,
    )?;
    Ok(feedback)
}
