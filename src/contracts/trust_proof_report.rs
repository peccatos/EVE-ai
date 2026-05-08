use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TrustProofReport {
    #[serde(default)]
    pub generated_at: u64,
    #[serde(default)]
    pub capability_policy_status: String,
    #[serde(default)]
    pub trust_decision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_snapshot_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_bundle_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_manifest_id: Option<String>,
    #[serde(default)]
    pub preflight_gate_v3_status: String,
    #[serde(default)]
    pub next_operator_commands: Vec<String>,
}
