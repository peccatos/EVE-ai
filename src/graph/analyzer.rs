use std::path::Path;

use crate::contracts::{MutationKind, MutationObjective, MutationPlan};
use crate::evolution::generator::default_kind_for_objective;
use crate::graph::load_graph;

pub fn propose_mutation_plans(memory_root: &str) -> Result<Vec<MutationPlan>, String> {
    let graph = load_graph(&Path::new(memory_root).join("graph.json"))?;
    let mut safe_files = graph
        .nodes
        .iter()
        .filter(|node| node.kind == "File" || node.kind == "TargetFile")
        .filter_map(|node| node.id.strip_prefix("file:").map(str::to_string))
        .filter(|file| is_safe_target(file))
        .collect::<Vec<_>>();
    safe_files.sort();
    safe_files.dedup();

    let mut plans = Vec::new();
    for file in safe_files.into_iter().take(8) {
        let evidence = graph
            .edges
            .iter()
            .filter(|edge| edge.to == format!("file:{file}"))
            .take(3)
            .map(|edge| edge.from.clone())
            .collect::<Vec<_>>();
        let objective = objective_for_file(&file);
        let mutation_kind = kind_for_file(&file, objective);
        plans.push(MutationPlan {
            id: format!("plan:{}", file.replace('/', "_").replace('.', "_")),
            objective,
            target_file: target_for_kind(&file, mutation_kind),
            mutation_kind,
            reason: format!("graph-guided safe improvement for {file}"),
            expected_gain: expected_gain(objective),
            estimated_risk: 0.12,
            evidence_weight: if evidence.is_empty() { 0.0 } else { 0.2 },
            graph_evidence: evidence,
        });
    }
    plans.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(plans)
}

pub fn render_plans(memory_root: &str) -> Result<String, String> {
    let plans = propose_mutation_plans(memory_root)?;
    let hypotheses = crate::evolution::rank_plans(&plans);
    if hypotheses.is_empty() {
        return Ok("(none)".to_string());
    }
    Ok(hypotheses
        .iter()
        .take(5)
        .map(|hypothesis| {
            format!(
                "{} priority={:.2} objective={:?} target={}",
                hypothesis.plan_id,
                hypothesis.priority,
                hypothesis.objective,
                hypothesis.target_file
            )
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

fn is_safe_target(file: &str) -> bool {
    file.starts_with("src/")
        && !file.starts_with("src/core/")
        && file != "src/main.rs"
        && file != "src/lib.rs"
        && file != "Cargo.toml"
        && !file.ends_with("/Cargo.toml")
}

fn objective_for_file(file: &str) -> MutationObjective {
    if file.contains("validator") {
        MutationObjective::ImproveValidation
    } else if file.contains("replay") || file.contains("promotion") {
        MutationObjective::ImproveReplayability
    } else if file.contains("graph") {
        MutationObjective::ImproveGraphMemory
    } else if file.contains("test") {
        MutationObjective::ImproveTests
    } else {
        MutationObjective::ImproveReliability
    }
}

fn kind_for_file(file: &str, objective: MutationObjective) -> MutationKind {
    if file.contains("tests/") {
        MutationKind::AppendComment
    } else {
        default_kind_for_objective(objective)
    }
}

fn target_for_kind(file: &str, kind: MutationKind) -> String {
    match kind {
        MutationKind::AddTestSkeleton => "tests/eva_generated_phase37_tests.rs".to_string(),
        _ => file.to_string(),
    }
}

fn expected_gain(objective: MutationObjective) -> f32 {
    match objective {
        MutationObjective::ImproveValidation | MutationObjective::ImproveReplayability => 0.55,
        MutationObjective::ImproveGraphMemory | MutationObjective::ImproveTests => 0.5,
        _ => 0.4,
    }
}
