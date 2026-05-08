use std::fs;
use std::path::Path;
use std::process::Command;

use crate::contracts::WorkspaceSnapshot;
use crate::evolution::memory;

pub fn build_workspace_snapshot(
    project_root: &str,
    memory_root: &str,
) -> Result<WorkspaceSnapshot, String> {
    let git_branch = git_stdout(project_root, &["branch", "--show-current"])
        .unwrap_or_else(|| "unknown".to_string());
    let git_head =
        git_stdout(project_root, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    let status_lines = git_stdout(project_root, &["status", "--short"])
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let tracked_count = git_stdout(project_root, &["ls-files"])
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    let mut modified_count = 0_usize;
    let mut untracked_count = 0_usize;
    for line in &status_lines {
        if line.starts_with("??") {
            untracked_count += 1;
        } else if !line.trim().is_empty() {
            modified_count += 1;
        }
    }
    let ignored_runtime_dirs_summary = [
        "memory/evidence",
        "memory/workspace_snapshots",
        "memory/recovery",
        "memory/operations",
        ".eva-evolution-tests",
        ".eva-runtime-tests",
        ".eva-operations-tests",
    ]
    .iter()
    .map(|path| format!("{path}:{}", Path::new(project_root).join(path).exists()))
    .collect::<Vec<_>>();
    let memory_artifact_counts = [
        "evidence",
        "workspace_snapshots",
        "recovery",
        "operations",
        "proof",
        "releases",
    ]
    .iter()
    .map(|dir| {
        format!(
            "{dir}:{}",
            count_files(&Path::new(memory_root).join(dir)).unwrap_or(0)
        )
    })
    .collect::<Vec<_>>();
    let sandbox_leak_count = count_immediate_entries(&Path::new(project_root).join("sandboxes"));
    let test_artifact_root_status = [
        ".eva-evolution-tests",
        ".eva-runtime-tests",
        ".eva-operations-tests",
    ]
    .iter()
    .map(|path| {
        let full = Path::new(project_root).join(path);
        format!("{path}:{}", full.exists())
    })
    .collect::<Vec<_>>();
    let generated_at = memory::now_unix();
    let snapshot = WorkspaceSnapshot {
        snapshot_id: format!("workspace-snapshot-{generated_at}"),
        generated_at,
        git_branch,
        git_head,
        tracked_count,
        untracked_count,
        modified_count,
        ignored_runtime_dirs_summary,
        memory_artifact_counts,
        sandbox_leak_count,
        test_artifact_root_status,
    };
    write_workspace_snapshot(memory_root, &snapshot)?;
    Ok(snapshot)
}

pub fn print_last_workspace_snapshot(memory_root: &str) -> Result<String, String> {
    let snapshot = latest_workspace_snapshot(memory_root)?
        .ok_or_else(|| "no workspace snapshots available".to_string())?;
    serde_json::to_string_pretty(&snapshot)
        .map_err(|error| format!("failed to serialize workspace snapshot: {error}"))
}

pub fn list_workspace_snapshots(memory_root: &str) -> Result<Vec<String>, String> {
    let mut snapshots = load_workspace_snapshots(memory_root)?;
    snapshots.sort_by(|left, right| {
        left.generated_at
            .cmp(&right.generated_at)
            .then_with(|| left.snapshot_id.cmp(&right.snapshot_id))
    });
    Ok(snapshots.into_iter().map(|item| item.snapshot_id).collect())
}

pub fn latest_workspace_snapshot_id(memory_root: &str) -> Result<Option<String>, String> {
    Ok(latest_workspace_snapshot(memory_root)?.map(|snapshot| snapshot.snapshot_id))
}

fn write_workspace_snapshot(memory_root: &str, snapshot: &WorkspaceSnapshot) -> Result<(), String> {
    let dir = Path::new(memory_root).join("workspace_snapshots");
    fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create workspace snapshot dir: {error}"))?;
    memory::write_json(dir.join(format!("{}.json", snapshot.snapshot_id)), snapshot)
}

fn load_workspace_snapshots(memory_root: &str) -> Result<Vec<WorkspaceSnapshot>, String> {
    let dir = Path::new(memory_root).join("workspace_snapshots");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut snapshots = Vec::new();
    for entry in fs::read_dir(&dir)
        .map_err(|error| format!("failed to read workspace snapshots: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("failed to read workspace snapshot entry: {error}"))?;
        let path = entry.path();
        if !path.extension().is_some_and(|ext| ext == "json") {
            continue;
        }
        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read workspace snapshot: {error}"))?;
        let snapshot: WorkspaceSnapshot = serde_json::from_str(&contents)
            .map_err(|error| format!("failed to parse workspace snapshot: {error}"))?;
        snapshots.push(snapshot);
    }
    Ok(snapshots)
}

fn latest_workspace_snapshot(memory_root: &str) -> Result<Option<WorkspaceSnapshot>, String> {
    let mut snapshots = load_workspace_snapshots(memory_root)?;
    snapshots.sort_by(|left, right| {
        left.generated_at
            .cmp(&right.generated_at)
            .then_with(|| left.snapshot_id.cmp(&right.snapshot_id))
    });
    Ok(snapshots.pop())
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
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        None
    } else {
        Some(stdout)
    }
}

fn count_files(path: &Path) -> Option<usize> {
    if !path.exists() {
        return Some(0);
    }
    if path.is_file() {
        return Some(1);
    }
    let mut count = 0_usize;
    for entry in fs::read_dir(path).ok()? {
        let entry = entry.ok()?;
        count += count_files(&entry.path())?;
    }
    Some(count)
}

fn count_immediate_entries(path: &Path) -> usize {
    if !path.exists() {
        return 0;
    }
    fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name() != ".gitkeep")
        .count()
}
