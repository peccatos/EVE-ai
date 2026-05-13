use crate::agent::outcome::{build_task_outcome, list_task_outcomes};
use crate::agent::storage::{id, memory_path, save_json_pretty};
use crate::contracts::{FitnessDecision, TaskOutcome, WorkFitnessScore};

pub fn score_task_outcome(outcome: &TaskOutcome) -> WorkFitnessScore {
    let validation_score = if outcome.validation_status == "passed" {
        4.0
    } else {
        0.0
    };
    let approval_score = if outcome.approved { 2.0 } else { 0.0 };
    let usefulness_score = if outcome.success { 3.0 } else { 0.5 };
    let risk_penalty = match outcome.risk_level.as_str() {
        "high" => 3.0,
        "medium" => 1.0,
        _ => 0.0,
    };
    let patch_size_penalty = if outcome.patch_ops_count > 5 {
        1.5
    } else {
        0.0
    };
    let failure_penalty = if outcome.failure_reason.is_some() {
        2.0
    } else {
        0.0
    };
    let raw_score: f32 = validation_score + approval_score + usefulness_score
        - risk_penalty
        - patch_size_penalty
        - failure_penalty;
    let final_score = raw_score.clamp(0.0, 10.0);
    let decision = if outcome.validation_status != "passed" {
        FitnessDecision::Reject
    } else if final_score >= 8.0 {
        FitnessDecision::Prefer
    } else if final_score >= 5.0 {
        FitnessDecision::Acceptable
    } else if final_score > 0.0 {
        FitnessDecision::Risky
    } else {
        FitnessDecision::Unknown
    };
    WorkFitnessScore {
        fitness_id: id("fitness"),
        task_id: outcome.task_id.clone(),
        proposal_id: outcome.proposal_id.clone(),
        validation_score,
        approval_score,
        usefulness_score,
        risk_penalty,
        patch_size_penalty,
        failure_penalty,
        final_score,
        decision,
        explanation: vec![
            format!("validation_status={}", outcome.validation_status),
            format!("approved={}", outcome.approved),
            format!("risk_level={}", outcome.risk_level),
        ],
    }
}

pub fn build_fitness(
    memory_root: &str,
    task_id: Option<&str>,
) -> Result<Vec<WorkFitnessScore>, String> {
    let outcomes = if let Some(task_id) = task_id {
        vec![build_task_outcome(memory_root, task_id)?]
    } else {
        list_task_outcomes(memory_root)?
    };
    let scores = outcomes.iter().map(score_task_outcome).collect::<Vec<_>>();
    save_json_pretty(
        &memory_path(memory_root, &["fitness", "latest_fitness.json"]),
        &scores,
    )?;
    Ok(scores)
}

pub fn print_fitness(memory_root: &str, task_id: Option<&str>) -> Result<String, String> {
    let scores = build_fitness(memory_root, task_id)?;
    let prefer = scores
        .iter()
        .filter(|score| score.decision == FitnessDecision::Prefer)
        .count();
    let acceptable = scores
        .iter()
        .filter(|score| score.decision == FitnessDecision::Acceptable)
        .count();
    let risky = scores
        .iter()
        .filter(|score| score.decision == FitnessDecision::Risky)
        .count();
    let reject = scores
        .iter()
        .filter(|score| score.decision == FitnessDecision::Reject)
        .count();
    let unknown = scores
        .iter()
        .filter(|score| score.decision == FitnessDecision::Unknown)
        .count();
    Ok(format!(
        "EVA Work Fitness\ntasks_scored={}\nprefer={}\nacceptable={}\nrisky={}\nreject={}\nunknown={}",
        scores.len(), prefer, acceptable, risky, reject, unknown
    ))
}
