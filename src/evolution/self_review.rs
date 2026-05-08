use std::fs;
use std::path::{Path, PathBuf};

use crate::contracts::SelfReviewPackage;
use crate::evolution::{
    build_artifact_audit, build_determinism_audit, build_preflight_gate, build_release_health,
    governance_status, memory, print_release_status, refresh_promotion_queue,
};

pub fn build_self_review_package(
    project_root: &str,
    memory_root: &str,
) -> Result<SelfReviewPackage, String> {
    let health = build_release_health(project_root, memory_root)?;
    let gate = build_preflight_gate(project_root, memory_root)?;
    let artifact = build_artifact_audit(project_root)?;
    let determinism = build_determinism_audit(project_root, memory_root)?;
    let governance = governance_status(project_root, memory_root)?;
    let queue = refresh_promotion_queue(project_root, memory_root)?;
    let promotion_ready_count = queue
        .items
        .iter()
        .filter(|item| item.lifecycle_state == "ready")
        .count();
    let self_modification_allowed_now = gate.gate_status == "pass"
        && governance.promotion_ready_approved_count > 0
        && !artifact.should_fail_release
        && determinism.deterministic_enough
        && health.health_grade == "green";
    let recommended_next_command = if self_modification_allowed_now {
        "cargo run -- --pr-package".to_string()
    } else if gate.gate_status == "warn" {
        "cargo run -- --promotion-ready-approved".to_string()
    } else {
        "cargo run -- --ops-status".to_string()
    };
    let package = SelfReviewPackage {
        package_id: format!("selfreview-{}", memory::now_unix()),
        created_at: memory::now_unix(),
        release_health: format!(
            "grade={} score={}",
            health.health_grade, health.health_score
        ),
        preflight_gate: format!("status={}", gate.gate_status),
        artifact_audit: format!(
            "status={}",
            if artifact.should_fail_release {
                "fail"
            } else {
                "pass"
            }
        ),
        determinism_audit: format!(
            "status={}",
            if determinism.deterministic_enough {
                "pass"
            } else {
                "fail"
            }
        ),
        governance_status: format!(
            "approved={} rejected={} deferred={} ready_approved={}",
            governance.approved_count,
            governance.rejected_count,
            governance.deferred_count,
            governance.promotion_ready_approved_count
        ),
        promotion_queue_status: format!(
            "ready={} blocked={}",
            promotion_ready_count,
            queue.items.len().saturating_sub(promotion_ready_count)
        ),
        release_status: print_release_status(memory_root)?,
        self_modification_allowed_now,
        self_modification_reason_ru: if self_modification_allowed_now {
            "Текущее состояние достаточно стабильно для ручного self-review пакета, но не для self-apply.".to_string()
        } else {
            "Режим консервативный: без approved release-кандидата и pass preflight self-modification не разрешается.".to_string()
        },
        manual_review_checklist: vec![
            "Проверить governance approval и replay_status=ok.".to_string(),
            "Проверить отсутствие sandbox leaks и tracked runtime artifacts.".to_string(),
            "Проверить preflight gate и release health перед любым ручным действием.".to_string(),
        ],
        forbidden_actions: vec![
            "auto_promote".to_string(),
            "self_apply".to_string(),
            "network".to_string(),
            "push".to_string(),
            "merge".to_string(),
            "rewrite_core_without_approval".to_string(),
            "delete_runtime_artifacts".to_string(),
        ],
        recommended_next_command,
    };
    write_self_review_package(memory_root, &package)?;
    Ok(package)
}

pub fn print_last_self_review_package(memory_root: &str) -> Result<String, String> {
    let package = latest_self_review_package(memory_root)?
        .ok_or_else(|| "no self review packages available".to_string())?;
    fs::read_to_string(self_review_markdown_path(memory_root, &package.package_id))
        .map_err(|error| format!("failed to read self review markdown: {error}"))
}

pub fn list_self_review_packages(memory_root: &str) -> Result<Vec<String>, String> {
    let mut packages = load_self_review_packages(memory_root)?;
    packages.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.package_id.cmp(&right.package_id))
    });
    Ok(packages.into_iter().map(|item| item.package_id).collect())
}

fn write_self_review_package(memory_root: &str, package: &SelfReviewPackage) -> Result<(), String> {
    let dir = Path::new(memory_root)
        .join("operations")
        .join("self_review");
    fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create self review dir: {error}"))?;
    memory::write_json(dir.join(format!("{}.json", package.package_id)), package)?;
    fs::write(
        dir.join(format!("{}.ru.md", package.package_id)),
        render_self_review_markdown(package),
    )
    .map_err(|error| format!("failed to write self review markdown: {error}"))
}

fn render_self_review_markdown(package: &SelfReviewPackage) -> String {
    format!(
        "# EVA Self Review Package\n\npackage_id={}\nrelease_health={}\npreflight_gate={}\nartifact_audit={}\ndeterminism_audit={}\ngovernance_status={}\npromotion_queue_status={}\nrelease_status={}\nself_modification_allowed_now={}\nreason_ru={}\nrecommended_next_command={}\n\nforbidden_actions:\n{}\n",
        package.package_id,
        package.release_health,
        package.preflight_gate,
        package.artifact_audit,
        package.determinism_audit,
        package.governance_status,
        package.promotion_queue_status,
        package.release_status,
        package.self_modification_allowed_now,
        package.self_modification_reason_ru,
        package.recommended_next_command,
        package
            .forbidden_actions
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn self_review_markdown_path(memory_root: &str, package_id: &str) -> PathBuf {
    Path::new(memory_root)
        .join("operations")
        .join("self_review")
        .join(format!("{package_id}.ru.md"))
}

fn latest_self_review_package(memory_root: &str) -> Result<Option<SelfReviewPackage>, String> {
    let mut packages = load_self_review_packages(memory_root)?;
    packages.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.package_id.cmp(&right.package_id))
    });
    Ok(packages.pop())
}

fn load_self_review_packages(memory_root: &str) -> Result<Vec<SelfReviewPackage>, String> {
    let dir = Path::new(memory_root)
        .join("operations")
        .join("self_review");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut packages = Vec::new();
    for entry in fs::read_dir(&dir)
        .map_err(|error| format!("failed to read self review packages: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("failed to read self review package entry: {error}"))?;
        let path = entry.path();
        if !path.extension().is_some_and(|ext| ext == "json") {
            continue;
        }
        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read self review package: {error}"))?;
        let package: SelfReviewPackage = serde_json::from_str(&contents)
            .map_err(|error| format!("failed to parse self review package: {error}"))?;
        packages.push(package);
    }
    Ok(packages)
}
