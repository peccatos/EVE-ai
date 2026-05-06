use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::contracts::{MutationContract, MutationKind};
use crate::evolution::{autonomy_status, load_report_json};
use crate::promotion::gate::check_promotion_gate;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateReview {
    pub run_id: String,
    pub mutation_kind: String,
    pub target_file: String,
    pub score: f32,
    pub risk: f32,
    pub useful_change: bool,
    pub replay_status: String,
    pub report_path: String,
    pub promotion_allowed: bool,
    pub russian_summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateReviewReport {
    pub run_id: String,
    pub promotion_allowed: bool,
    pub replay_status: String,
    pub recommendation_ru: String,
    pub markdown_path: String,
}

pub fn review_candidate(
    project_root: &str,
    memory_root: &str,
    run_id: &str,
) -> Result<CandidateReview, String> {
    let summary = crate::evolution::memory::load_candidate_summary(memory_root, run_id)?;
    let mutation = crate::evolution::memory::load_candidate(memory_root, run_id)?;
    let replay_status = replay_status(memory_root, run_id)?;
    let report_path = Path::new(memory_root)
        .join("reports")
        .join(format!("{run_id}.ru.md"));
    let promotion_allowed = promotion_allowed(
        project_root,
        memory_root,
        &summary,
        &mutation,
        &replay_status,
    )?;
    let review = CandidateReview {
        run_id: run_id.to_string(),
        mutation_kind: summary.mutation_kind.clone(),
        target_file: summary.target_file.clone(),
        score: summary.score,
        risk: summary.risk,
        useful_change: summary.useful_change,
        replay_status: replay_status.clone(),
        report_path: report_path.display().to_string(),
        promotion_allowed,
        russian_summary: russian_summary(&summary, &mutation, &replay_status, promotion_allowed),
    };
    write_review_report(memory_root, &review)?;
    Ok(review)
}

pub fn candidate_diff(memory_root: &str, run_id: &str) -> Result<String, String> {
    let mutation = crate::evolution::memory::load_candidate(memory_root, run_id)?;
    Ok(render_diff(&mutation))
}

pub fn review_report_markdown(memory_root: &str, run_id: &str) -> Result<String, String> {
    let path = Path::new(memory_root)
        .join("reviews")
        .join(format!("{run_id}.ru.md"));
    fs::read_to_string(path).map_err(|error| format!("failed to read review report: {error}"))
}

fn write_review_report(memory_root: &str, review: &CandidateReview) -> Result<(), String> {
    let dir = Path::new(memory_root).join("reviews");
    fs::create_dir_all(&dir).map_err(|error| format!("failed to create reviews dir: {error}"))?;
    let markdown_path = dir.join(format!("{}.ru.md", review.run_id));
    let markdown = render_review_markdown(review);
    fs::write(&markdown_path, markdown)
        .map_err(|error| format!("failed to write review markdown: {error}"))?;
    let review_report = CandidateReviewReport {
        run_id: review.run_id.clone(),
        promotion_allowed: review.promotion_allowed,
        replay_status: review.replay_status.clone(),
        recommendation_ru: review.russian_summary.clone(),
        markdown_path: markdown_path.display().to_string(),
    };
    crate::evolution::memory::write_json(
        dir.join(format!("{}.review.json", review.run_id)),
        &review_report,
    )
}

fn promotion_allowed(
    project_root: &str,
    memory_root: &str,
    summary: &crate::evolution::CandidateSummary,
    mutation: &MutationContract,
    replay_status: &str,
) -> Result<bool, String> {
    if !summary.useful_change {
        return Ok(false);
    }
    if replay_status != "ok" {
        return Ok(false);
    }
    if matches!(mutation.kind, MutationKind::AppendComment) {
        return Ok(false);
    }
    let gate = check_promotion_gate(mutation, summary.score);
    if !gate.allowed {
        return Ok(false);
    }
    let autonomy = autonomy_status(project_root, memory_root)?;
    Ok(autonomy.blockers.is_empty())
}

fn replay_status(memory_root: &str, run_id: &str) -> Result<String, String> {
    if let Ok(report) = load_report_json(memory_root, run_id) {
        return Ok(report.replay_status);
    }
    let path = Path::new(memory_root)
        .join("replays")
        .join(format!("{run_id}.json"));
    if !path.exists() {
        return Ok("not_run".to_string());
    }
    let contents =
        fs::read_to_string(path).map_err(|error| format!("failed to read replay: {error}"))?;
    let replay: crate::evolution::ReplayResult = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse replay: {error}"))?;
    let passed = replay.matches_stored_summary
        && replay.cargo_check_ok
        && replay.cargo_test_ok
        && replay.cargo_run_ok
        && replay.replay_status != crate::contracts::EvolutionStatus::Failed;
    Ok(if passed { "ok" } else { "failed" }.to_string())
}

fn render_diff(mutation: &MutationContract) -> String {
    match mutation.kind {
        MutationKind::AddUnitTest
        | MutationKind::AddReplayAssertion
        | MutationKind::AppendComment => {
            format!(
                "target: {}\nkind: {:?}\nappend:\n{}\n",
                mutation.target_file,
                mutation.kind,
                mutation.append.as_deref().unwrap_or("(none)")
            )
        }
        MutationKind::ReplaceText
        | MutationKind::ParameterTune
        | MutationKind::AddLearningSummaryField
        | MutationKind::AddMetricUpdate => format!(
            "target: {}\nkind: {:?}\nsearch:\n{}\n\nreplace:\n{}\n",
            mutation.target_file,
            mutation.kind,
            mutation.search.as_deref().unwrap_or("(none)"),
            mutation.replace.as_deref().unwrap_or("(none)")
        ),
        MutationKind::AddTestSkeleton | MutationKind::AddMetricField => format!(
            "target: {}\nkind: {:?}\npayload:\n{}\n",
            mutation.target_file,
            mutation.kind,
            mutation.append.as_deref().unwrap_or("(none)")
        ),
    }
}

fn russian_summary(
    summary: &crate::evolution::CandidateSummary,
    mutation: &MutationContract,
    replay_status: &str,
    promotion_allowed: bool,
) -> String {
    format!(
        "Кандидат {} меняет {} через {:?}. Score {:.1}, risk {:.2}, useful={}, replay={}, promotion_ready={}.",
        summary.run_id,
        summary.target_file,
        mutation.kind,
        summary.score,
        summary.risk,
        summary.useful_change,
        replay_status,
        promotion_allowed
    )
}

fn render_review_markdown(review: &CandidateReview) -> String {
    format!(
        "# Review EVA\n\n## Кандидат\nrun_id: {}\nkind: {}\nfile: {}\n\n## Что изменено\n{}\n\n## Почему полезно\nuseful_change={}\nscore={:.1}\n\n## Проверки\nrisk={:.2}\nreport={}\n\n## Replay\nstatus={}\n\n## Риск\nОценка риска {:.2}\n\n## Готовность к promotion\n{}\n\n## Рекомендация EVA\n{}\n",
        review.run_id,
        review.mutation_kind,
        review.target_file,
        review.target_file,
        review.useful_change,
        review.score,
        review.risk,
        review.report_path,
        review.replay_status,
        review.risk,
        if review.promotion_allowed { "готов" } else { "не готов" },
        review.russian_summary
    )
}
