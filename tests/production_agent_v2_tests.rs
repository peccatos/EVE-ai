mod agent_v2_support;

use std::fs;

use agent_v2_support::temp_agent_root;
use eva_runtime_with_task_validator::{
    build_production_agent_v2_readiness, build_repo_map, inspect_workspace,
    load_tui_state_from_project_root, print_agent_v2_readiness,
};

#[test]
fn git_status_is_never_empty_and_repo_map_exists() {
    let root = temp_agent_root("v2-git-status");
    let memory = root.join("memory");
    let inspection =
        inspect_workspace(root.to_str().unwrap(), memory.to_str().unwrap()).expect("inspection");
    assert!(matches!(
        inspection.git_status.as_str(),
        "clean" | "dirty" | "unknown"
    ));
    let map = build_repo_map(root.to_str().unwrap(), memory.to_str().unwrap()).expect("repo map");
    assert!(map.cargo_project);
    assert!(map.entrypoints.contains(&"src/main.rs".to_string()));
    assert!(map.entrypoints.contains(&"src/lib.rs".to_string()));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn agent_v2_readiness_reports_missing_then_ready_components() {
    let root = temp_agent_root("v2-readiness");
    let memory = root.join("memory");
    let initial = build_production_agent_v2_readiness(memory.to_str().unwrap()).expect("readiness");
    assert!(!initial.production_agent_v2_ready);
    assert!(initial.blockers.contains(&"repo_map_missing".to_string()));
    build_repo_map(root.to_str().unwrap(), memory.to_str().unwrap()).expect("repo map");
    fs::create_dir_all(memory.join("proposals")).expect("proposals");
    fs::write(memory.join("proposals/latest_proposal.json"), "{}").expect("proposal marker");
    fs::create_dir_all(memory.join("plans")).expect("plans");
    fs::write(memory.join("plans/latest_plan.json"), "{}").expect("plan marker");
    fs::create_dir_all(memory.join("task_outcomes")).expect("outcomes");
    fs::create_dir_all(memory.join("validations")).expect("validations");
    fs::write(memory.join("validations/latest_validation.json"), "{}").expect("validation marker");
    let output = print_agent_v2_readiness(memory.to_str().unwrap()).expect("print");
    assert!(output.contains("production_agent_v2_ready=true"));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn tui_loads_agent_v2_state_without_openai_or_writes() {
    let root = temp_agent_root("v2-tui");
    let memory = root.join("memory");
    build_repo_map(root.to_str().unwrap(), memory.to_str().unwrap()).expect("repo map");
    fs::create_dir_all(memory.join("task_outcomes")).expect("outcomes");
    fs::write(memory.join("task_outcomes/task-a.json"), "{}").expect("outcome marker");
    build_production_agent_v2_readiness(memory.to_str().unwrap()).expect("readiness");
    let state = load_tui_state_from_project_root(&root);
    assert_eq!(state.agent.repo_map_modules, 2);
    assert_eq!(state.agent.task_outcome_count, 1);
    assert_eq!(state.agent.llm_provider, "rule_based");
    fs::remove_dir_all(root).expect("cleanup");
}
