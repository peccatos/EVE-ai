use std::process::Command;

use crate::contracts::SandboxResult;
use crate::evolution::{memory, mutator, scorer};
use crate::promotion::gate::check_promotion_gate;
use crate::sandbox::{manager, runner, snapshot};

pub fn list_candidates(memory_root: &str) -> Result<String, String> {
    let summaries = memory::list_candidate_summaries(memory_root)?;
    if summaries.is_empty() {
        return Ok("(none)".to_string());
    }
    Ok(summaries
        .iter()
        .map(|summary| {
            format!(
                "{} score={:.1} risk={:.2} target={}",
                summary.run_id, summary.score, summary.risk, summary.target_file
            )
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

pub fn replay_candidate(project_root: &str, memory_root: &str, run_id: &str) -> Result<(), String> {
    let mutation = memory::load_candidate(memory_root, run_id)?;
    let summary = memory::load_candidate_summary(memory_root, run_id)?;
    let sandbox_path = manager::create_sandbox_path();
    snapshot::copy_project(project_root, &sandbox_path)?;
    let result = replay_in_sandbox(&sandbox_path, &mutation);
    let cleanup = manager::destroy_sandbox(&sandbox_path);
    let sandbox_destroyed = cleanup.is_ok();
    let sandbox = result?;
    cleanup?;

    let test_ref = sandbox
        .test
        .as_ref()
        .ok_or_else(|| "replay did not run cargo test".to_string())?;
    let score = scorer::score_cycle(&sandbox.check, test_ref, sandbox.run.as_ref());
    let stdout = memory::combined_stdout(&sandbox);
    let stderr = memory::combined_stderr(&sandbox);
    let replay = memory::ReplayResult {
        run_id: run_id.to_string(),
        replay_status: if score.score >= memory::CANDIDATE_THRESHOLD && score.accepted {
            crate::contracts::EvolutionStatus::Candidate
        } else if score.accepted {
            crate::contracts::EvolutionStatus::Passed
        } else {
            crate::contracts::EvolutionStatus::Failed
        },
        matches_stored_summary: (score.score - summary.score).abs() < f32::EPSILON
            && score.check_passed == summary.cargo_check_ok
            && score.test_passed == summary.cargo_test_ok
            && score.run_passed == summary.cargo_run_ok,
        score: score.score,
        cargo_check_ok: score.check_passed,
        cargo_test_ok: score.test_passed,
        cargo_run_ok: score.run_passed,
        stdout_digest: crate::contracts::sha256_digest(&stdout),
        stderr_digest: crate::contracts::sha256_digest(&stderr),
        stderr_tail: crate::contracts::tail(&stderr, 1200),
        sandbox_destroyed,
        timestamp_unix: memory::now_unix(),
    };
    crate::evolution::metrics::update_metrics_after_replay(memory_root, &replay)?;
    memory::store_replay_result(memory_root, run_id, &replay)
}

pub fn promote_candidate(
    project_root: &str,
    memory_root: &str,
    run_id: &str,
) -> Result<(), String> {
    let mutation = memory::load_candidate(memory_root, run_id)?;
    let summary = memory::load_candidate_summary(memory_root, run_id)?;
    let decision = check_promotion_gate(&mutation, summary.score);
    if !decision.allowed {
        return Err(decision.reason);
    }

    let target_path = std::path::Path::new(project_root).join(&mutation.target_file);
    let original = std::fs::read_to_string(&target_path)
        .map_err(|error| format!("failed to backup promotion target: {error}"))?;

    mutator::apply_mutation(project_root, &mutation)?;
    let validation = validate_promoted_project(project_root);
    let (check, test) = match validation {
        Ok(result) => result,
        Err(error) => {
            std::fs::write(&target_path, original)
                .map_err(|restore_error| format!("{error}; restore failed: {restore_error}"))?;
            return Err(error);
        }
    };
    let sandbox = SandboxResult {
        sandbox_path: project_root.to_string(),
        check,
        test: Some(test),
        run: None,
    };
    let test_ref = sandbox
        .test
        .as_ref()
        .ok_or_else(|| "promotion did not run cargo test".to_string())?;
    let score = scorer::score_cycle(&sandbox.check, test_ref, None);
    if !score.accepted {
        return Err("promotion validation failed".to_string());
    }
    let entry = memory::build_log_entry(
        memory::new_run_id(),
        &mutation,
        &score,
        &sandbox,
        true,
        false,
    );
    memory::append_jsonl(
        std::path::Path::new(memory_root).join("evolution.jsonl"),
        &entry,
    )?;
    crate::evolution::metrics::update_metrics_after_log(memory_root, &entry)?;
    Ok(())
}

fn validate_promoted_project(
    project_root: &str,
) -> Result<
    (
        crate::contracts::CommandResult,
        crate::contracts::CommandResult,
    ),
    String,
> {
    run_host_command(project_root, "cargo", &["fmt"])?;
    let check = run_host_command(project_root, "cargo", &["check"])?;
    let test = run_host_command(project_root, "cargo", &["test"])?;
    Ok((check, test))
}

fn replay_in_sandbox(
    sandbox_path: &str,
    mutation: &crate::contracts::MutationContract,
) -> Result<SandboxResult, String> {
    mutator::apply_mutation(sandbox_path, mutation)?;
    let check = runner::run_cargo_check(sandbox_path);
    let test = if check.success {
        Some(runner::run_cargo_test(sandbox_path))
    } else {
        None
    };
    let run = if test.as_ref().is_some_and(|result| result.success) {
        Some(runner::run_cargo_run(sandbox_path))
    } else {
        None
    };
    Ok(SandboxResult {
        sandbox_path: sandbox_path.to_string(),
        check,
        test,
        run,
    })
}

fn run_host_command(
    project_root: &str,
    bin: &str,
    args: &[&str],
) -> Result<crate::contracts::CommandResult, String> {
    let start = std::time::Instant::now();
    let output = Command::new(bin)
        .args(args)
        .current_dir(project_root)
        .output()
        .map_err(|error| format!("failed to run {bin}: {error}"))?;
    let result = crate::contracts::CommandResult {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        duration_ms: start.elapsed().as_millis(),
    };
    if result.success {
        Ok(result)
    } else {
        Err(format!(
            "{bin} {} failed: {}",
            args.join(" "),
            result.stderr
        ))
    }
}
