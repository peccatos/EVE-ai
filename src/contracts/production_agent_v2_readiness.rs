use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProductionAgentV2Readiness {
    pub ci_proof_ok: bool,
    pub openai_adapter_ok: bool,
    pub rule_based_fallback_ok: bool,
    pub structured_proposals_ok: bool,
    pub proposal_preview_ok: bool,
    pub dry_run_ok: bool,
    pub code_edit_engine_ok: bool,
    pub repo_map_ok: bool,
    pub repo_aware_planner_ok: bool,
    pub task_outcome_memory_ok: bool,
    pub validation_ok: bool,
    pub tui_visibility_ok: bool,
    pub safety_policy_ok: bool,
    pub production_agent_v2_ready: bool,
    pub warnings: Vec<String>,
    pub blockers: Vec<String>,
}
