use serde::{Deserialize, Serialize};

use crate::contracts::OperatorApprovalRecord;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GovernanceTrustGate {
    #[serde(default)]
    pub allowed: bool,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub reason_ru: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GovernanceStatus {
    #[serde(default)]
    pub approved_count: usize,
    #[serde(default)]
    pub rejected_count: usize,
    #[serde(default)]
    pub deferred_count: usize,
    #[serde(default)]
    pub promotion_ready_approved_count: usize,
    #[serde(default)]
    pub auto_promote: bool,
    #[serde(default)]
    pub operator_approval_required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ApprovalStatus {
    #[serde(default)]
    pub run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_record: Option<OperatorApprovalRecord>,
    #[serde(default)]
    pub current_decision: String,
    #[serde(default)]
    pub promotable: bool,
}
