mod agent_v2_support;

use std::fs;

use agent_v2_support::temp_agent_root;
use eva_runtime_with_task_validator::{
    proposal_from_llm_response, LlmResponse, LlmStatus, PatchOperationKind, ProposalStatus,
};
use serde_json::json;

fn response(value: serde_json::Value) -> LlmResponse {
    LlmResponse {
        request_id: "req".to_string(),
        provider: "openai".to_string(),
        model: "gpt-5.5".to_string(),
        status: LlmStatus::Completed,
        output_text: value.to_string(),
        parsed_json: Some(value),
        warnings: Vec::new(),
        blockers: Vec::new(),
    }
}

#[test]
fn mock_openai_valid_structured_proposal_is_accepted() {
    let root = temp_agent_root("llm-valid-proposal");
    let memory = root.join("memory");
    let proposal = proposal_from_llm_response(
        memory.to_str().unwrap(),
        "task-1",
        "plan-1",
        "document",
        &response(json!({
            "summary": "docs",
            "files_to_change": ["docs/example.md"],
            "risk_level": "low",
            "patch_ops": [{"path":"docs/example.md","op":"CreateFile","description":"doc","content":"# Example\n"}]
        })),
    )
    .expect("proposal");
    assert_eq!(proposal.status, ProposalStatus::AwaitingApproval);
    assert_eq!(proposal.patch_ops[0].op, PatchOperationKind::CreateFile);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn malformed_or_forbidden_llm_proposals_are_refused() {
    let root = temp_agent_root("llm-refused-proposal");
    let memory = root.join("memory");
    assert!(proposal_from_llm_response(
        memory.to_str().unwrap(),
        "task-1",
        "plan-1",
        "bad",
        &response(json!({"summary":"bad"})),
    )
    .is_err());
    let forbidden = proposal_from_llm_response(
        memory.to_str().unwrap(),
        "task-1",
        "plan-1",
        "bad",
        &response(json!({
            "summary": "bad",
            "files_to_change": ["memory/x.json"],
            "risk_level": "low",
            "approved": true,
            "patch_ops": [{"path":"../src/lib.rs","op":"CreateFile","description":"bad","content":"x"}]
        })),
    )
    .expect("forbidden proposal");
    assert_eq!(forbidden.status, ProposalStatus::Refused);
    assert!(forbidden
        .blockers
        .iter()
        .any(|b| b.contains("path_traversal")));
    assert!(forbidden
        .blockers
        .contains(&"llm_attempted_gate_bypass".to_string()));
    fs::remove_dir_all(root).expect("cleanup");
}
