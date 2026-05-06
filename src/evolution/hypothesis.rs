use serde::{Deserialize, Serialize};

use crate::contracts::{MutationObjective, MutationPlan};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvolutionHypothesis {
    pub id: String,
    pub plan_id: String,
    pub objective: MutationObjective,
    pub target_file: String,
    pub expected_gain: f32,
    pub estimated_risk: f32,
    pub evidence_weight: f32,
    pub priority: f32,
    pub graph_evidence: Vec<String>,
}

pub fn rank_plans(plans: &[MutationPlan]) -> Vec<EvolutionHypothesis> {
    let mut hypotheses = plans
        .iter()
        .map(|plan| EvolutionHypothesis {
            id: format!("hypothesis:{}", plan.id),
            plan_id: plan.id.clone(),
            objective: plan.objective,
            target_file: plan.target_file.clone(),
            expected_gain: plan.expected_gain,
            estimated_risk: plan.estimated_risk,
            evidence_weight: plan.evidence_weight,
            priority: plan.expected_gain - plan.estimated_risk + plan.evidence_weight,
            graph_evidence: plan.graph_evidence.clone(),
        })
        .collect::<Vec<_>>();
    hypotheses.sort_by(|left, right| {
        right
            .priority
            .total_cmp(&left.priority)
            .then_with(|| left.plan_id.cmp(&right.plan_id))
    });
    hypotheses
}
