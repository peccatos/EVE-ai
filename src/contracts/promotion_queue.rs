use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PromotionQueueItem {
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub mutation_kind: String,
    #[serde(default)]
    pub mutation_class: String,
    #[serde(default)]
    pub target_file: String,
    #[serde(default)]
    pub score: f32,
    #[serde(default)]
    pub risk: f32,
    #[serde(default)]
    pub replay_status: String,
    #[serde(default)]
    pub promotion_state: String,
    #[serde(default)]
    pub promotion_allowed: bool,
    #[serde(default)]
    pub promotion_blockers: Vec<String>,
    #[serde(default)]
    pub report_path: String,
    #[serde(default)]
    pub lifecycle_state: String,
    #[serde(default)]
    pub reason_ru: String,
    #[serde(default)]
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PromotionQueue {
    #[serde(default)]
    pub items: Vec<PromotionQueueItem>,
    #[serde(default)]
    pub generated_at: u64,
}
