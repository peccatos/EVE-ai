use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::contracts::{ApprovalStatus, OperatorApprovalRecord, PromotionQueueItem};
use crate::evolution::{candidate_lifecycle, memory, refresh_promotion_queue};

const APPROVAL_LOG: &str = "approval_log.jsonl";
const REJECTION_LOG: &str = "rejections.jsonl";
const DEFERRED_LOG: &str = "deferred.jsonl";

pub fn approve_candidate(
    project_root: &str,
    memory_root: &str,
    run_id: &str,
    reason: &str,
) -> Result<OperatorApprovalRecord, String> {
    let _ = refresh_promotion_queue(project_root, memory_root);
    let item = candidate_lifecycle(project_root, memory_root, run_id)?;
    validate_approval_candidate(memory_root, &item)?;
    upsert_append_only(memory_root, &item, "approve", reason, APPROVAL_LOG)
}

pub fn reject_candidate(
    project_root: &str,
    memory_root: &str,
    run_id: &str,
    reason: &str,
) -> Result<OperatorApprovalRecord, String> {
    let _ = refresh_promotion_queue(project_root, memory_root);
    let item = candidate_lifecycle(project_root, memory_root, run_id)?;
    upsert_append_only(memory_root, &item, "reject", reason, REJECTION_LOG)
}

pub fn defer_candidate(
    project_root: &str,
    memory_root: &str,
    run_id: &str,
    reason: &str,
) -> Result<OperatorApprovalRecord, String> {
    let _ = refresh_promotion_queue(project_root, memory_root);
    let item = candidate_lifecycle(project_root, memory_root, run_id)?;
    upsert_append_only(memory_root, &item, "defer", reason, DEFERRED_LOG)
}

pub fn approval_status(
    project_root: &str,
    memory_root: &str,
    run_id: &str,
) -> Result<ApprovalStatus, String> {
    let _ = refresh_promotion_queue(project_root, memory_root);
    let item = candidate_lifecycle(project_root, memory_root, run_id)?;
    let latest_record = latest_record_for_run(memory_root, run_id)?;
    Ok(ApprovalStatus {
        run_id: run_id.to_string(),
        current_decision: latest_record
            .as_ref()
            .map(|record| record.decision.clone())
            .unwrap_or_else(|| "none".to_string()),
        promotable: latest_record
            .as_ref()
            .is_some_and(|record| record.decision == "approve" && item.lifecycle_state == "ready"),
        latest_record,
    })
}

pub fn approval_log(memory_root: &str) -> Result<Vec<OperatorApprovalRecord>, String> {
    let mut records = read_records(memory_root, APPROVAL_LOG)?;
    records.extend(read_records(memory_root, REJECTION_LOG)?);
    records.extend(read_records(memory_root, DEFERRED_LOG)?);
    records.sort_by(|left, right| {
        left.run_id
            .cmp(&right.run_id)
            .then_with(|| left.created_at.cmp(&right.created_at))
            .then_with(|| left.decision.cmp(&right.decision))
    });
    Ok(records)
}

pub fn latest_record_for_run(
    memory_root: &str,
    run_id: &str,
) -> Result<Option<OperatorApprovalRecord>, String> {
    let records = approval_log(memory_root)?;
    Ok(records
        .into_iter()
        .filter(|record| record.run_id == run_id)
        .max_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.decision.cmp(&right.decision))
        }))
}

pub fn latest_decisions(memory_root: &str) -> Result<Vec<OperatorApprovalRecord>, String> {
    let mut map = std::collections::BTreeMap::<String, OperatorApprovalRecord>::new();
    for record in approval_log(memory_root)? {
        let replace = match map.get(&record.run_id) {
            Some(existing) => {
                record.created_at > existing.created_at
                    || (record.created_at == existing.created_at
                        && record.decision > existing.decision)
            }
            None => true,
        };
        if replace {
            map.insert(record.run_id.clone(), record);
        }
    }
    Ok(map.into_values().collect())
}

pub fn record_promotion_event(
    project_root: &str,
    memory_root: &str,
    run_id: &str,
    reason: &str,
) -> Result<OperatorApprovalRecord, String> {
    let _ = refresh_promotion_queue(project_root, memory_root);
    let item = candidate_lifecycle(project_root, memory_root, run_id)?;
    upsert_append_only(memory_root, &item, "promoted", reason, APPROVAL_LOG)
}

fn validate_approval_candidate(memory_root: &str, item: &PromotionQueueItem) -> Result<(), String> {
    if item.mutation_class == "cosmetic" {
        return Err("cannot approve cosmetic candidate".to_string());
    }
    if item.mutation_class == "unsafe" {
        return Err("cannot approve unsafe candidate".to_string());
    }
    if item.mutation_class == "legacy" || item.mutation_class.is_empty() {
        return Err("cannot approve legacy candidate".to_string());
    }
    if item.lifecycle_state == "already_promoted" {
        return Err("cannot approve already promoted candidate".to_string());
    }
    if item.replay_status != "ok" {
        return Err("cannot approve candidate without replay ok".to_string());
    }
    if item.risk > memory::PROMOTION_RISK_LIMIT {
        return Err("cannot approve candidate above risk threshold".to_string());
    }
    if item.report_path.is_empty() || !Path::new(&item.report_path).exists() {
        return Err("cannot approve candidate missing report path".to_string());
    }
    if item
        .promotion_blockers
        .iter()
        .any(|blocker| blocker == "forbidden_target")
    {
        return Err("cannot approve candidate blocked by forbidden target".to_string());
    }
    let summary = memory::load_candidate_summary(memory_root, &item.run_id)?;
    if !summary.useful_change {
        return Err("cannot approve candidate with useful_change=false".to_string());
    }
    Ok(())
}

fn upsert_append_only(
    memory_root: &str,
    item: &PromotionQueueItem,
    decision: &str,
    reason: &str,
    file_name: &str,
) -> Result<OperatorApprovalRecord, String> {
    let existing = read_records(memory_root, file_name)?
        .into_iter()
        .find(|record| record.run_id == item.run_id && record.decision == decision);
    if let Some(record) = existing {
        return Ok(record);
    }
    let record = OperatorApprovalRecord {
        run_id: item.run_id.clone(),
        mutation_kind: item.mutation_kind.clone(),
        mutation_class: item.mutation_class.clone(),
        target_file: item.target_file.clone(),
        score: item.score,
        risk: item.risk,
        replay_status: item.replay_status.clone(),
        promotion_state: item.promotion_state.clone(),
        promotion_allowed: item.promotion_allowed,
        promotion_blockers: item.promotion_blockers.clone(),
        report_path: item.report_path.clone(),
        decision: decision.to_string(),
        reason: reason.to_string(),
        created_at: memory::now_unix(),
    };
    append_record(memory_root, file_name, &record)?;
    Ok(record)
}

fn append_record(
    memory_root: &str,
    file_name: &str,
    record: &OperatorApprovalRecord,
) -> Result<(), String> {
    let path = governance_file(memory_root, file_name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create governance dir: {error}"))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|error| format!("failed to open governance log: {error}"))?;
    writeln!(
        file,
        "{}",
        serde_json::to_string(record)
            .map_err(|error| format!("failed to serialize governance record: {error}"))?
    )
    .map_err(|error| format!("failed to append governance record: {error}"))
}

fn read_records(memory_root: &str, file_name: &str) -> Result<Vec<OperatorApprovalRecord>, String> {
    let path = governance_file(memory_root, file_name);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read governance log: {error}"))?;
    Ok(contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<OperatorApprovalRecord>(line).ok())
        .collect())
}

fn governance_file(memory_root: &str, file_name: &str) -> PathBuf {
    Path::new(memory_root).join("governance").join(file_name)
}
