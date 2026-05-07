use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SupervisedRun {
    #[serde(default)]
    pub supervised_run_id: String,
    #[serde(default)]
    pub initial_task_path: String,
    #[serde(default)]
    pub current_task_path: String,
    #[serde(default)]
    pub started_at: u64,
    #[serde(default)]
    pub finished_at: u64,
    #[serde(default)]
    pub max_rounds: usize,
    #[serde(default)]
    pub executed_rounds: usize,
    #[serde(default)]
    pub bounded_run_ids: Vec<String>,
    #[serde(default)]
    pub campaign_ids: Vec<String>,
    #[serde(default)]
    pub adjusted_task_paths: Vec<String>,
    #[serde(default)]
    pub ready_candidate_run_ids: Vec<String>,
    #[serde(default)]
    pub rejected_candidate_run_ids: Vec<String>,
    #[serde(default)]
    pub zero_yield_rounds: usize,
    #[serde(default)]
    pub replay_failed_rounds: usize,
    #[serde(default)]
    pub stop_reason: String,
    #[serde(default)]
    pub final_status: String,
    #[serde(default)]
    pub auto_promote: bool,
}
