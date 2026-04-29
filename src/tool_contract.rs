use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolRequest {
    WriteFile { path: PathBuf, contents: String },
    RemoveFile { path: PathBuf },
    CargoCheck { workdir: PathBuf },
    CargoTest { workdir: PathBuf, args: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandOutput {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolResponse {
    Command(CommandOutput),
    Write { bytes_written: u64 },
    Remove { existed: bool },
}
