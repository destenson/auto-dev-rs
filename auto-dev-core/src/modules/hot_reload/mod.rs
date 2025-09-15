// Hot-Reload Infrastructure
//
// Provides seamless module reloading with state preservation and zero downtime

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

pub mod coordinator;
pub mod migration;
pub mod scheduler;
pub mod state_manager;
pub mod traffic_controller;
pub mod verifier;

#[cfg(test)]
mod tests;

pub use coordinator::{ReloadCoordinator, ReloadPhase, ReloadResult};
pub use migration::{FieldMapping, MigrationEngine, MigrationRule, TransformType};
pub use scheduler::{ReloadScheduler, ReloadRequest, ReloadPriority, SchedulerConfig, SchedulingStrategy};
pub use state_manager::{StateManager, StateSnapshot, StateVersion};
pub use traffic_controller::{TrafficController, TrafficState};
pub use verifier::{ReloadVerifier, TestType, VerificationResult, VerificationTest};

/// Configuration for hot-reload behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotReloadConfig {
    /// Maximum time to wait for traffic draining
    pub drain_timeout: Duration,
    
    /// Maximum time for the entire reload operation
    pub reload_timeout: Duration,
    
    /// Enable automatic rollback on failure
    pub auto_rollback: bool,
    
    /// Number of verification attempts before considering reload failed
    pub max_verification_attempts: u32,
    
    /// Delay between verification attempts
    pub verification_delay: Duration,
    
    /// Enable concurrent module reloads
    pub allow_concurrent_reloads: bool,
    
    /// Maximum memory usage during reload (bytes)
    pub max_memory_usage: usize,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            drain_timeout: Duration::from_secs(5),
            reload_timeout: Duration::from_secs(30),
            auto_rollback: true,
            max_verification_attempts: 3,
            verification_delay: Duration::from_millis(100),
            allow_concurrent_reloads: true,
            max_memory_usage: 100 * 1024 * 1024, // 100MB
        }
    }
}

/// Metrics collected during hot-reload operations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReloadMetrics {
    pub total_reloads: u64,
    pub successful_reloads: u64,
    pub failed_reloads: u64,
    pub rollbacks: u64,
    pub average_reload_time_ms: u64,
    pub messages_preserved: u64,
    pub state_migration_count: u64,
    pub last_reload_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// Error types specific to hot-reload operations
#[derive(Debug, thiserror::Error)]
pub enum HotReloadError {
    #[error("Traffic draining timeout exceeded")]
    DrainTimeout,
    
    #[error("State migration failed: {0}")]
    MigrationFailed(String),
    
    #[error("Module verification failed: {0}")]
    VerificationFailed(String),
    
    #[error("Rollback failed: {0}")]
    RollbackFailed(String),
    
    #[error("Concurrent reload not allowed")]
    ConcurrentReloadDenied,
    
    #[error("Memory limit exceeded during reload")]
    MemoryLimitExceeded,
    
    #[error("Incompatible state version: expected {expected}, got {actual}")]
    IncompatibleStateVersion { expected: String, actual: String },
}

/// Result type for hot-reload operations
pub type HotReloadResult<T> = std::result::Result<T, HotReloadError>;