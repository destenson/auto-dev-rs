//! Self-development integration and orchestration module
//!
//! This module ties together all self-development capabilities to enable
//! auto-dev-rs to autonomously improve itself while maintaining stability,
//! safety, and continuous operation.

pub mod control;
pub mod coordinator;
pub mod monitor;
pub mod orchestrator;
pub mod state_machine;

pub use control::{CommandResult, ControlCommand, OperatorInterface};
pub use coordinator::ComponentCoordinator;
pub use monitor::SafetyMonitor;
pub use orchestrator::{
    ChangeMetrics, ChangeStatus, ChangeType, PendingChange, PlanDigest, PlanStep, RiskLevel,
    SelfDevOrchestrator, SelfDevStatus, TestResults, TestRunSummary,
};
pub use state_machine::{DevelopmentState, DevelopmentStateMachine};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SelfDevError {
    #[error("Orchestration error: {0}")]
    Orchestration(String),

    #[error("State transition error: {0}")]
    StateTransition(String),

    #[error("Safety validation failed: {0}")]
    SafetyViolation(String),

    #[error("Component coordination error: {0}")]
    Coordination(String),

    #[error("Control command error: {0}")]
    Control(String),

    #[error("Configuration error: {0}")]
    Configuration(String),
}

pub type Result<T> = std::result::Result<T, SelfDevError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfDevConfig {
    pub enabled: bool,
    pub mode: DevelopmentMode,
    pub safety_level: SafetyLevel,
    pub auto_approve: bool,
    pub max_changes_per_day: usize,
    pub require_tests: bool,
    pub require_documentation: bool,
    pub components: ComponentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DevelopmentMode {
    Observation,
    Assisted,
    SemiAutonomous,
    FullyAutonomous,
}

#[repr(u8)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SafetyLevel {
    Permissive,
    Standard,
    Strict,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComponentConfig {
    pub monitoring: bool,
    pub synthesis: bool,
    pub testing: bool,
    pub deployment: bool,
    pub learning: bool,
}

impl Default for SelfDevConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: DevelopmentMode::Observation,
            safety_level: SafetyLevel::Strict,
            auto_approve: false,
            max_changes_per_day: 10,
            require_tests: true,
            require_documentation: true,
            components: ComponentConfig {
                monitoring: true,
                synthesis: false,
                testing: false,
                deployment: false,
                learning: false,
            },
        }
    }
}

pub async fn initialize(config: SelfDevConfig) -> Result<SelfDevOrchestrator> {
    SelfDevOrchestrator::new(config).await
}
