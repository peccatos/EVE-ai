use std::fs;
use std::path::Path;

use crate::agent::storage::{memory_path, now_unix, save_json_pretty};
use crate::contracts::{RepoMap, RepoModule};

pub fn build_repo_map(project_root: &str, memory_root: &str) -> Result<RepoMap, String> {
    let root = Path::new(project_root);
    let mut map = RepoMap {
        generated_at: now_unix(),
        cargo_project: root.join("Cargo.toml").exists(),
        package_name: read_package_name(&root.join("Cargo.toml")),
        entrypoints: existing(root, &["src/main.rs", "src/lib.rs"]),
        modules: Vec::new(),
        tests: Vec::new(),
        docs: Vec::new(),
        cli_routes: Vec::new(),
        contracts: Vec::new(),
        risk_zones: vec!["src/".to_string(), "Cargo.toml".to_string()],
        warnings: Vec::new(),
    };
    collect_files(root, root, &mut map)?;
    map.modules.sort_by(|a, b| a.path.cmp(&b.path));
    map.tests.sort();
    map.docs.sort();
    map.cli_routes.sort();
    map.cli_routes.dedup();
    map.contracts.sort();
    map.contracts.dedup();
    save_json_pretty(
        &memory_path(memory_root, &["repo_map", "latest_repo_map.json"]),
        &map,
    )?;
    Ok(map)
}

pub fn print_repo_map(project_root: &str, memory_root: &str) -> Result<String, String> {
    let map = build_repo_map(project_root, memory_root)?;
    Ok(format!(
        "EVA Repo Map\ncargo_project={}\npackage={}\nentrypoints={}\nmodules={}\ntests={}\ndocs={}\ncli_routes={}\ncontracts={}\nrisk_zones={}",
        map.cargo_project,
        map.package_name.as_deref().unwrap_or("unknown"),
        map.entrypoints.join(","),
        map.modules.len(),
        map.tests.len(),
        map.docs.len(),
        map.cli_routes.join(","),
        map.contracts.join(","),
        map.risk_zones.join(",")
    ))
}

fn existing(root: &Path, paths: &[&str]) -> Vec<String> {
    paths
        .iter()
        .filter(|path| root.join(path).exists())
        .map(|path| (*path).to_string())
        .collect()
}

fn read_package_name(path: &Path) -> Option<String> {
    let contents = fs::read_to_string(path).ok()?;
    for line in contents.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("name") {
            if let Some(value) = rest.split('=').nth(1) {
                return Some(value.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

fn collect_files(root: &Path, dir: &Path, map: &mut RepoMap) -> Result<(), String> {
    if ignored(root, dir) {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|error| format!("read {}: {error}", dir.display()))? {
        let entry = entry.map_err(|error| format!("read dir entry: {error}"))?;
        let path = entry.path();
        if ignored(root, &path) {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, map)?;
            continue;
        }
        let rel = rel(root, &path);
        if rel.ends_with(".rs") && rel.starts_with("src/") {
            let contents = fs::read_to_string(&path).unwrap_or_default();
            if rel == "src/main.rs" {
                map.cli_routes.extend(extract_cli_routes(&contents));
            }
            if rel.starts_with("src/contracts/") {
                map.contracts.push(rel.clone());
            }
            map.modules.push(RepoModule {
                path: rel,
                kind: "rust".to_string(),
                public_items: extract_public_items(&contents),
            });
        } else if rel.ends_with(".rs") && rel.starts_with("tests/") {
            map.tests.push(rel);
        } else if rel.ends_with(".md") && (rel.starts_with("docs/") || rel == "README.md") {
            map.docs.push(rel);
        }
    }
    Ok(())
}

fn ignored(root: &Path, path: &Path) -> bool {
    let rel = rel(root, path);
    [
        ".git",
        "target",
        "memory",
        "releases",
        "sandboxes",
        ".eva-runtime-tests",
        ".eva-evolution-tests",
    ]
    .iter()
    .any(|prefix| rel == *prefix || rel.starts_with(&format!("{prefix}/")))
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn extract_public_items(contents: &str) -> Vec<String> {
    contents
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            for prefix in [
                "pub fn ",
                "pub struct ",
                "pub enum ",
                "pub trait ",
                "pub mod ",
            ] {
                if let Some(rest) = trimmed.strip_prefix(prefix) {
                    let name = rest
                        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
                        .next()
                        .unwrap_or("")
                        .to_string();
                    if !name.is_empty() {
                        return Some(name);
                    }
                }
            }
            None
        })
        .collect()
}

fn extract_cli_routes(contents: &str) -> Vec<String> {
    contents
        .lines()
        .filter(|line| line.contains("\"--") || line.contains("\"agent") || line.contains("\"task"))
        .filter_map(|line| {
            let start = line.find('"')?;
            let rest = &line[start + 1..];
            let end = rest.find('"')?;
            let value = &rest[..end];
            if value.starts_with("--")
                || value
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
            {
                Some(value.to_string())
            } else {
                None
            }
        })
        .collect()
}
