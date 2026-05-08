use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RuntimeCliCommandContract {
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub mutates_source: bool,
    #[serde(default)]
    pub mutates_external_repo: bool,
    #[serde(default)]
    pub requires_operator_approval: bool,
    #[serde(default)]
    pub safety_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RuntimeCliContractReport {
    #[serde(default)]
    pub generated_at: u64,
    #[serde(default)]
    pub commands: Vec<RuntimeCliCommandContract>,
}
