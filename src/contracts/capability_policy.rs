use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CapabilityPolicy {
    #[serde(default)]
    pub generated_at: u64,
    #[serde(default)]
    pub auto_promote_allowed: bool,
    #[serde(default)]
    pub network_push_allowed: bool,
    #[serde(default)]
    pub merge_allowed: bool,
    #[serde(default)]
    pub external_repo_mutation_allowed: bool,
    #[serde(default)]
    pub self_apply_allowed: bool,
    #[serde(default)]
    pub source_mutation_without_approval_allowed: bool,
    #[serde(default)]
    pub metadata_generation_allowed: bool,
    #[serde(default)]
    pub local_read_only_inspection_allowed: bool,
    #[serde(default)]
    pub sandboxed_validation_allowed_when_isolated: bool,
    #[serde(default)]
    pub denied_capabilities: Vec<String>,
    #[serde(default)]
    pub allowed_capabilities: Vec<String>,
}
