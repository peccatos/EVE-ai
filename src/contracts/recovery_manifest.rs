use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RecoveryManifest {
    #[serde(default)]
    pub manifest_id: String,
    #[serde(default)]
    pub generated_at: u64,
    #[serde(default)]
    pub current_branch: String,
    #[serde(default)]
    pub current_head: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_release_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_release_manifest_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_rollback_manifest_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_evidence_bundle_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_workspace_snapshot_id: Option<String>,
    #[serde(default)]
    pub recovery_steps: Vec<String>,
    #[serde(default)]
    pub prohibited_automatic_actions: Vec<String>,
}
