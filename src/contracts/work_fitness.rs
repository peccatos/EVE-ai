use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FitnessDecision {
    Prefer,
    Acceptable,
    Risky,
    Reject,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkFitnessScore {
    pub fitness_id: String,
    pub task_id: String,
    pub proposal_id: Option<String>,
    pub validation_score: f32,
    pub approval_score: f32,
    pub usefulness_score: f32,
    pub risk_penalty: f32,
    pub patch_size_penalty: f32,
    pub failure_penalty: f32,
    pub final_score: f32,
    pub decision: FitnessDecision,
    pub explanation: Vec<String>,
}
