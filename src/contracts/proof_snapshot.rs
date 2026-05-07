use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProofSnapshot {
    #[serde(default)]
    pub snapshot_id: String,
    #[serde(default)]
    pub total_runs: u64,
    #[serde(default)]
    pub candidate_count: usize,
    #[serde(default)]
    pub replay_passed: usize,
    #[serde(default)]
    pub promoted_count: usize,
    #[serde(default)]
    pub promotion_queue_ready: usize,
    #[serde(default)]
    pub promotion_queue_blocked: usize,
    #[serde(default)]
    pub approved_count: usize,
    #[serde(default)]
    pub rejected_count: usize,
    #[serde(default)]
    pub deferred_count: usize,
    #[serde(default)]
    pub release_proposal_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_bounded_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_supervised_run_id: Option<String>,
    #[serde(default)]
    pub auto_promote: bool,
    #[serde(default)]
    pub operator_approval_required: bool,
    #[serde(default)]
    pub created_at: u64,
}
