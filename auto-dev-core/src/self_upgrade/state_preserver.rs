//! State preservation across upgrades

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use crate::info;

/// Preserves application state across upgrades
pub struct StatePreserver {
    state_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeState {
    pub timestamp: String,
    pub version: String,
    pub active_tasks: Vec<String>,
    pub config: Value,
    pub environment: std::collections::HashMap<String, String>,
}

impl StatePreserver {
    pub fn new(state_dir: PathBuf) -> Self {
        Self { state_dir }
    }

    /// Save current application state
    pub async fn save_state(&self) -> Result<Value> {
        info!("Saving application state");

        std::fs::create_dir_all(&self.state_dir)?;

        let state = UpgradeState {
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            active_tasks: self.get_active_tasks(),
            config: self.load_current_config()?,
            environment: std::env::vars().collect(),
        };

        let state_value = serde_json::to_value(&state)?;
        let state_file = self.state_dir.join("current_state.json");
        std::fs::write(&state_file, serde_json::to_string_pretty(&state_value)?)?;

        Ok(state_value)
    }

    /// Restore state after upgrade
    pub async fn restore_state(&self, state: Value) -> Result<()> {
        info!("Restoring application state");

        let upgrade_state: UpgradeState = serde_json::from_value(state)?;

        // Restore environment variables
        for (key, value) in upgrade_state.environment {
            if !key.starts_with("CARGO_") && !key.starts_with("RUST_") {
                unsafe {
                    std::env::set_var(key, value);
                }
            }
        }

        info!(
            "State restored from version {} at {}",
            upgrade_state.version, upgrade_state.timestamp
        );

        Ok(())
    }

    fn get_active_tasks(&self) -> Vec<String> {
        // TODO: Get actual active tasks from the system
        Vec::new()
    }

    fn load_current_config(&self) -> Result<Value> {
        let config_path = PathBuf::from(".auto-dev/config.toml");
        if config_path.exists() {
            let config_str = std::fs::read_to_string(config_path)?;
            let config: Value = toml::from_str(&config_str)?;
            Ok(config)
        } else {
            Ok(Value::Object(serde_json::Map::new()))
        }
    }
}
