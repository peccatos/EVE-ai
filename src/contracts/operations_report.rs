use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OperationsReport {
    #[serde(default)]
    pub generated_at: u64,
    #[serde(default)]
    pub release_health_grade: String,
    #[serde(default)]
    pub release_health_score: u32,
    #[serde(default)]
    pub preflight_gate_status: String,
    #[serde(default)]
    pub artifact_audit_status: String,
    #[serde(default)]
    pub determinism_audit_status: String,
    #[serde(default)]
    pub governance_approved_count: usize,
    #[serde(default)]
    pub governance_rejected_count: usize,
    #[serde(default)]
    pub governance_deferred_count: usize,
    #[serde(default)]
    pub promotion_ready_count: usize,
    #[serde(default)]
    pub promotion_blocked_count: usize,
    #[serde(default)]
    pub release_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_release_id: Option<String>,
    #[serde(default)]
    pub proof_support_capabilities: Vec<String>,
    #[serde(default)]
    pub auto_promote: bool,
    #[serde(default)]
    pub operator_approval_required: bool,
    #[serde(default)]
    pub future_phases_allowed_now: bool,
    #[serde(default)]
    pub next_safe_operator_action: String,
}
