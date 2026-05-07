use serde::{Deserialize, Serialize};

use crate::contracts::{MutationKind, RecombinedHypothesis, TaskContract};
use crate::evolution::{
    classify_mutation_kind_label, load_recombined_hypotheses, matches_target_patterns,
    mutation_class_label, MutationClass,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CampaignRecombinationDiagnostics {
    pub recombination_fallback_attempted: bool,
    pub recombination_fallback_used: bool,
    pub recombination_candidates_seen: usize,
    pub recombination_accepted: usize,
    pub recombination_rejected_by_target: usize,
    pub recombination_rejected_by_kind: usize,
    pub recombination_rejected_by_risk: usize,
    pub recombination_rejected_by_forbidden_target: usize,
    pub recombination_rejected_by_class: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_hypothesis_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_risk: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recombination_fallback_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CampaignRecombinationPreview {
    pub diagnostics: CampaignRecombinationDiagnostics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_hypothesis: Option<RecombinedHypothesis>,
}

pub fn preview_campaign_recombination(
    memory_root: &str,
    task: &TaskContract,
) -> Result<CampaignRecombinationPreview, String> {
    let (selected_hypothesis, diagnostics) = select_task_compatible_hypothesis(memory_root, task)?;
    Ok(CampaignRecombinationPreview {
        diagnostics,
        selected_hypothesis,
    })
}

pub fn select_task_compatible_hypothesis(
    memory_root: &str,
    task: &TaskContract,
) -> Result<
    (
        Option<RecombinedHypothesis>,
        CampaignRecombinationDiagnostics,
    ),
    String,
> {
    let hypotheses = load_recombined_hypotheses(memory_root)?;
    Ok(select_task_compatible_from_hypotheses(hypotheses, task))
}

pub fn select_task_compatible_from_hypotheses(
    hypotheses: Vec<RecombinedHypothesis>,
    task: &TaskContract,
) -> (
    Option<RecombinedHypothesis>,
    CampaignRecombinationDiagnostics,
) {
    let mut diagnostics = CampaignRecombinationDiagnostics {
        recombination_fallback_attempted: true,
        ..CampaignRecombinationDiagnostics::default()
    };
    diagnostics.recombination_candidates_seen = hypotheses.len();

    for hypothesis in hypotheses {
        let class = classify_mutation_kind_label(&hypothesis.suggested_mutation_kind, true);
        if class != MutationClass::Useful {
            diagnostics.recombination_rejected_by_class += 1;
            continue;
        }
        if is_forbidden_target(&hypothesis.suggested_target)
            || matches_target_patterns(&hypothesis.suggested_target, &task.forbidden_targets)
        {
            diagnostics.recombination_rejected_by_forbidden_target += 1;
            continue;
        }
        if !task.allowed_targets.is_empty()
            && !matches_target_patterns(&hypothesis.suggested_target, &task.allowed_targets)
        {
            diagnostics.recombination_rejected_by_target += 1;
            continue;
        }
        let kind = mutation_kind_from_label(&hypothesis.suggested_mutation_kind);
        if !task.allowed_mutation_kinds.is_empty()
            && kind.is_some_and(|kind| !task.allowed_mutation_kinds.contains(&kind))
        {
            diagnostics.recombination_rejected_by_kind += 1;
            continue;
        }
        if task.denied_mutation_kinds.iter().any(|denied| {
            format!("{denied:?}").eq_ignore_ascii_case(&hypothesis.suggested_mutation_kind)
        }) {
            diagnostics.recombination_rejected_by_kind += 1;
            continue;
        }
        if hypothesis.estimated_risk > task.max_risk {
            diagnostics.recombination_rejected_by_risk += 1;
            continue;
        }

        diagnostics.recombination_fallback_used = true;
        diagnostics.recombination_accepted = 1;
        diagnostics.selected_hypothesis_id = Some(hypothesis.hypothesis_id.clone());
        diagnostics.selected_target = Some(hypothesis.suggested_target.clone());
        diagnostics.selected_kind = Some(hypothesis.suggested_mutation_kind.clone());
        diagnostics.selected_risk = Some(hypothesis.estimated_risk);
        diagnostics.recombination_fallback_reason = Some(format!(
            "selected useful recombined hypothesis with class={} target={} kind={}",
            mutation_class_label(class),
            hypothesis.suggested_target,
            hypothesis.suggested_mutation_kind
        ));
        return (Some(hypothesis), diagnostics);
    }

    diagnostics.recombination_fallback_reason = Some(
        "no recombined hypothesis satisfied task target/kind/risk/class constraints".to_string(),
    );
    (None, diagnostics)
}

fn mutation_kind_from_label(label: &str) -> Option<MutationKind> {
    match label {
        "addunittest" => Some(MutationKind::AddUnitTest),
        "addreplayassertion" => Some(MutationKind::AddReplayAssertion),
        "addlearningsummaryfield" => Some(MutationKind::AddLearningSummaryField),
        "addmetricupdate" => Some(MutationKind::AddMetricUpdate),
        _ => None,
    }
}

fn is_forbidden_target(file: &str) -> bool {
    file.starts_with("src/core/")
        || file == "src/main.rs"
        || file == "src/lib.rs"
        || file == "Cargo.toml"
        || file.ends_with("/Cargo.toml")
}
