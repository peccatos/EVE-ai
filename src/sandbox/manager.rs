use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::sandbox::limits::DEFAULT_SANDBOX_ROOT;

pub fn create_sandbox_path() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    Path::new(DEFAULT_SANDBOX_ROOT)
        .join(format!("eva-sandbox-{now}-{}", std::process::id()))
        .to_string_lossy()
        .to_string()
}

pub fn destroy_sandbox(path: &str) -> Result<(), String> {
    let path = Path::new(path);
    if !path.exists() {
        return Ok(());
    }
    ensure_sandbox_path(path)?;
    fs::remove_dir_all(path).map_err(|error| format!("failed to destroy sandbox: {error}"))
}

fn ensure_sandbox_path(path: &Path) -> Result<(), String> {
    let normalized = normalize(path);
    if !normalized
        .components()
        .any(|component| component.as_os_str() == DEFAULT_SANDBOX_ROOT)
    {
        return Err("refusing to destroy path outside sandboxes/".to_string());
    }
    Ok(())
}

fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        normalized.push(component.as_os_str());
    }
    normalized
}
