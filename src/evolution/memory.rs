use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::contracts::{
    sha256_digest, tail, EvolutionLogEntry, EvolutionStatus, MutationContract, SandboxResult,
};
use crate::evolution::scorer::EvolutionScore;

pub const DEFAULT_EVOLUTION_LOG_PATH: &str = "memory/evolution.jsonl";
pub const DEFAULT_CANDIDATE_DIR: &str = "memory/candidates";
pub const DEFAULT_REPLAY_DIR: &str = "memory/replays";
pub const CANDIDATE_THRESHOLD: f32 = 5.0;
pub const PROMOTION_THRESHOLD: f32 = 7.0;
pub const PROMOTION_RISK_LIMIT: f32 = 0.25;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateSummary {
    pub run_id: String,
    pub mutation_id: String,
    pub status: EvolutionStatus,
    pub target_file: String,
    pub mutation_kind: String,
    pub risk: f32,
    pub score: f32,
    pub cargo_check_ok: bool,
    pub cargo_test_ok: bool,
    pub cargo_run_ok: bool,
    pub stdout_digest: String,
    pub stderr_digest: String,
    pub stderr_tail: String,
    pub timestamp_unix: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayResult {
    pub run_id: String,
    pub replay_status: EvolutionStatus,
    pub score: f32,
    pub matches_stored_summary: bool,
    pub cargo_check_ok: bool,
    pub cargo_test_ok: bool,
    pub cargo_run_ok: bool,
    pub stdout_digest: String,
    pub stderr_digest: String,
    pub stderr_tail: String,
    pub sandbox_destroyed: bool,
    pub timestamp_unix: u64,
}

pub fn record_evolution(entry: &EvolutionLogEntry) -> Result<(), String> {
    append_jsonl(DEFAULT_EVOLUTION_LOG_PATH, entry)
}

pub fn build_log_entry(
    run_id: String,
    mutation: &MutationContract,
    score: &EvolutionScore,
    sandbox: &SandboxResult,
    retained_in_core: bool,
    sandbox_destroyed: bool,
) -> EvolutionLogEntry {
    build_log_entry_with_plan(
        run_id,
        None,
        None,
        None,
        Vec::new(),
        mutation,
        score,
        sandbox,
        retained_in_core,
        sandbox_destroyed,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_log_entry_with_plan(
    run_id: String,
    plan_id: Option<String>,
    hypothesis_id: Option<String>,
    objective: Option<String>,
    graph_evidence: Vec<String>,
    mutation: &MutationContract,
    score: &EvolutionScore,
    sandbox: &SandboxResult,
    retained_in_core: bool,
    sandbox_destroyed: bool,
) -> EvolutionLogEntry {
    let stdout = combined_stdout(sandbox);
    let stderr = combined_stderr(sandbox);
    let status = if retained_in_core {
        EvolutionStatus::Promoted
    } else if score.score >= CANDIDATE_THRESHOLD && score.accepted {
        EvolutionStatus::Candidate
    } else if score.accepted {
        EvolutionStatus::Passed
    } else {
        EvolutionStatus::Failed
    };

    EvolutionLogEntry {
        run_id,
        plan_id,
        hypothesis_id,
        objective,
        graph_evidence,
        mutation_id: mutation.id.clone(),
        status,
        target_file: mutation.target_file.clone(),
        mutation_kind: format!("{:?}", mutation.kind).to_ascii_lowercase(),
        risk: mutation.risk,
        score: score.score,
        cargo_check_ok: score.check_passed,
        cargo_test_ok: score.test_passed,
        cargo_run_ok: score.run_passed,
        retained_in_core,
        sandbox_destroyed,
        stdout_digest: sha256_digest(&stdout),
        stderr_digest: sha256_digest(&stderr),
        stderr_tail: tail(&stderr, 1200),
        timestamp_unix: now_unix(),
    }
}

pub fn maybe_store_candidate(
    memory_root: &str,
    entry: &EvolutionLogEntry,
    mutation: &MutationContract,
) -> Result<bool, String> {
    if entry.score < CANDIDATE_THRESHOLD || entry.status == EvolutionStatus::Failed {
        return Ok(false);
    }
    store_candidate(memory_root, entry, mutation)?;
    Ok(true)
}

pub fn store_candidate(
    memory_root: &str,
    entry: &EvolutionLogEntry,
    mutation: &MutationContract,
) -> Result<(), String> {
    let dir = Path::new(memory_root).join("candidates");
    fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create candidate directory: {error}"))?;
    write_json(
        dir.join(format!("{}.mutation.json", entry.run_id)),
        mutation,
    )?;
    write_json(
        dir.join(format!("{}.summary.json", entry.run_id)),
        &CandidateSummary::from(entry),
    )
}

pub fn load_candidate(memory_root: &str, run_id: &str) -> Result<MutationContract, String> {
    let path = Path::new(memory_root)
        .join("candidates")
        .join(format!("{run_id}.mutation.json"));
    let contents =
        fs::read_to_string(&path).map_err(|error| format!("failed to read candidate: {error}"))?;
    serde_json::from_str(&contents).map_err(|error| format!("failed to parse candidate: {error}"))
}

pub fn load_candidate_summary(memory_root: &str, run_id: &str) -> Result<CandidateSummary, String> {
    let path = Path::new(memory_root)
        .join("candidates")
        .join(format!("{run_id}.summary.json"));
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read candidate summary: {error}"))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse candidate summary: {error}"))
}

pub fn list_candidate_summaries(memory_root: &str) -> Result<Vec<CandidateSummary>, String> {
    let dir = Path::new(memory_root).join("candidates");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut summaries: Vec<CandidateSummary> = Vec::new();
    for entry in
        fs::read_dir(&dir).map_err(|error| format!("failed to read candidates: {error}"))?
    {
        let entry = entry.map_err(|error| format!("failed to read candidate entry: {error}"))?;
        let path = entry.path();
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".summary.json"))
        {
            continue;
        }
        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read candidate summary: {error}"))?;
        summaries.push(
            serde_json::from_str(&contents)
                .map_err(|error| format!("failed to parse candidate summary: {error}"))?,
        );
    }
    summaries.sort_by(|left, right| left.run_id.cmp(&right.run_id));
    Ok(summaries)
}

pub fn store_replay_result(
    memory_root: &str,
    run_id: &str,
    result: &ReplayResult,
) -> Result<(), String> {
    let dir = Path::new(memory_root).join("replays");
    fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create replay directory: {error}"))?;
    write_json(dir.join(format!("{run_id}.json")), result)
}

pub fn append_jsonl(path: impl AsRef<Path>, value: &impl Serialize) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create evolution log directory: {error}"))?;
    }
    let line = serde_json::to_string(value)
        .map_err(|error| format!("failed to serialize evolution log entry: {error}"))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("failed to open evolution log: {error}"))?;
    writeln!(file, "{line}").map_err(|error| format!("failed to write evolution log: {error}"))?;
    Ok(())
}

pub fn write_json(path: impl AsRef<Path>, value: &impl Serialize) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create json directory: {error}"))?;
    }
    let contents = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize json: {error}"))?;
    fs::write(path, contents).map_err(|error| format!("failed to write json: {error}"))
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

pub fn new_run_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("run-{millis}-{}", std::process::id())
}

pub fn combined_stdout(sandbox: &SandboxResult) -> String {
    combine_outputs(sandbox, true)
}

pub fn combined_stderr(sandbox: &SandboxResult) -> String {
    combine_outputs(sandbox, false)
}

fn combine_outputs(sandbox: &SandboxResult, stdout: bool) -> String {
    let mut parts = Vec::new();
    parts.push(if stdout {
        sandbox.check.stdout.clone()
    } else {
        sandbox.check.stderr.clone()
    });
    if let Some(test) = &sandbox.test {
        parts.push(if stdout {
            test.stdout.clone()
        } else {
            test.stderr.clone()
        });
    }
    if let Some(run) = &sandbox.run {
        parts.push(if stdout {
            run.stdout.clone()
        } else {
            run.stderr.clone()
        });
    }
    parts.join("\n")
}

impl From<&EvolutionLogEntry> for CandidateSummary {
    fn from(entry: &EvolutionLogEntry) -> Self {
        Self {
            run_id: entry.run_id.clone(),
            mutation_id: entry.mutation_id.clone(),
            status: entry.status,
            target_file: entry.target_file.clone(),
            mutation_kind: entry.mutation_kind.clone(),
            risk: entry.risk,
            score: entry.score,
            cargo_check_ok: entry.cargo_check_ok,
            cargo_test_ok: entry.cargo_test_ok,
            cargo_run_ok: entry.cargo_run_ok,
            stdout_digest: entry.stdout_digest.clone(),
            stderr_digest: entry.stderr_digest.clone(),
            stderr_tail: entry.stderr_tail.clone(),
            timestamp_unix: entry.timestamp_unix,
        }
    }
}
