use std::fs;
use std::path::{Path, PathBuf};

use crate::contracts::{sha256_digest, ExternalPatchPackage};
use crate::evolution::memory;

pub fn build_external_patch_package(
    memory_root: &str,
    repo_path: &str,
) -> Result<ExternalPatchPackage, String> {
    let canonical = validate_external_repo_path(repo_path)?;
    let detected_cargo_project = canonical.join("Cargo.toml").exists();
    let detected_git_repo = canonical.join(".git").exists();
    let package_id = format!(
        "extpatch-{}",
        &sha256_digest(&canonical.display().to_string())[..8]
    );
    let package = ExternalPatchPackage {
        package_id: package_id.clone(),
        repo_path: canonical.display().to_string(),
        created_at: memory::now_unix(),
        detected_cargo_project,
        detected_git_repo,
        suggested_validation_commands: suggested_validation_commands(detected_cargo_project),
        safe_patch_strategy_ru: "Подготовить patch metadata локально, затем вручную проверить diff и validation в целевом репозитории без push/merge.".to_string(),
        risk_notes: vec![
            "Внешний репозиторий не мутируется.".to_string(),
            "Сетевые операции запрещены.".to_string(),
            "Patch package остаётся metadata-only.".to_string(),
        ],
        allowed_next_steps: vec![
            "Локально открыть репозиторий и вручную проверить нужные цели.".to_string(),
            "Запустить suggested_validation_commands в целевом репозитории вручную.".to_string(),
        ],
        forbidden_next_steps: vec![
            "git push".to_string(),
            "git merge".to_string(),
            "network clone".to_string(),
            "автоматическое изменение внешнего исходника".to_string(),
        ],
        metadata_only: true,
        source_mutated: false,
        auto_promote: false,
    };
    write_external_patch_package(memory_root, &package)?;
    Ok(package)
}

pub fn print_last_external_patch_package(memory_root: &str) -> Result<String, String> {
    let package = latest_external_patch_package(memory_root)?
        .ok_or_else(|| "no external patch packages available".to_string())?;
    fs::read_to_string(external_patch_markdown_path(
        memory_root,
        &package.package_id,
    ))
    .map_err(|error| format!("failed to read external patch package markdown: {error}"))
}

pub fn list_external_patch_packages(memory_root: &str) -> Result<Vec<String>, String> {
    let mut packages = load_external_patch_packages(memory_root)?;
    packages.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.package_id.cmp(&right.package_id))
    });
    Ok(packages.into_iter().map(|item| item.package_id).collect())
}

fn validate_external_repo_path(repo_path: &str) -> Result<PathBuf, String> {
    if repo_path.starts_with("http://")
        || repo_path.starts_with("https://")
        || repo_path.starts_with("git@")
    {
        return Err("network urls are not allowed".to_string());
    }
    let path = Path::new(repo_path);
    if !path.exists() {
        return Err("external repo path does not exist".to_string());
    }
    if !path.is_dir() {
        return Err("external repo path must be a directory".to_string());
    }
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("failed to canonicalize external repo path: {error}"))?;
    Ok(canonical)
}

fn suggested_validation_commands(detected_cargo_project: bool) -> Vec<String> {
    if detected_cargo_project {
        vec![
            "cargo fmt --check".to_string(),
            "cargo check".to_string(),
            "cargo test".to_string(),
            "git status --short".to_string(),
        ]
    } else {
        vec![
            "git status --short".to_string(),
            "find . -maxdepth 2 -type f".to_string(),
        ]
    }
}

fn write_external_patch_package(
    memory_root: &str,
    package: &ExternalPatchPackage,
) -> Result<(), String> {
    let dir = Path::new(memory_root)
        .join("operations")
        .join("external_patches");
    fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create external patch dir: {error}"))?;
    memory::write_json(dir.join(format!("{}.json", package.package_id)), package)?;
    fs::write(
        dir.join(format!("{}.ru.md", package.package_id)),
        render_external_patch_markdown(package),
    )
    .map_err(|error| format!("failed to write external patch markdown: {error}"))
}

fn render_external_patch_markdown(package: &ExternalPatchPackage) -> String {
    format!(
        "# EVA External Patch Package\n\npackage_id={}\nrepo_path={}\ndetected_cargo_project={}\ndetected_git_repo={}\nmetadata_only={}\nsource_mutated={}\nauto_promote={}\n\nsafe_patch_strategy_ru={}\n\nallowed_next_steps:\n{}\n\nforbidden_next_steps:\n{}\n",
        package.package_id,
        package.repo_path,
        package.detected_cargo_project,
        package.detected_git_repo,
        package.metadata_only,
        package.source_mutated,
        package.auto_promote,
        package.safe_patch_strategy_ru,
        package
            .allowed_next_steps
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n"),
        package
            .forbidden_next_steps
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn external_patch_markdown_path(memory_root: &str, package_id: &str) -> PathBuf {
    Path::new(memory_root)
        .join("operations")
        .join("external_patches")
        .join(format!("{package_id}.ru.md"))
}

fn latest_external_patch_package(
    memory_root: &str,
) -> Result<Option<ExternalPatchPackage>, String> {
    let mut packages = load_external_patch_packages(memory_root)?;
    packages.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.package_id.cmp(&right.package_id))
    });
    Ok(packages.pop())
}

fn load_external_patch_packages(memory_root: &str) -> Result<Vec<ExternalPatchPackage>, String> {
    let dir = Path::new(memory_root)
        .join("operations")
        .join("external_patches");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut packages = Vec::new();
    for entry in fs::read_dir(&dir)
        .map_err(|error| format!("failed to read external patch packages: {error}"))?
    {
        let entry = entry
            .map_err(|error| format!("failed to read external patch package entry: {error}"))?;
        let path = entry.path();
        if !path.extension().is_some_and(|ext| ext == "json") {
            continue;
        }
        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read external patch package: {error}"))?;
        let package: ExternalPatchPackage = serde_json::from_str(&contents)
            .map_err(|error| format!("failed to parse external patch package: {error}"))?;
        packages.push(package);
    }
    Ok(packages)
}
