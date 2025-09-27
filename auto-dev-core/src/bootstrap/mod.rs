//! Bootstrap sequence for safe self-development initialization
//! 
//! This module implements the bootstrap process that initializes auto-dev-rs's
//! self-development mode, setting up the environment, validating prerequisites,
//! and starting the continuous self-improvement loop.

pub mod sequence;
pub mod validator;
pub mod initializer;
pub mod snapshot;
pub mod preflight;

pub use sequence::BootstrapSequence;
pub use validator::EnvironmentValidator;
pub use initializer::SystemInitializer;
pub use snapshot::BaselineCreator;
pub use preflight::PreflightChecker;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::path::PathBuf;

#[derive(Debug, Error)]
pub enum BootstrapError {
    #[error("Pre-flight check failed: {0}")]
    PreflightFailed(String),
    
    #[error("Environment validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Initialization failed: {0}")]
    InitializationFailed(String),
    
    #[error("Baseline creation failed: {0}")]
    BaselineFailed(String),
    
    #[error("Activation failed: {0}")]
    ActivationFailed(String),
    
    #[error("Bootstrap already in progress")]
    AlreadyInProgress,
    
    #[error("Bootstrap interrupted at stage: {0}")]
    Interrupted(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
}

pub type Result<T> = std::result::Result<T, BootstrapError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfig {
    pub preflight: PreflightConfig,
    pub baseline: BaselineConfig,
    pub safety: SafetyConfig,
    pub modules: ModulesConfig,
    pub monitoring: MonitoringConfig,
    pub loop_config: LoopConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightConfig {
    pub strict: bool,
    pub check_disk_space: bool,
    pub required_disk_gb: u64,
    pub check_git_state: bool,
    pub require_clean_git: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineConfig {
    pub include_performance: bool,
    pub include_capabilities: bool,
    pub include_metrics: bool,
    pub snapshot_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    pub require_clean_git: bool,
    pub require_passing_tests: bool,
    pub create_backup: bool,
    pub backup_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModulesConfig {
    pub load_existing: bool,
    pub verify_signatures: bool,
    pub sandbox_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub start_immediately: bool,
    pub initial_delay_seconds: u64,
    pub watch_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopConfig {
    pub initial_delay_seconds: u64,
    pub iteration_interval_seconds: u64,
    pub max_iterations_per_day: Option<u32>,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            preflight: PreflightConfig {
                strict: true,
                check_disk_space: true,
                required_disk_gb: 1,
                check_git_state: true,
                require_clean_git: true,
            },
            baseline: BaselineConfig {
                include_performance: true,
                include_capabilities: true,
                include_metrics: true,
                snapshot_dir: PathBuf::from(".auto-dev/snapshots"),
            },
            safety: SafetyConfig {
                require_clean_git: true,
                require_passing_tests: true,
                create_backup: true,
                backup_dir: PathBuf::from(".auto-dev/backups"),
            },
            modules: ModulesConfig {
                load_existing: true,
                verify_signatures: false,
                sandbox_enabled: true,
            },
            monitoring: MonitoringConfig {
                start_immediately: false,
                initial_delay_seconds: 10,
                watch_paths: vec!["src".to_string(), "PRPs".to_string()],
            },
            loop_config: LoopConfig {
                initial_delay_seconds: 10,
                iteration_interval_seconds: 300,
                max_iterations_per_day: Some(100),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BootstrapStage {
    NotStarted,
    PreflightChecks,
    EnvironmentSetup,
    BaselineCreation,
    Activation,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapStatus {
    pub current_stage: BootstrapStage,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub checkpoint_path: PathBuf,
    pub error: Option<String>,
}

impl Default for BootstrapStatus {
    fn default() -> Self {
        Self {
            current_stage: BootstrapStage::NotStarted,
            started_at: None,
            completed_at: None,
            checkpoint_path: PathBuf::from(".auto-dev/bootstrap.checkpoint"),
            error: None,
        }
    }
}

pub async fn bootstrap(config: BootstrapConfig, dry_run: bool) -> Result<()> {
    let mut sequence = BootstrapSequence::new(config);
    
    if dry_run {
        sequence.dry_run().await
    } else {
        sequence.execute().await
    }
}

pub async fn resume_bootstrap() -> Result<()> {
    let mut sequence = BootstrapSequence::resume().await?;
    sequence.execute().await
}

pub async fn bootstrap_status() -> Result<BootstrapStatus> {
    BootstrapSequence::get_status().await
}

pub async fn reset_bootstrap() -> Result<()> {
    BootstrapSequence::reset().await
}