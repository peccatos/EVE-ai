use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskStrategyMemory {
    pub generated_at: u64,
    pub strategies: Vec<TaskStrategyPattern>,
    pub warnings: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskStrategyPattern {
    pub task_kind: String,
    pub recommended_strategy: String,
    pub success_count: u64,
    pub failure_count: u64,
    pub confidence: f32,
    pub reason: String,
}
