mod agent_v2_support;

use std::fs;

use agent_v2_support::temp_agent_root;
use eva_runtime_with_task_validator::{
    approve_proposal, create_task, dry_run_apply, plan_task, print_proposal_show, propose_task,
};

#[test]
fn proposal_show_and_dry_run_are_read_only() {
    let root = temp_agent_root("proposal-dry-run");
    let memory = root.join("memory");
    let task = create_task(memory.to_str().unwrap(), "document dry run").expect("task");
    plan_task(
        root.to_str().unwrap(),
        memory.to_str().unwrap(),
        &task.task_id,
    )
    .expect("plan");
    let proposal = propose_task(
        root.to_str().unwrap(),
        memory.to_str().unwrap(),
        &task.task_id,
    )
    .expect("proposal");
    let show = print_proposal_show(memory.to_str().unwrap(), &proposal.proposal_id).expect("show");
    assert!(show.contains("EVA Patch Proposal"));
    let before_exists = root.join(&proposal.files_to_change[0]).exists();
    assert!(!before_exists);
    let dry = dry_run_apply(
        root.to_str().unwrap(),
        memory.to_str().unwrap(),
        &proposal.proposal_id,
    )
    .expect("dry run");
    assert!(dry.contains("would_apply=false"));
    assert!(dry.contains("not_approved"));
    assert!(!root.join(&proposal.files_to_change[0]).exists());
    assert!(!memory.join("applies").exists());
    approve_proposal(memory.to_str().unwrap(), &proposal.proposal_id).expect("approve");
    let dry = dry_run_apply(
        root.to_str().unwrap(),
        memory.to_str().unwrap(),
        &proposal.proposal_id,
    )
    .expect("dry run approved");
    assert!(dry.contains("would_apply=true"));
    assert!(!root.join(&proposal.files_to_change[0]).exists());
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn proposal_show_handles_missing_proposal() {
    let root = temp_agent_root("proposal-show-missing");
    let output =
        print_proposal_show(root.join("memory").to_str().unwrap(), "missing").expect("show");
    assert!(output.contains("proposal not found"));
    fs::remove_dir_all(root).expect("cleanup");
}
