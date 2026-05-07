use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskAdjustment {
    pub adjustment_id: String,
    pub source_task_id: String,
    pub source_campaign_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_corpus_id: Option<String>,
    pub zero_candidate_reason: String,
    pub diagnosis_ru: String,
    pub recommended_changes: Vec<String>,
    pub original_task_path: String,
    pub adjusted_task_path: String,
    pub safety_notes: Vec<String>,
    #[serde(default)]
    pub created_at: u64,
}
