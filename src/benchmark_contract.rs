use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkFailureType {
    #[default]
    Unknown,
    CargoCheck,
    CargoTest,
    RuntimeFailure,
    AssertionFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkSourceType {
    GithubSearch,
    Issue,
    FailingTest,
    CompileFailure,
    CiFailure,
    LocalFixture,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustBugfixCase {
    pub case_id: String,
    pub repo_full_name: String,
    pub repo_url: String,
    pub license: String,
    pub default_branch: String,
    pub source_type: BenchmarkSourceType,
    pub source_reference: String,
    pub goal: String,
    pub local_repo_path: String,
    pub failure_type: BenchmarkFailureType,
    pub initial_failure_observed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reproduction_notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BenchmarkCaseManifest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub benchmark_mode: Option<String>,
    #[serde(default)]
    pub cases: Vec<RustBugfixCase>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepositoryDiscoveryCase {
    pub case_id: String,
    pub repo_full_name: String,
    pub repo_url: String,
    pub license: String,
    pub default_branch: String,
    pub source_type: String,
    pub source_reference: String,
    pub goal: String,
    pub local_repo_path: String,
    pub failure_type: String,
    pub initial_failure_observed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reproduction_notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo_size_kb: Option<u64>,
    #[serde(default)]
    pub has_tests_or_ci: bool,
    #[serde(default)]
    pub search_score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RepositoryDiscoveryManifest {
    #[serde(default)]
    pub cases: Vec<RepositoryDiscoveryCase>,
}
