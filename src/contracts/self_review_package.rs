use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SelfReviewPackage {
    #[serde(default)]
    pub package_id: String,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub release_health: String,
    #[serde(default)]
    pub preflight_gate: String,
    #[serde(default)]
    pub artifact_audit: String,
    #[serde(default)]
    pub determinism_audit: String,
    #[serde(default)]
    pub governance_status: String,
    #[serde(default)]
    pub promotion_queue_status: String,
    #[serde(default)]
    pub release_status: String,
    #[serde(default)]
    pub self_modification_allowed_now: bool,
    #[serde(default)]
    pub self_modification_reason_ru: String,
    #[serde(default)]
    pub manual_review_checklist: Vec<String>,
    #[serde(default)]
    pub forbidden_actions: Vec<String>,
    #[serde(default)]
    pub recommended_next_command: String,
}
