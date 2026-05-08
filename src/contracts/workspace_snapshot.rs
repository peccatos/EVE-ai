use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkspaceSnapshot {
    #[serde(default)]
    pub snapshot_id: String,
    #[serde(default)]
    pub generated_at: u64,
    #[serde(default)]
    pub git_branch: String,
    #[serde(default)]
    pub git_head: String,
    #[serde(default)]
    pub tracked_count: usize,
    #[serde(default)]
    pub untracked_count: usize,
    #[serde(default)]
    pub modified_count: usize,
    #[serde(default)]
    pub ignored_runtime_dirs_summary: Vec<String>,
    #[serde(default)]
    pub memory_artifact_counts: Vec<String>,
    #[serde(default)]
    pub sandbox_leak_count: usize,
    #[serde(default)]
    pub test_artifact_root_status: Vec<String>,
}
