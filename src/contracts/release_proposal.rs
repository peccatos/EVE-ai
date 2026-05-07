use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ReleaseProposalItem {
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub mutation_kind: String,
    #[serde(default)]
    pub target_file: String,
    #[serde(default)]
    pub score: f32,
    #[serde(default)]
    pub risk: f32,
    #[serde(default)]
    pub approval_reason: String,
    #[serde(default)]
    pub report_path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ReleaseProposal {
    #[serde(default)]
    pub proposal_id: String,
    #[serde(default)]
    pub items: Vec<ReleaseProposalItem>,
    #[serde(default)]
    pub auto_promote: bool,
    #[serde(default)]
    pub forbidden_targets_preserved: bool,
    #[serde(default)]
    pub rejected_count: usize,
    #[serde(default)]
    pub deferred_count: usize,
    #[serde(default)]
    pub ready_approved_count: usize,
    #[serde(default)]
    pub created_at: u64,
}
