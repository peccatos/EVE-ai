use crate::contracts::CapabilityPolicy;
use crate::evolution::memory;

pub fn build_capability_policy() -> CapabilityPolicy {
    CapabilityPolicy {
        generated_at: memory::now_unix(),
        auto_promote_allowed: false,
        network_push_allowed: false,
        merge_allowed: false,
        external_repo_mutation_allowed: false,
        self_apply_allowed: false,
        source_mutation_without_approval_allowed: false,
        metadata_generation_allowed: true,
        local_read_only_inspection_allowed: true,
        sandboxed_validation_allowed_when_isolated: true,
        denied_capabilities: vec![
            "auto_promote".to_string(),
            "network_push".to_string(),
            "merge".to_string(),
            "external_repo_mutation".to_string(),
            "self_apply".to_string(),
            "source_mutation_without_approval".to_string(),
        ],
        allowed_capabilities: vec![
            "metadata_generation".to_string(),
            "local_read_only_inspection".to_string(),
            "sandboxed_validation_when_isolated".to_string(),
        ],
    }
}

pub fn print_capability_policy() -> Result<String, String> {
    serde_json::to_string_pretty(&build_capability_policy())
        .map_err(|error| format!("failed to serialize capability policy: {error}"))
}
