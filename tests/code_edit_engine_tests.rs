mod agent_v2_support;

use std::fs;

use agent_v2_support::temp_agent_root;
use eva_runtime_with_task_validator::{
    apply_proposal, approve_proposal, create_task, proposal_from_llm_response, validate_patch_path,
    LlmResponse, LlmStatus, ProposalStatus,
};
use serde_json::json;

fn response(value: serde_json::Value) -> LlmResponse {
    LlmResponse {
        request_id: "req".to_string(),
        provider: "mock".to_string(),
        model: "mock".to_string(),
        status: LlmStatus::Completed,
        output_text: value.to_string(),
        parsed_json: Some(value),
        warnings: Vec::new(),
        blockers: Vec::new(),
    }
}

fn proposal(
    memory: &str,
    path: &str,
    op: &str,
    content: Option<&str>,
    find: Option<&str>,
    replace: Option<&str>,
) -> String {
    let task = create_task(memory, "edit").expect("task");
    let mut patch = json!({"path":path,"op":op,"description":"test"});
    if let Some(content) = content {
        patch["content"] = json!(content);
    }
    if let Some(find) = find {
        patch["find"] = json!(find);
    }
    if let Some(replace) = replace {
        patch["replace"] = json!(replace);
    }
    proposal_from_llm_response(
        memory,
        &task.task_id,
        "plan-1",
        "edit",
        &response(json!({"summary":"edit","files_to_change":[path],"risk_level":"low","patch_ops":[patch]})),
    )
    .expect("proposal")
    .proposal_id
}

#[test]
fn replace_exact_text_is_ambiguity_safe() {
    let root = temp_agent_root("replace-exact");
    let memory = root.join("memory");
    fs::write(root.join("docs/exact.md"), "alpha\nbeta\n").expect("doc");
    let id = proposal(
        memory.to_str().unwrap(),
        "docs/exact.md",
        "ReplaceExactText",
        None,
        Some("beta"),
        Some("gamma"),
    );
    approve_proposal(memory.to_str().unwrap(), &id).expect("approve");
    let result =
        apply_proposal(root.to_str().unwrap(), memory.to_str().unwrap(), &id).expect("apply");
    assert!(result.blockers.is_empty());
    assert!(fs::read_to_string(root.join("docs/exact.md"))
        .unwrap()
        .contains("gamma"));

    fs::write(root.join("docs/zero.md"), "alpha\n").expect("doc");
    let id = proposal(
        memory.to_str().unwrap(),
        "docs/zero.md",
        "ReplaceExactText",
        None,
        Some("missing"),
        Some("x"),
    );
    approve_proposal(memory.to_str().unwrap(), &id).expect("approve");
    let result =
        apply_proposal(root.to_str().unwrap(), memory.to_str().unwrap(), &id).expect("apply");
    assert!(result
        .blockers
        .contains(&"exact_text_not_found".to_string()));

    fs::write(root.join("docs/multi.md"), "a\na\n").expect("doc");
    let id = proposal(
        memory.to_str().unwrap(),
        "docs/multi.md",
        "ReplaceExactText",
        None,
        Some("a"),
        Some("b"),
    );
    approve_proposal(memory.to_str().unwrap(), &id).expect("approve");
    let result =
        apply_proposal(root.to_str().unwrap(), memory.to_str().unwrap(), &id).expect("apply");
    assert!(result
        .blockers
        .contains(&"ambiguous_exact_text_match".to_string()));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn edit_engine_refuses_invalid_file_states_and_paths() {
    let root = temp_agent_root("edit-invalid-states");
    let memory = root.join("memory");
    let id = proposal(
        memory.to_str().unwrap(),
        "docs/intro.md",
        "CreateFile",
        Some("x"),
        None,
        None,
    );
    approve_proposal(memory.to_str().unwrap(), &id).expect("approve");
    let result =
        apply_proposal(root.to_str().unwrap(), memory.to_str().unwrap(), &id).expect("apply");
    assert!(result.blockers.contains(&"create_file_exists".to_string()));

    let id = proposal(
        memory.to_str().unwrap(),
        "docs/missing.md",
        "AppendFile",
        Some("x"),
        None,
        None,
    );
    approve_proposal(memory.to_str().unwrap(), &id).expect("approve");
    let result =
        apply_proposal(root.to_str().unwrap(), memory.to_str().unwrap(), &id).expect("apply");
    assert!(result.blockers.contains(&"append_file_missing".to_string()));

    let id = proposal(
        memory.to_str().unwrap(),
        "docs/missing-replace.md",
        "ReplaceFileIfExists",
        Some("x"),
        None,
        None,
    );
    approve_proposal(memory.to_str().unwrap(), &id).expect("approve");
    let result =
        apply_proposal(root.to_str().unwrap(), memory.to_str().unwrap(), &id).expect("apply");
    assert!(result
        .blockers
        .contains(&"replace_file_missing".to_string()));

    for path in [
        "../src/main.rs",
        "/etc/passwd",
        "memory/x.json",
        ".git/config",
        "target/debug/x",
    ] {
        assert!(validate_patch_path(path).is_err());
    }
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn large_patch_is_refused() {
    let root = temp_agent_root("large-patch");
    let memory = root.join("memory");
    let large = "x".repeat(21 * 1024);
    let proposal = proposal_from_llm_response(
        memory.to_str().unwrap(),
        "task-1",
        "plan-1",
        "large",
        &response(json!({"summary":"large","files_to_change":["docs/large.md"],"risk_level":"low","patch_ops":[{"path":"docs/large.md","op":"CreateFile","description":"large","content":large}]})),
    )
    .expect("proposal");
    assert_eq!(proposal.status, ProposalStatus::Refused);
    assert!(proposal.blockers.contains(&"patch_too_large".to_string()));
    fs::remove_dir_all(root).expect("cleanup");
}
