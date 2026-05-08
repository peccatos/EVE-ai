use crate::contracts::{FuturePhaseEntry, FuturePhaseRegistry};
use crate::evolution::memory;

pub fn build_future_phase_registry() -> FuturePhaseRegistry {
    FuturePhaseRegistry {
        generated_at: memory::now_unix(),
        entries: vec![
            FuturePhaseEntry {
                phase: "10.0".to_string(),
                name: "CI/PR Integration Runtime".to_string(),
                status: "completed_by_phase_13_0x".to_string(),
                allowed_now: false,
                reason: "merged into local operations integration runtime".to_string(),
            },
            FuturePhaseEntry {
                phase: "11.0".to_string(),
                name: "Daemonized Operator Service".to_string(),
                status: "completed_by_phase_13_0x".to_string(),
                allowed_now: false,
                reason: "merged into local operations integration runtime".to_string(),
            },
            FuturePhaseEntry {
                phase: "12.0".to_string(),
                name: "External Repository Patch Pipeline".to_string(),
                status: "completed_by_phase_13_0x".to_string(),
                allowed_now: false,
                reason: "merged into local operations integration runtime".to_string(),
            },
            FuturePhaseEntry {
                phase: "13.0".to_string(),
                name: "Controlled Self-Modification Review Runtime".to_string(),
                status: "completed_by_phase_13_0x".to_string(),
                allowed_now: false,
                reason: "absorbed by the combined Phase 13.0X operations layer".to_string(),
            },
            FuturePhaseEntry {
                phase: "14.0".to_string(),
                name: "Stable Local Release Candidate Flow".to_string(),
                status: "planned".to_string(),
                allowed_now: false,
                reason: "requires additional operator validation hardening".to_string(),
            },
            FuturePhaseEntry {
                phase: "15.0".to_string(),
                name: "Local CI Runner / Matrix Validation".to_string(),
                status: "planned".to_string(),
                allowed_now: false,
                reason: "requires stable local release candidate flow first".to_string(),
            },
            FuturePhaseEntry {
                phase: "16.0".to_string(),
                name: "External Repo Patch Dry-Run Runtime".to_string(),
                status: "planned".to_string(),
                allowed_now: false,
                reason: "requires richer external patch validation metadata".to_string(),
            },
            FuturePhaseEntry {
                phase: "17.0".to_string(),
                name: "Governance-backed PR Export".to_string(),
                status: "planned".to_string(),
                allowed_now: false,
                reason: "requires stronger release package/operator workflow evidence".to_string(),
            },
            FuturePhaseEntry {
                phase: "18.0".to_string(),
                name: "Controlled Daemon Mode".to_string(),
                status: "planned".to_string(),
                allowed_now: false,
                reason: "requires bounded operator service proofs and release stability"
                    .to_string(),
            },
        ],
    }
}

pub fn print_future_phases() -> String {
    build_future_phase_registry()
        .entries
        .iter()
        .map(|entry| {
            format!(
                "Phase {}: {} status={} allowed_now={} reason={}",
                entry.phase, entry.name, entry.status, entry.allowed_now, entry.reason
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn print_future_phases_json() -> Result<String, String> {
    serde_json::to_string_pretty(&build_future_phase_registry())
        .map_err(|error| format!("failed to serialize future phase registry: {error}"))
}
