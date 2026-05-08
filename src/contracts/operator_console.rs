use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OperatorConsoleReport {
    #[serde(default)]
    pub generated_at: u64,
    #[serde(default)]
    pub status_lines: Vec<String>,
    #[serde(default)]
    pub next_commands: Vec<String>,
}
