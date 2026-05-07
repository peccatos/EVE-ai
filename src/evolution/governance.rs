use std::fs;
use std::path::Path;

use crate::contracts::{GovernanceStatus, GovernanceTrustGate, PromotionQueueItem};
use crate::evolution::{
    candidate_lifecycle, latest_decisions, latest_record_for_run, memory, promotion_ready_items,
    record_promotion_event, refresh_promotion_queue,
};
use crate::promotion::apply::promote_candidate;

pub fn governance_status(
    project_root: &str,
    memory_root: &str,
) -> Result<GovernanceStatus, String> {
    let decisions = latest_decisions(memory_root)?;
    let approved_count = decisions
        .iter()
        .filter(|record| record.decision == "approve")
        .count();
    let rejected_count = decisions
        .iter()
        .filter(|record| record.decision == "reject")
        .count();
    let deferred_count = decisions
        .iter()
        .filter(|record| record.decision == "defer")
        .count();
    let promotion_ready_approved_count = promotion_ready_approved(project_root, memory_root)?.len();
    Ok(GovernanceStatus {
        approved_count,
        rejected_count,
        deferred_count,
        promotion_ready_approved_count,
        auto_promote: false,
        operator_approval_required: true,
    })
}

pub fn promotion_ready_approved(
    project_root: &str,
    memory_root: &str,
) -> Result<Vec<PromotionQueueItem>, String> {
    let mut items = Vec::new();
    for item in promotion_ready_items(project_root, memory_root)? {
        let status = latest_record_for_run(memory_root, &item.run_id)?;
        if !status
            .as_ref()
            .is_some_and(|record| record.decision == "approve")
        {
            continue;
        }
        let gate = governance_trust_gate(project_root, memory_root, &item.run_id, false)?;
        if gate.allowed {
            items.push(item);
        }
    }
    items.sort_by(|left, right| left.run_id.cmp(&right.run_id));
    Ok(items)
}

pub fn governance_trust_gate(
    project_root: &str,
    memory_root: &str,
    run_id: &str,
    require_proof_snapshot: bool,
) -> Result<GovernanceTrustGate, String> {
    let _ = refresh_promotion_queue(project_root, memory_root);
    let item = candidate_lifecycle(project_root, memory_root, run_id)?;
    let latest = latest_record_for_run(memory_root, run_id)?;
    let mut blockers = Vec::new();

    match latest.as_ref().map(|record| record.decision.as_str()) {
        Some("approve") => {}
        Some("reject") => blockers.push("rejected_by_operator".to_string()),
        Some("defer") => blockers.push("deferred_by_operator".to_string()),
        Some("promoted") => blockers.push("already_promoted_by_governance".to_string()),
        _ => blockers.push("approval_missing".to_string()),
    }

    if item.replay_status != "ok" {
        blockers.push("replay_not_ok".to_string());
    }
    if item.mutation_class != "useful" {
        blockers.push("mutation_class_not_useful".to_string());
    }
    if item
        .promotion_blockers
        .iter()
        .any(|blocker| blocker == "forbidden_target")
    {
        blockers.push("forbidden_target".to_string());
    }
    if item.lifecycle_state == "already_promoted" {
        blockers.push("already_promoted".to_string());
    }
    if item.risk > memory::PROMOTION_RISK_LIMIT {
        blockers.push("risk_above_limit".to_string());
    }
    if require_proof_snapshot && latest_proof_snapshot_path(memory_root).is_none() {
        blockers.push("proof_snapshot_missing".to_string());
    }

    if let Some(approval) = latest
        .as_ref()
        .filter(|record| record.decision == "approve")
    {
        if approval.mutation_kind != item.mutation_kind
            || approval.mutation_class != item.mutation_class
            || approval.target_file != item.target_file
            || (approval.score - item.score).abs() > f32::EPSILON
            || (approval.risk - item.risk).abs() > f32::EPSILON
            || approval.replay_status != item.replay_status
            || approval.promotion_state != item.promotion_state
        {
            blockers.push("approval_stale".to_string());
        }
    }

    blockers.sort();
    blockers.dedup();
    let allowed = blockers.is_empty();
    Ok(GovernanceTrustGate {
        allowed,
        reason_ru: if allowed {
            "Governance trust gate разрешил ручной promotion.".to_string()
        } else {
            format!(
                "Governance trust gate заблокировал promotion: {}.",
                blockers.join(", ")
            )
        },
        blockers,
    })
}

pub fn promote_approved_candidate(
    project_root: &str,
    memory_root: &str,
    run_id: &str,
) -> Result<String, String> {
    let latest = latest_record_for_run(memory_root, run_id)?;
    if latest.is_none() {
        return Err("no approval exists for candidate".to_string());
    }
    match latest.as_ref().map(|record| record.decision.as_str()) {
        Some("reject") => return Err("candidate was rejected by operator".to_string()),
        Some("defer") => return Err("candidate was deferred by operator".to_string()),
        Some("approve") => {}
        Some("promoted") => return Err("candidate is already governed as promoted".to_string()),
        _ => return Err("no approval exists for candidate".to_string()),
    }

    let gate = governance_trust_gate(project_root, memory_root, run_id, false)?;
    if !gate.allowed {
        return Err(format!(
            "governance trust gate blocked promotion: {:?}",
            gate.blockers
        ));
    }

    promote_candidate(project_root, memory_root, run_id)?;
    refresh_promotion_queue(project_root, memory_root)?;
    let _ = record_promotion_event(
        project_root,
        memory_root,
        run_id,
        "manual promote-approved completed",
    )?;
    Ok("promotion_status: ok".to_string())
}

fn latest_proof_snapshot_path(memory_root: &str) -> Option<String> {
    let dir = Path::new(memory_root)
        .join("governance")
        .join("proof_snapshots");
    if !dir.exists() {
        return None;
    }
    let mut entries = fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect::<Vec<_>>();
    entries.sort();
    entries
        .last()
        .and_then(|path| path.to_str().map(str::to_string))
}
