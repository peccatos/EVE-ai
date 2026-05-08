use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EvidenceBundle {
    #[serde(default)]
    pub bundle_id: String,
    #[serde(default)]
    pub generated_at: u64,
    #[serde(default)]
    pub proof_report_summary: String,
    #[serde(default)]
    pub governance_summary: String,
    #[serde(default)]
    pub release_summary: String,
    #[serde(default)]
    pub operations_summary: String,
    #[serde(default)]
    pub artifact_audit_summary: String,
    #[serde(default)]
    pub determinism_audit_summary: String,
    #[serde(default)]
    pub preflight_summary: String,
    #[serde(default)]
    pub trust_decision_summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_release_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_bounded_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_supervised_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_proof_snapshot_id: Option<String>,
}
