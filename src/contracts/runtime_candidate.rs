use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RuntimeCandidateManifest {
    #[serde(default)]
    pub candidate_id: String,
    #[serde(default)]
    pub generated_at: u64,
    #[serde(default)]
    pub git_branch: String,
    #[serde(default)]
    pub git_head: String,
    #[serde(default)]
    pub completed_phases: Vec<String>,
    #[serde(default)]
    pub planned_phases: Vec<String>,
    #[serde(default)]
    pub support_flags: Vec<String>,
    #[serde(default)]
    pub governance_state: String,
    #[serde(default)]
    pub release_state: String,
    #[serde(default)]
    pub trust_state: String,
    #[serde(default)]
    pub operations_state: String,
    #[serde(default)]
    pub preflight_gate_v3_state: String,
    #[serde(default)]
    pub auto_promote: bool,
    #[serde(default)]
    pub operator_approval_required: bool,
    #[serde(default)]
    pub sandbox_leak_count: usize,
    #[serde(default)]
    pub release_count: usize,
    #[serde(default)]
    pub ready_candidates: usize,
    #[serde(default)]
    pub approved_count: usize,
    #[serde(default)]
    pub blocked_candidates_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_evidence_bundle_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_workspace_snapshot_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_recovery_manifest_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_proof_snapshot_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_bounded_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_supervised_run_id: Option<String>,
    #[serde(default)]
    pub rc_status: String,
}
