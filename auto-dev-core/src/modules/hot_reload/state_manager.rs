// State Manager - Handles state preservation during hot-reload

use super::{HotReloadError, HotReloadResult};
use crate::modules::{ModuleState, ModuleVersion, ModuleRuntime, loader::LoadedModule};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// A snapshot of module state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub module_id: String,
    pub version: StateVersion,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub state: ModuleState,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Version information for state compatibility
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub schema_version: u32,
}

impl StateVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            schema_version: 1,
        }
    }

    pub fn from_module_version(version: &ModuleVersion) -> Self {
        Self {
            major: version.major,
            minor: version.minor,
            patch: version.patch,
            schema_version: 1,
        }
    }

    pub fn is_compatible_with(&self, other: &StateVersion) -> bool {
        // Major version must match for compatibility
        self.major == other.major && self.schema_version == other.schema_version
    }
}

impl std::fmt::Display for StateVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Manages state snapshots and restoration
pub struct StateManager {
    snapshots: Arc<RwLock<HashMap<String, Vec<StateSnapshot>>>>,
    max_snapshots_per_module: usize,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            max_snapshots_per_module: 10,
        }
    }

    /// Create a snapshot of module state
    pub async fn create_snapshot(
        &self,
        module_id: &str,
        state: &ModuleState,
    ) -> HotReloadResult<StateSnapshot> {
        debug!("Creating state snapshot for module: {}", module_id);

        let snapshot = StateSnapshot {
            module_id: module_id.to_string(),
            version: StateVersion::from_module_version(&state.version),
            timestamp: chrono::Utc::now(),
            state: state.clone(),
            metadata: HashMap::new(),
        };

        // Store snapshot
        let mut snapshots = self.snapshots.write().await;
        let module_snapshots = snapshots.entry(module_id.to_string()).or_insert_with(Vec::new);
        
        // Add new snapshot
        module_snapshots.push(snapshot.clone());
        
        // Limit number of snapshots
        if module_snapshots.len() > self.max_snapshots_per_module {
            module_snapshots.remove(0);
        }

        info!(
            "Created snapshot for module {} at version {}",
            module_id, snapshot.version
        );

        Ok(snapshot)
    }

    /// Restore a module state from snapshot
    pub async fn restore_snapshot(
        &self,
        module_id: &str,
        snapshot: StateSnapshot,
        runtime: Arc<ModuleRuntime>,
    ) -> HotReloadResult<()> {
        debug!("Restoring state snapshot for module: {}", module_id);

        // Verify snapshot is for the correct module
        if snapshot.module_id != module_id {
            return Err(HotReloadError::VerificationFailed(
                format!(
                    "Snapshot module ID mismatch: expected {}, got {}",
                    module_id, snapshot.module_id
                ),
            ));
        }

        // Restore the state
        runtime
            .restore_module_state(module_id, snapshot.state)
            .await
            .map_err(|e| HotReloadError::MigrationFailed(e.to_string()))?;

        info!(
            "Restored snapshot for module {} from version {}",
            module_id, snapshot.version
        );

        Ok(())
    }

    /// Get the latest snapshot for a module
    pub async fn get_latest_snapshot(&self, module_id: &str) -> Option<StateSnapshot> {
        let snapshots = self.snapshots.read().await;
        snapshots
            .get(module_id)
            .and_then(|v| v.last())
            .cloned()
    }

    /// Get all snapshots for a module
    pub async fn get_snapshots(&self, module_id: &str) -> Vec<StateSnapshot> {
        let snapshots = self.snapshots.read().await;
        snapshots
            .get(module_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Clear snapshots for a module
    pub async fn clear_snapshots(&self, module_id: &str) {
        let mut snapshots = self.snapshots.write().await;
        snapshots.remove(module_id);
    }

    /// Extract state version from a ModuleState
    pub fn get_state_version(&self, state: &ModuleState) -> StateVersion {
        StateVersion::from_module_version(&state.version)
    }

    /// Extract module version from a LoadedModule
    pub fn get_module_version(&self, _module: &LoadedModule) -> StateVersion {
        // For now, return a default version. In a real implementation,
        // we would need to add public accessors to WasmModule and NativeModule
        // or make their metadata fields public
        StateVersion::new(1, 0, 0)
    }

    /// Create a differential snapshot (only changed fields)
    pub async fn create_diff_snapshot(
        &self,
        module_id: &str,
        current_state: &ModuleState,
        previous_snapshot: Option<&StateSnapshot>,
    ) -> HotReloadResult<StateSnapshot> {
        debug!("Creating differential snapshot for module: {}", module_id);

        let mut snapshot = StateSnapshot {
            module_id: module_id.to_string(),
            version: StateVersion::from_module_version(&current_state.version),
            timestamp: chrono::Utc::now(),
            state: current_state.clone(),
            metadata: HashMap::new(),
        };

        // If we have a previous snapshot, mark what changed
        if let Some(prev) = previous_snapshot {
            let mut changes = Vec::new();
            
            // Compare state fields
            for (key, value) in current_state.data.iter() {
                if prev.state.data.get(key) != Some(value) {
                    changes.push(key.clone());
                }
            }
            
            // Store change information in metadata
            snapshot.metadata.insert(
                "changed_fields".to_string(),
                serde_json::json!(changes),
            );
            snapshot.metadata.insert(
                "previous_version".to_string(),
                serde_json::json!(prev.version.to_string()),
            );
        }

        // Store snapshot
        let mut snapshots = self.snapshots.write().await;
        let module_snapshots = snapshots.entry(module_id.to_string()).or_insert_with(Vec::new);
        module_snapshots.push(snapshot.clone());

        Ok(snapshot)
    }

    /// Validate that a state can be restored to a module
    pub fn validate_state_compatibility(
        &self,
        state_version: &StateVersion,
        module_version: &StateVersion,
    ) -> bool {
        state_version.is_compatible_with(module_version)
    }

    /// Compress state snapshots to save memory
    pub async fn compress_snapshots(&self, module_id: &str) -> Result<()> {
        let mut snapshots = self.snapshots.write().await;
        
        if let Some(module_snapshots) = snapshots.get_mut(module_id) {
            // Keep only the most recent snapshots
            let keep_count = self.max_snapshots_per_module / 2;
            if module_snapshots.len() > keep_count {
                let drain_count = module_snapshots.len() - keep_count;
                module_snapshots.drain(0..drain_count);
            }
        }

        Ok(())
    }
}