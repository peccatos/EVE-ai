use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RuntimeServiceMetadata {
    #[serde(default)]
    pub service_name: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub daemonized: bool,
    #[serde(default)]
    pub attach_supported: bool,
    #[serde(default)]
    pub watch_supported: bool,
    #[serde(default)]
    pub status_supported: bool,
    #[serde(default)]
    pub network_required: bool,
    #[serde(default)]
    pub network_push_allowed: bool,
    #[serde(default)]
    pub auto_restart_allowed: bool,
    #[serde(default)]
    pub pid_tracking: String,
    #[serde(default)]
    pub systemd_install: String,
    #[serde(default)]
    pub external_side_effects: bool,
}
