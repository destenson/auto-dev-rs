// Reload Coordinator - Orchestrates the hot-reload process

use super::{
    HotReloadConfig, HotReloadError, HotReloadResult, MigrationEngine, ReloadMetrics,
    StateManager, TrafficController, ReloadVerifier,
};
use crate::modules::{ModuleLoader, ModuleRegistry, ModuleRuntime, ModuleState};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};

/// Phases of the reload process
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReloadPhase {
    Idle,
    Prepare,
    Drain,
    Snapshot,
    Migrate,
    Swap,
    Restore,
    Verify,
    Commit,
    Rollback,
}

/// Result of a reload operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReloadResult {
    pub success: bool,
    pub module_id: String,
    pub duration_ms: u64,
    pub messages_preserved: u64,
    pub state_migrated: bool,
    pub error: Option<String>,
}

/// Manages module reload operations in progress
#[derive(Clone)]
struct ReloadTransaction {
    module_id: String,
    phase: ReloadPhase,
    started_at: Instant,
    old_state: Option<ModuleState>,
    new_module_path: PathBuf,
    messages_buffered: Vec<crate::modules::Message>,
}

/// Coordinates the hot-reload process
pub struct ReloadCoordinator {
    config: HotReloadConfig,
    state_manager: Arc<StateManager>,
    traffic_controller: Arc<TrafficController>,
    migration_engine: Arc<MigrationEngine>,
    verifier: Arc<ReloadVerifier>,
    metrics: Arc<RwLock<ReloadMetrics>>,
    active_reloads: Arc<Mutex<HashMap<String, ReloadTransaction>>>,
}

impl ReloadCoordinator {
    pub fn new(config: HotReloadConfig) -> Self {
        Self {
            config: config.clone(),
            state_manager: Arc::new(StateManager::new()),
            traffic_controller: Arc::new(TrafficController::new(config.drain_timeout)),
            migration_engine: Arc::new(MigrationEngine::new()),
            verifier: Arc::new(ReloadVerifier::new()),
            metrics: Arc::new(RwLock::new(ReloadMetrics::default())),
            active_reloads: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Execute a hot-reload for a module
    pub async fn reload_module(
        &self,
        module_id: &str,
        new_path: PathBuf,
        registry: Arc<RwLock<ModuleRegistry>>,
        loader: Arc<ModuleLoader>,
        runtime: Arc<ModuleRuntime>,
    ) -> HotReloadResult<ReloadResult> {
        let start = Instant::now();
        info!("Starting hot-reload for module: {}", module_id);

        // Check for concurrent reloads
        if !self.config.allow_concurrent_reloads {
            let active = self.active_reloads.lock().await;
            if !active.is_empty() {
                return Err(HotReloadError::ConcurrentReloadDenied);
            }
        }

        // Create reload transaction
        let mut transaction = ReloadTransaction {
            module_id: module_id.to_string(),
            phase: ReloadPhase::Idle,
            started_at: start,
            old_state: None,
            new_module_path: new_path.clone(),
            messages_buffered: Vec::new(),
        };

        // Store transaction
        {
            let mut active = self.active_reloads.lock().await;
            active.insert(module_id.to_string(), transaction.clone());
        }

        // Execute reload phases
        let result = self
            .execute_reload(&mut transaction, registry, loader, runtime)
            .await;

        // Clean up transaction
        {
            let mut active = self.active_reloads.lock().await;
            active.remove(module_id);
        }

        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.total_reloads += 1;
        
        let duration = start.elapsed();
        let reload_result = match result {
            Ok(messages_preserved) => {
                metrics.successful_reloads += 1;
                metrics.messages_preserved += messages_preserved;
                metrics.last_reload_time = Some(chrono::Utc::now());
                
                // Update average reload time
                let total_time = metrics.average_reload_time_ms * (metrics.successful_reloads - 1);
                metrics.average_reload_time_ms = 
                    (total_time + duration.as_millis() as u64) / metrics.successful_reloads;
                
                ReloadResult {
                    success: true,
                    module_id: module_id.to_string(),
                    duration_ms: duration.as_millis() as u64,
                    messages_preserved,
                    state_migrated: transaction.old_state.is_some(),
                    error: None,
                }
            }
            Err(e) => {
                metrics.failed_reloads += 1;
                
                ReloadResult {
                    success: false,
                    module_id: module_id.to_string(),
                    duration_ms: duration.as_millis() as u64,
                    messages_preserved: 0,
                    state_migrated: false,
                    error: Some(e.to_string()),
                }
            }
        };

        if reload_result.success {
            Ok(reload_result)
        } else {
            Err(HotReloadError::VerificationFailed(
                reload_result.error.unwrap_or_default(),
            ))
        }
    }

    /// Execute the reload transaction through all phases
    async fn execute_reload(
        &self,
        transaction: &mut ReloadTransaction,
        registry: Arc<RwLock<ModuleRegistry>>,
        loader: Arc<ModuleLoader>,
        runtime: Arc<ModuleRuntime>,
    ) -> HotReloadResult<u64> {
        // Phase 1: Prepare
        self.set_phase(transaction, ReloadPhase::Prepare).await;
        debug!("Phase: Prepare - Loading new module version");
        
        let format = loader.get_format(&transaction.module_id)
            .map_err(|e| HotReloadError::VerificationFailed(e.to_string()))?;
        
        let new_module = loader
            .load(transaction.new_module_path.clone(), format)
            .await
            .map_err(|e| HotReloadError::VerificationFailed(e.to_string()))?;

        // Phase 2: Drain
        self.set_phase(transaction, ReloadPhase::Drain).await;
        debug!("Phase: Drain - Stopping new requests to old module");
        
        self.traffic_controller
            .start_draining(&transaction.module_id)
            .await?;
        
        // Wait for drain or timeout
        let drain_start = Instant::now();
        while !self.traffic_controller.is_drained(&transaction.module_id).await {
            if drain_start.elapsed() > self.config.drain_timeout {
                self.traffic_controller.cancel_draining(&transaction.module_id).await;
                return Err(HotReloadError::DrainTimeout);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Phase 3: Snapshot
        self.set_phase(transaction, ReloadPhase::Snapshot).await;
        debug!("Phase: Snapshot - Capturing current state");
        
        transaction.old_state = runtime
            .get_module_state(&transaction.module_id)
            .await
            .ok();
        
        let snapshot = if let Some(ref state) = transaction.old_state {
            Some(self.state_manager.create_snapshot(&transaction.module_id, state).await?)
        } else {
            None
        };

        // Phase 4: Migrate
        self.set_phase(transaction, ReloadPhase::Migrate).await;
        debug!("Phase: Migrate - Transforming state if needed");
        
        let migrated_state = if let Some(ref old_state) = transaction.old_state {
            // Check if migration is needed
            let old_version = self.state_manager.get_state_version(old_state);
            let new_version = self.state_manager.get_module_version(&new_module);
            
            if old_version != new_version {
                info!("Migrating state from {} to {}", old_version, new_version);
                self.migration_engine
                    .migrate_state(old_state.clone(), old_version, new_version)
                    .await?
            } else {
                old_state.clone()
            }
        } else {
            // No previous state, use default
            ModuleState::new(crate::modules::ModuleVersion::new(1, 0, 0))
        };

        // Phase 5: Swap
        self.set_phase(transaction, ReloadPhase::Swap).await;
        debug!("Phase: Swap - Replacing old module with new");
        
        // Buffer any incoming messages during swap
        self.traffic_controller
            .start_buffering(&transaction.module_id)
            .await;
        
        // Perform atomic swap
        let swap_result = registry
            .write()
            .await
            .update(&transaction.module_id, new_module)
            .await;
        
        if let Err(e) = swap_result {
            error!("Module swap failed: {}", e);
            self.rollback(transaction, snapshot, runtime).await?;
            return Err(HotReloadError::VerificationFailed(e.to_string()));
        }

        // Phase 6: Restore
        self.set_phase(transaction, ReloadPhase::Restore).await;
        debug!("Phase: Restore - Loading state into new module");
        
        if let Err(e) = runtime
            .restore_module_state(&transaction.module_id, migrated_state)
            .await
        {
            error!("State restoration failed: {}", e);
            self.rollback(transaction, snapshot, runtime).await?;
            return Err(HotReloadError::MigrationFailed(e.to_string()));
        }

        // Phase 7: Verify
        self.set_phase(transaction, ReloadPhase::Verify).await;
        debug!("Phase: Verify - Ensuring module works");
        
        for attempt in 0..self.config.max_verification_attempts {
            let verification = self
                .verifier
                .verify_module(&transaction.module_id, runtime.clone())
                .await;
            
            match verification {
                Ok(result) if result.is_healthy => {
                    info!("Module verification successful on attempt {}", attempt + 1);
                    break;
                }
                Ok(result) => {
                    warn!("Module verification failed: {:?}", result.issues);
                    if attempt == self.config.max_verification_attempts - 1 {
                        self.rollback(transaction, snapshot, runtime).await?;
                        return Err(HotReloadError::VerificationFailed(
                            format!("Health check failed: {:?}", result.issues),
                        ));
                    }
                }
                Err(e) => {
                    error!("Verification error: {}", e);
                    if attempt == self.config.max_verification_attempts - 1 {
                        self.rollback(transaction, snapshot, runtime).await?;
                        return Err(HotReloadError::VerificationFailed(e.to_string()));
                    }
                }
            }
            
            tokio::time::sleep(self.config.verification_delay).await;
        }

        // Phase 8: Commit
        self.set_phase(transaction, ReloadPhase::Commit).await;
        debug!("Phase: Commit - Finalizing reload");
        
        // Resume traffic and deliver buffered messages
        let messages_count = self
            .traffic_controller
            .resume_traffic(&transaction.module_id)
            .await?;
        
        info!(
            "Hot-reload completed successfully for module: {}",
            transaction.module_id
        );
        
        Ok(messages_count as u64)
    }

    /// Rollback a failed reload
    async fn rollback(
        &self,
        transaction: &mut ReloadTransaction,
        snapshot: Option<crate::modules::hot_reload::StateSnapshot>,
        runtime: Arc<ModuleRuntime>,
    ) -> HotReloadResult<()> {
        self.set_phase(transaction, ReloadPhase::Rollback).await;
        warn!("Rolling back reload for module: {}", transaction.module_id);

        // Restore original state if we have a snapshot
        if let Some(snapshot) = snapshot {
            self.state_manager
                .restore_snapshot(&transaction.module_id, snapshot, runtime)
                .await
                .map_err(|e| HotReloadError::RollbackFailed(e.to_string()))?;
        }

        // Cancel traffic control
        self.traffic_controller
            .cancel_draining(&transaction.module_id)
            .await;

        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.rollbacks += 1;

        Ok(())
    }

    /// Update the current phase of a reload transaction
    async fn set_phase(&self, transaction: &mut ReloadTransaction, phase: ReloadPhase) {
        transaction.phase = phase;
        debug!(
            "Module {} reload phase: {:?}",
            transaction.module_id, phase
        );
    }

    /// Get current reload metrics
    pub async fn get_metrics(&self) -> ReloadMetrics {
        self.metrics.read().await.clone()
    }

    /// Check if a module is currently being reloaded
    pub async fn is_reloading(&self, module_id: &str) -> bool {
        let active = self.active_reloads.lock().await;
        active.contains_key(module_id)
    }

    /// Get the current phase of a reload in progress
    pub async fn get_reload_phase(&self, module_id: &str) -> Option<ReloadPhase> {
        let active = self.active_reloads.lock().await;
        active.get(module_id).map(|t| t.phase)
    }
}