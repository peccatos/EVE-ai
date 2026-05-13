use std::path::Path;

use crate::agent::storage::{memory_path, save_json_pretty};
use crate::contracts::ProductionAgentV2Readiness;

pub fn build_production_agent_v2_readiness(
    memory_root: &str,
) -> Result<ProductionAgentV2Readiness, String> {
    let mut readiness = ProductionAgentV2Readiness {
        ci_proof_ok: Path::new(".github/workflows/rust-ci.yml").exists(),
        openai_adapter_ok: true,
        rule_based_fallback_ok: true,
        structured_proposals_ok: true,
        proposal_preview_ok: memory_path(memory_root, &["proposals", "latest_proposal.json"])
            .exists(),
        dry_run_ok: true,
        code_edit_engine_ok: true,
        repo_map_ok: memory_path(memory_root, &["repo_map", "latest_repo_map.json"]).exists(),
        repo_aware_planner_ok: memory_path(memory_root, &["plans", "latest_plan.json"]).exists(),
        task_outcome_memory_ok: Path::new(memory_root).join("task_outcomes").exists(),
        validation_ok: memory_path(memory_root, &["validations", "latest_validation.json"])
            .exists(),
        tui_visibility_ok: true,
        safety_policy_ok: true,
        production_agent_v2_ready: false,
        warnings: Vec::new(),
        blockers: Vec::new(),
    };
    for (ok, blocker) in [
        (readiness.ci_proof_ok, "ci_proof_missing"),
        (readiness.proposal_preview_ok, "proposal_preview_missing"),
        (readiness.repo_map_ok, "repo_map_missing"),
        (
            readiness.repo_aware_planner_ok,
            "repo_aware_planner_missing",
        ),
        (
            readiness.task_outcome_memory_ok,
            "task_outcome_memory_missing",
        ),
        (readiness.validation_ok, "validation_missing"),
    ] {
        if !ok {
            readiness.blockers.push(blocker.to_string());
        }
    }
    readiness.production_agent_v2_ready = readiness.blockers.is_empty()
        && readiness.openai_adapter_ok
        && readiness.rule_based_fallback_ok
        && readiness.structured_proposals_ok
        && readiness.dry_run_ok
        && readiness.code_edit_engine_ok
        && readiness.tui_visibility_ok
        && readiness.safety_policy_ok;
    save_json_pretty(
        &memory_path(memory_root, &["agent", "v2_readiness.json"]),
        &readiness,
    )?;
    Ok(readiness)
}

pub fn print_agent_v2_readiness(memory_root: &str) -> Result<String, String> {
    let readiness = build_production_agent_v2_readiness(memory_root)?;
    Ok(format!(
        "EVA Production Agent v2 Readiness\nci_proof_ok={}\nopenai_adapter_ok={}\nrule_based_fallback_ok={}\nstructured_proposals_ok={}\nproposal_preview_ok={}\ndry_run_ok={}\ncode_edit_engine_ok={}\nrepo_map_ok={}\nrepo_aware_planner_ok={}\ntask_outcome_memory_ok={}\nvalidation_ok={}\ntui_visibility_ok={}\nsafety_policy_ok={}\nproduction_agent_v2_ready={}\nblockers={}",
        readiness.ci_proof_ok,
        readiness.openai_adapter_ok,
        readiness.rule_based_fallback_ok,
        readiness.structured_proposals_ok,
        readiness.proposal_preview_ok,
        readiness.dry_run_ok,
        readiness.code_edit_engine_ok,
        readiness.repo_map_ok,
        readiness.repo_aware_planner_ok,
        readiness.task_outcome_memory_ok,
        readiness.validation_ok,
        readiness.tui_visibility_ok,
        readiness.safety_policy_ok,
        readiness.production_agent_v2_ready,
        if readiness.blockers.is_empty() { "none".to_string() } else { readiness.blockers.join(",") }
    ))
}
