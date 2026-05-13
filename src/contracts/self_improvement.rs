use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SelfImprovementProposal {
    pub self_improvement_id: String,
    pub source_outcomes: Vec<String>,
    pub weakness: String,
    pub proposed_goal: String,
    pub recommended_strategy: String,
    pub allowed_scope: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub risk_level: String,
    pub approval_required: bool,
    pub proposal_id: Option<String>,
    pub warnings: Vec<String>,
    pub blockers: Vec<String>,
}
