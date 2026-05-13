use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepoMap {
    pub generated_at: u64,
    pub cargo_project: bool,
    pub package_name: Option<String>,
    pub entrypoints: Vec<String>,
    pub modules: Vec<RepoModule>,
    pub tests: Vec<String>,
    pub docs: Vec<String>,
    pub cli_routes: Vec<String>,
    pub contracts: Vec<String>,
    pub risk_zones: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepoModule {
    pub path: String,
    pub kind: String,
    pub public_items: Vec<String>,
}
