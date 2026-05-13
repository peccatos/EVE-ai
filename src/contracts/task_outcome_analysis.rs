use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskOutcomeAnalysis {
    pub generated_at: u64,
    pub total_tasks: u64,
    pub successful_tasks: u64,
    pub failed_tasks: u64,
    pub validation_pass_rate: f32,
    pub approval_rate: f32,
    pub apply_success_rate: f32,
    pub most_successful_task_types: Vec<String>,
    pub most_failed_task_types: Vec<String>,
    pub risky_paths: Vec<String>,
    pub safe_paths: Vec<String>,
    pub common_failure_reasons: Vec<String>,
    pub warnings: Vec<String>,
    pub blockers: Vec<String>,
}
