use crate::agent::storage::{id, memory_path, save_json_pretty};
use crate::agent::task::create_task;
use crate::contracts::SelfImprovementProposal;
use crate::evolution::outcome_analyzer::build_task_outcome_analysis;
use crate::evolution::strategy_selection::select_strategy;

pub fn propose_self_improvement(memory_root: &str) -> Result<SelfImprovementProposal, String> {
    let analysis = build_task_outcome_analysis(memory_root)?;
    if analysis.total_tasks == 0 {
        let proposal = SelfImprovementProposal {
            self_improvement_id: id("self-improve"),
            source_outcomes: Vec::new(),
            weakness: "no_task_outcomes".to_string(),
            proposed_goal: "collect real task outcomes before self-improvement".to_string(),
            recommended_strategy: "manual_decomposition_required".to_string(),
            allowed_scope: allowed_scope(),
            forbidden_paths: forbidden_paths(),
            risk_level: "low".to_string(),
            approval_required: true,
            proposal_id: None,
            warnings: vec!["no_outcomes_available".to_string()],
            blockers: vec!["no_task_outcomes".to_string()],
        };
        save(&proposal, memory_root)?;
        return Ok(proposal);
    }
    let weakness = analysis
        .common_failure_reasons
        .first()
        .cloned()
        .unwrap_or_else(|| "improve_agent_docs_and_tests".to_string());
    let proposed_goal = format!("document and test mitigation for {weakness}");
    let (strategy, _, _) = select_strategy(memory_root, &proposed_goal)?;
    let task = create_task(memory_root, &proposed_goal)?;
    let proposal = SelfImprovementProposal {
        self_improvement_id: id("self-improve"),
        source_outcomes: vec![format!("outcomes={}", analysis.total_tasks)],
        weakness,
        proposed_goal,
        recommended_strategy: strategy,
        allowed_scope: allowed_scope(),
        forbidden_paths: forbidden_paths(),
        risk_level: "low".to_string(),
        approval_required: true,
        proposal_id: None,
        warnings: vec!["proposal_only_no_apply".to_string()],
        blockers: Vec::new(),
    };
    save(&proposal, memory_root)?;
    save_json_pretty(
        &memory_path(memory_root, &["self_improvement", "latest_task.json"]),
        &task,
    )?;
    Ok(proposal)
}

pub fn print_self_improve_propose(memory_root: &str) -> Result<String, String> {
    let proposal = propose_self_improvement(memory_root)?;
    if !proposal.blockers.is_empty() {
        return Ok(format!(
            "self-improvement refused\nreason={}\napproval_required=true",
            proposal.blockers.join(",")
        ));
    }
    Ok(format!(
        "EVA Self-Improvement Proposal\nself_improvement_id={}\nweakness={}\nproposed_goal={}\nrecommended_strategy={}\napproval_required={}\nproposal_only=true",
        proposal.self_improvement_id,
        proposal.weakness,
        proposal.proposed_goal,
        proposal.recommended_strategy,
        proposal.approval_required
    ))
}

fn save(proposal: &SelfImprovementProposal, memory_root: &str) -> Result<(), String> {
    save_json_pretty(
        &memory_path(
            memory_root,
            &[
                "self_improvement",
                &format!("{}.json", proposal.self_improvement_id),
            ],
        ),
        proposal,
    )?;
    save_json_pretty(
        &memory_path(memory_root, &["self_improvement", "latest.json"]),
        proposal,
    )
}

fn allowed_scope() -> Vec<String> {
    [
        "docs/",
        "tests/",
        "src/agent/",
        "src/llm/",
        "src/tui/",
        "src/contracts/",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn forbidden_paths() -> Vec<String> {
    [
        ".git/",
        "target/",
        "memory/",
        "releases/",
        "sandboxes/",
        "approval_bypass",
        "validation_bypass",
        "network_behavior",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}
