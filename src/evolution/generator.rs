use crate::contracts::{MutationContract, MutationKind, MutationObjective, MutationPlan};

pub fn generate_safe_mutation() -> MutationContract {
    MutationContract {
        id: "phase1-append-runtime-note".to_string(),
        kind: MutationKind::AppendComment,
        target_file: "src/runtime_cycle.rs".to_string(),
        search: None,
        replace: None,
        append: Some("// EVA Phase 1 sandbox-only mutation probe.".to_string()),
        reason: "prove bounded sandbox mutation without touching core project".to_string(),
        expected_gain: 0.05,
        risk: 0.1,
    }
}

pub fn generate_from_plan(plan: &MutationPlan) -> MutationContract {
    let (search, replace, append) = match plan.mutation_kind {
        MutationKind::AppendComment => (
            None,
            None,
            Some(format!(
                "// EVA planned note: {}.",
                safe_reason_fragment(&plan.reason)
            )),
        ),
        MutationKind::ReplaceText => (
            Some("// EVA Phase 1 sandbox-only mutation probe.".to_string()),
            Some("// EVA graph-guided sandbox-only mutation probe.".to_string()),
            None,
        ),
        MutationKind::ParameterTune => (
            Some("risk: 0.1".to_string()),
            Some("risk: 0.09".to_string()),
            None,
        ),
        MutationKind::AddTestSkeleton => (
            None,
            None,
            Some(format!(
                "\n#[test]\nfn eva_generated_{}_skeleton() {{\n    assert!(true);\n}}\n",
                plan.id.replace('-', "_").replace(':', "_")
            )),
        ),
        MutationKind::AddMetricField => (
            None,
            None,
            Some("// EVA metric placeholder: planned compact metric extension.".to_string()),
        ),
    };

    MutationContract {
        id: format!("mutation:{}", plan.id),
        kind: plan.mutation_kind,
        target_file: plan.target_file.clone(),
        search,
        replace,
        append,
        reason: format!(
            "planned {:?} from graph evidence: {}",
            plan.objective,
            plan.graph_evidence.join(",")
        ),
        expected_gain: plan.expected_gain.clamp(0.0, 1.0),
        risk: plan.estimated_risk.clamp(0.0, 1.0),
    }
}

pub fn default_kind_for_objective(objective: MutationObjective) -> MutationKind {
    match objective {
        MutationObjective::ImproveTests => MutationKind::AddTestSkeleton,
        MutationObjective::ImproveScoring | MutationObjective::ReduceStorage => {
            MutationKind::AddMetricField
        }
        _ => MutationKind::AppendComment,
    }
}

fn safe_reason_fragment(reason: &str) -> String {
    reason
        .chars()
        .filter(|ch| {
            ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace() || *ch == '-' || *ch == '_'
        })
        .take(120)
        .collect::<String>()
        .trim()
        .to_string()
}
