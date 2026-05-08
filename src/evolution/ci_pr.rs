use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::contracts::{sha256_digest, PrPackage};
use crate::evolution::{
    build_preflight_gate, build_release_health, latest_release_id, memory,
    promotion_ready_approved, promotion_ready_items,
};

const SAFETY_CHECKLIST: &[&str] = &[
    "cargo fmt --check",
    "cargo check",
    "cargo test --lib",
    "cargo test",
    "find sandboxes -mindepth 1 -maxdepth 2 -type d -print",
    "git status --short",
];

pub fn build_pr_package(project_root: &str, memory_root: &str) -> Result<PrPackage, String> {
    let health = build_release_health(project_root, memory_root)?;
    let gate = build_preflight_gate(project_root, memory_root)?;
    let approved_items = promotion_ready_approved(project_root, memory_root)?;
    let release_candidates = promotion_ready_items(project_root, memory_root)?;
    let approved_candidate_ids = approved_items
        .iter()
        .map(|item| item.run_id.clone())
        .collect::<Vec<_>>();
    let release_candidate_ids = release_candidates
        .iter()
        .map(|item| item.run_id.clone())
        .collect::<Vec<_>>();
    let changed_source_files = approved_items
        .iter()
        .chain(release_candidates.iter())
        .map(|item| item.target_file.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let source_branch = current_branch(project_root);
    let latest_release_id = latest_release_id(memory_root)?;
    let status = if approved_candidate_ids.is_empty() {
        "draft_no_release_candidate".to_string()
    } else {
        "ready_for_export".to_string()
    };
    let seed = format!(
        "{}|{}|{}|{}|{}",
        source_branch.as_deref().unwrap_or("detached"),
        latest_release_id.as_deref().unwrap_or("none"),
        approved_candidate_ids.join(","),
        release_candidate_ids.join(","),
        status
    );
    let package_id = format!("prpkg-{}", &sha256_digest(&seed)[..8]);
    let created_at = memory::now_unix();
    let package = PrPackage {
        package_id: package_id.clone(),
        created_at,
        source_branch,
        latest_release_id,
        release_health_summary: format!(
            "grade={} score={} ready={} blocked={}",
            health.health_grade, health.health_score, health.ready_count, health.blocked_count
        ),
        preflight_gate_summary: format!(
            "status={} blockers={} warnings={}",
            gate.gate_status,
            if gate.blockers.is_empty() {
                "none".to_string()
            } else {
                gate.blockers.join(",")
            },
            if gate.warnings.is_empty() {
                "none".to_string()
            } else {
                gate.warnings.join(",")
            }
        ),
        approved_candidate_ids,
        release_candidate_ids,
        changed_source_files,
        recommended_pr_title: recommended_pr_title(&status, created_at),
        recommended_pr_body_ru: recommended_pr_body_ru(&health, &gate),
        safety_checklist: SAFETY_CHECKLIST
            .iter()
            .map(|item| item.to_string())
            .collect(),
        status,
        metadata_only: true,
        no_network: true,
        no_push: true,
        no_merge: true,
        auto_promote: false,
        operator_approval_required: true,
    };
    write_pr_package(memory_root, &package)?;
    Ok(package)
}

pub fn print_last_pr_package(memory_root: &str) -> Result<String, String> {
    let package =
        latest_pr_package(memory_root)?.ok_or_else(|| "no pr packages available".to_string())?;
    fs::read_to_string(pr_package_markdown_path(memory_root, &package.package_id))
        .map_err(|error| format!("failed to read pr package markdown: {error}"))
}

pub fn list_pr_packages(memory_root: &str) -> Result<Vec<String>, String> {
    let mut packages = load_pr_packages(memory_root)?;
    packages.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.package_id.cmp(&right.package_id))
    });
    Ok(packages.into_iter().map(|item| item.package_id).collect())
}

fn recommended_pr_title(status: &str, created_at: u64) -> String {
    if status == "draft_no_release_candidate" {
        format!("draft: EVA operations package {created_at}")
    } else {
        format!("EVA operations package {created_at}")
    }
}

fn recommended_pr_body_ru(
    health: &crate::contracts::ReleaseHealthReport,
    gate: &crate::contracts::PreflightGateReport,
) -> String {
    format!(
        "## EVA Operations Package\n\n- metadata_only=true\n- auto_promote=false\n- operator_approval_required=true\n- release_health={} score={}\n- preflight_gate={}\n\nBlockers: {}\nWarnings: {}\n",
        health.health_grade,
        health.health_score,
        gate.gate_status,
        if gate.blockers.is_empty() {
            "none".to_string()
        } else {
            gate.blockers.join(", ")
        },
        if gate.warnings.is_empty() {
            "none".to_string()
        } else {
            gate.warnings.join(", ")
        }
    )
}

fn current_branch(project_root: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(project_root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        None
    } else {
        Some(branch)
    }
}

fn write_pr_package(memory_root: &str, package: &PrPackage) -> Result<(), String> {
    let dir = Path::new(memory_root)
        .join("operations")
        .join("pr_packages");
    fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create pr package dir: {error}"))?;
    memory::write_json(dir.join(format!("{}.json", package.package_id)), package)?;
    fs::write(
        dir.join(format!("{}.ru.md", package.package_id)),
        render_pr_package_markdown(package),
    )
    .map_err(|error| format!("failed to write pr package markdown: {error}"))
}

fn render_pr_package_markdown(package: &PrPackage) -> String {
    format!(
        "# EVA PR Package\n\npackage_id={}\nstatus={}\nsource_branch={}\nlatest_release_id={}\nrelease_health={}\npreflight_gate={}\napproved_candidates={}\nrelease_candidates={}\nchanged_source_files={}\nrecommended_title={}\nmetadata_only={}\nno_network={}\nno_push={}\nno_merge={}\nauto_promote={}\noperator_approval_required={}\n\n## Safety checklist\n{}\n\n## Recommended PR body\n{}\n",
        package.package_id,
        package.status,
        package.source_branch.as_deref().unwrap_or("unknown"),
        package.latest_release_id.as_deref().unwrap_or("none"),
        package.release_health_summary,
        package.preflight_gate_summary,
        if package.approved_candidate_ids.is_empty() {
            "none".to_string()
        } else {
            package.approved_candidate_ids.join(", ")
        },
        if package.release_candidate_ids.is_empty() {
            "none".to_string()
        } else {
            package.release_candidate_ids.join(", ")
        },
        if package.changed_source_files.is_empty() {
            "none".to_string()
        } else {
            package.changed_source_files.join(", ")
        },
        package.recommended_pr_title,
        package.metadata_only,
        package.no_network,
        package.no_push,
        package.no_merge,
        package.auto_promote,
        package.operator_approval_required,
        package
            .safety_checklist
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n"),
        package.recommended_pr_body_ru
    )
}

fn pr_package_markdown_path(memory_root: &str, package_id: &str) -> std::path::PathBuf {
    Path::new(memory_root)
        .join("operations")
        .join("pr_packages")
        .join(format!("{package_id}.ru.md"))
}

fn latest_pr_package(memory_root: &str) -> Result<Option<PrPackage>, String> {
    let mut packages = load_pr_packages(memory_root)?;
    packages.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.package_id.cmp(&right.package_id))
    });
    Ok(packages.pop())
}

fn load_pr_packages(memory_root: &str) -> Result<Vec<PrPackage>, String> {
    let dir = Path::new(memory_root)
        .join("operations")
        .join("pr_packages");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut packages = Vec::new();
    for entry in
        fs::read_dir(&dir).map_err(|error| format!("failed to read pr packages: {error}"))?
    {
        let entry = entry.map_err(|error| format!("failed to read pr package entry: {error}"))?;
        let path = entry.path();
        if !path.extension().is_some_and(|ext| ext == "json") {
            continue;
        }
        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read pr package: {error}"))?;
        let package: PrPackage = serde_json::from_str(&contents)
            .map_err(|error| format!("failed to parse pr package: {error}"))?;
        packages.push(package);
    }
    Ok(packages)
}
