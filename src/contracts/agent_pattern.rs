use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentPatternSummary {
    pub generated_at: u64,
    pub patterns: Vec<AgentPattern>,
    pub warnings: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentPattern {
    pub pattern_id: String,
    pub task_kind: String,
    pub outcome: String,
    pub count: u64,
    pub evidence_task_ids: Vec<String>,
    pub reason: String,
}
