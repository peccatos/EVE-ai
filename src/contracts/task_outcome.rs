use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskOutcome {
    pub outcome_id: String,
    pub task_id: String,
    pub goal: String,
    pub planner: String,
    pub proposer: String,
    pub llm_used: bool,
    pub proposal_id: Option<String>,
    pub approval_id: Option<String>,
    pub apply_id: Option<String>,
    pub validation_id: Option<String>,
    pub report_id: Option<String>,
    pub pr_summary_id: Option<String>,
    pub files_changed: Vec<String>,
    pub patch_ops_count: usize,
    pub risk_level: String,
    pub approved: bool,
    pub applied: bool,
    pub validation_status: String,
    pub success: bool,
    pub failure_reason: Option<String>,
    pub lessons: Vec<String>,
    pub warnings: Vec<String>,
    pub blockers: Vec<String>,
    pub created_at: u64,
}
