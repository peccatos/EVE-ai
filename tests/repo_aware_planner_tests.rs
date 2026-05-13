mod agent_v2_support;

use std::fs;

use agent_v2_support::temp_agent_root;
use eva_runtime_with_task_validator::{build_repo_map, create_task, plan_task};

#[test]
fn repo_map_detects_project_shape_and_ignores_runtime_dirs() {
    let root = temp_agent_root("repo-map-shape");
    fs::create_dir_all(root.join("target/debug")).expect("target");
    fs::write(
        root.join("target/debug/ignored.rs"),
        "pub fn ignored() {}\n",
    )
    .expect("target file");
    fs::create_dir_all(root.join("memory")).expect("memory");
    fs::write(root.join("memory/ignored.rs"), "pub fn ignored() {}\n").expect("memory file");
    let memory = root.join("memory");
    let map = build_repo_map(root.to_str().unwrap(), memory.to_str().unwrap()).expect("repo map");
    assert!(map.modules.iter().any(|m| m.path == "src/main.rs"));
    assert!(map.modules.iter().any(|m| m.path == "src/lib.rs"));
    assert!(map.tests.iter().any(|p| p == "tests/basic_tests.rs"));
    assert!(map.docs.iter().any(|p| p == "docs/intro.md"));
    assert!(!map.modules.iter().any(|m| m.path.contains("target/")));
    assert!(!map.modules.iter().any(|m| m.path.contains("memory/")));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn planner_uses_repo_map_for_cli_and_test_tasks() {
    let root = temp_agent_root("repo-aware-plan");
    let memory = root.join("memory");
    let task = create_task(memory.to_str().unwrap(), "add cli command").expect("task");
    let plan = plan_task(
        root.to_str().unwrap(),
        memory.to_str().unwrap(),
        &task.task_id,
    )
    .expect("plan");
    assert!(plan.likely_files.iter().any(|path| path == "src/main.rs"));

    let task = create_task(memory.to_str().unwrap(), "add test for apply").expect("task");
    let plan = plan_task(
        root.to_str().unwrap(),
        memory.to_str().unwrap(),
        &task.task_id,
    )
    .expect("plan");
    assert!(plan
        .likely_files
        .iter()
        .any(|path| path.starts_with("tests/")));
    fs::remove_dir_all(root).expect("cleanup");
}
