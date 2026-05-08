use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ExternalPatchPackage {
    #[serde(default)]
    pub package_id: String,
    #[serde(default)]
    pub repo_path: String,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub detected_cargo_project: bool,
    #[serde(default)]
    pub detected_git_repo: bool,
    #[serde(default)]
    pub suggested_validation_commands: Vec<String>,
    #[serde(default)]
    pub safe_patch_strategy_ru: String,
    #[serde(default)]
    pub risk_notes: Vec<String>,
    #[serde(default)]
    pub allowed_next_steps: Vec<String>,
    #[serde(default)]
    pub forbidden_next_steps: Vec<String>,
    #[serde(default)]
    pub metadata_only: bool,
    #[serde(default)]
    pub source_mutated: bool,
    #[serde(default)]
    pub auto_promote: bool,
}
