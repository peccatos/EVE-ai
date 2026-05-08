use std::path::Path;

use crate::contracts::RuntimeServiceMetadata;
use crate::evolution::memory;

pub fn build_runtime_service_metadata(memory_root: &str) -> Result<RuntimeServiceMetadata, String> {
    let metadata = RuntimeServiceMetadata {
        service_name: "eva-runtime".to_string(),
        mode: "local_operator".to_string(),
        daemonized: false,
        attach_supported: true,
        watch_supported: true,
        status_supported: true,
        network_required: false,
        network_push_allowed: false,
        auto_restart_allowed: false,
        pid_tracking: "metadata_only".to_string(),
        systemd_install: "not_performed".to_string(),
        external_side_effects: false,
    };
    memory::write_json(
        Path::new(memory_root)
            .join("runtime_service")
            .join("eva-runtime.json"),
        &metadata,
    )?;
    Ok(metadata)
}

pub fn print_runtime_service(memory_root: &str) -> Result<String, String> {
    serde_json::to_string_pretty(&build_runtime_service_metadata(memory_root)?)
        .map_err(|error| format!("failed to serialize runtime service metadata: {error}"))
}
