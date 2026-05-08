use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FinalRcReport {
    #[serde(default)]
    pub report_id: String,
    #[serde(default)]
    pub generated_at: u64,
    #[serde(default)]
    pub rc_status: String,
    #[serde(default)]
    pub runtime_candidate_id: String,
    #[serde(default)]
    pub runtime_validation_id: String,
    #[serde(default)]
    pub report_path: String,
}
