use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PrPackage {
    #[serde(default)]
    pub package_id: String,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_release_id: Option<String>,
    #[serde(default)]
    pub release_health_summary: String,
    #[serde(default)]
    pub preflight_gate_summary: String,
    #[serde(default)]
    pub approved_candidate_ids: Vec<String>,
    #[serde(default)]
    pub release_candidate_ids: Vec<String>,
    #[serde(default)]
    pub changed_source_files: Vec<String>,
    #[serde(default)]
    pub recommended_pr_title: String,
    #[serde(default)]
    pub recommended_pr_body_ru: String,
    #[serde(default)]
    pub safety_checklist: Vec<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub metadata_only: bool,
    #[serde(default)]
    pub no_network: bool,
    #[serde(default)]
    pub no_push: bool,
    #[serde(default)]
    pub no_merge: bool,
    #[serde(default)]
    pub auto_promote: bool,
    #[serde(default)]
    pub operator_approval_required: bool,
}
