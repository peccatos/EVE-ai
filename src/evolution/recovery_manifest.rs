use std::fs;
use std::path::Path;
use std::process::Command;

use crate::contracts::RecoveryManifest;
use crate::evolution::{
    latest_evidence_bundle_id, latest_release_id, latest_workspace_snapshot_id, memory,
};

pub fn build_recovery_manifest(
    project_root: &str,
    memory_root: &str,
) -> Result<RecoveryManifest, String> {
    let generated_at = memory::now_unix();
    let manifest = RecoveryManifest {
        manifest_id: format!("recovery-{generated_at}"),
        generated_at,
        current_branch: git_stdout(project_root, &["branch", "--show-current"])
            .unwrap_or_else(|| "unknown".to_string()),
        current_head: git_stdout(project_root, &["rev-parse", "HEAD"])
            .unwrap_or_else(|| "unknown".to_string()),
        latest_release_id: latest_release_id(memory_root)?,
        latest_release_manifest_path: latest_path(memory_root, "releases/manifests", "json")?,
        latest_rollback_manifest_path: latest_path(memory_root, "releases/rollback", "json")?,
        latest_evidence_bundle_id: latest_evidence_bundle_id(memory_root)?,
        latest_workspace_snapshot_id: latest_workspace_snapshot_id(memory_root)?,
        recovery_steps: vec![
            "git status --short".to_string(),
            "cargo run -- --workspace-snapshot".to_string(),
            "cargo run -- --evidence-bundle".to_string(),
            "cargo run -- --preflight-gate-v3".to_string(),
            "Проверить release/recovery manifests вручную перед любыми изменениями.".to_string(),
        ],
        prohibited_automatic_actions: vec![
            "push".to_string(),
            "merge".to_string(),
            "auto-promote".to_string(),
            "self-apply".to_string(),
        ],
    };
    write_recovery_manifest(memory_root, &manifest)?;
    Ok(manifest)
}

pub fn print_last_recovery_manifest(memory_root: &str) -> Result<String, String> {
    let manifest = latest_recovery_manifest(memory_root)?
        .ok_or_else(|| "no recovery manifests available".to_string())?;
    serde_json::to_string_pretty(&manifest)
        .map_err(|error| format!("failed to serialize recovery manifest: {error}"))
}

pub fn list_recovery_manifests(memory_root: &str) -> Result<Vec<String>, String> {
    let mut manifests = load_recovery_manifests(memory_root)?;
    manifests.sort_by(|left, right| {
        left.generated_at
            .cmp(&right.generated_at)
            .then_with(|| left.manifest_id.cmp(&right.manifest_id))
    });
    Ok(manifests.into_iter().map(|item| item.manifest_id).collect())
}

pub fn latest_recovery_manifest_id(memory_root: &str) -> Result<Option<String>, String> {
    Ok(latest_recovery_manifest(memory_root)?.map(|manifest| manifest.manifest_id))
}

fn write_recovery_manifest(memory_root: &str, manifest: &RecoveryManifest) -> Result<(), String> {
    let dir = Path::new(memory_root).join("recovery");
    fs::create_dir_all(&dir).map_err(|error| format!("failed to create recovery dir: {error}"))?;
    memory::write_json(dir.join(format!("{}.json", manifest.manifest_id)), manifest)
}

fn load_recovery_manifests(memory_root: &str) -> Result<Vec<RecoveryManifest>, String> {
    let dir = Path::new(memory_root).join("recovery");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut manifests = Vec::new();
    for entry in
        fs::read_dir(&dir).map_err(|error| format!("failed to read recovery dir: {error}"))?
    {
        let entry = entry.map_err(|error| format!("failed to read recovery entry: {error}"))?;
        let path = entry.path();
        if !path.extension().is_some_and(|ext| ext == "json") {
            continue;
        }
        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read recovery manifest: {error}"))?;
        let manifest: RecoveryManifest = serde_json::from_str(&contents)
            .map_err(|error| format!("failed to parse recovery manifest: {error}"))?;
        manifests.push(manifest);
    }
    Ok(manifests)
}

fn latest_recovery_manifest(memory_root: &str) -> Result<Option<RecoveryManifest>, String> {
    let mut manifests = load_recovery_manifests(memory_root)?;
    manifests.sort_by(|left, right| {
        left.generated_at
            .cmp(&right.generated_at)
            .then_with(|| left.manifest_id.cmp(&right.manifest_id))
    });
    Ok(manifests.pop())
}

fn latest_path(
    memory_root: &str,
    relative_dir: &str,
    extension: &str,
) -> Result<Option<String>, String> {
    let dir = Path::new(memory_root).join(relative_dir);
    if !dir.exists() {
        return Ok(None);
    }
    let mut paths = fs::read_dir(dir)
        .map_err(|error| format!("failed to read recovery dependency dir: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == extension))
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths.last().map(|path| path.display().to_string()))
}

fn git_stdout(project_root: &str, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(project_root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
