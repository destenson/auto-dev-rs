//! Safety validation gates for preventing dangerous self-modifications
//!
//! This module implements comprehensive safety checks that validate
//! all self-modifications before they're applied, ensuring system
//! stability during autonomous development.

pub mod gates;
pub mod validators;
pub mod invariants;
pub mod contracts;
pub mod analyzer;

pub use gates::{SafetyGatekeeper, ValidationGate, GateResult};
pub use validators::{
    StaticValidator, SemanticValidator, SecurityValidator, 
    PerformanceValidator, ReversibilityValidator
};
pub use invariants::InvariantChecker;
pub use contracts::ContractVerifier;
pub use analyzer::StaticAnalyzer;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SafetyError {
    #[error("Static analysis failed: {0}")]
    StaticAnalysis(String),
    
    #[error("Semantic validation failed: {0}")]
    SemanticValidation(String),
    
    #[error("Security gate blocked: {0}")]
    SecurityViolation(String),
    
    #[error("Performance issue detected: {0}")]
    PerformanceViolation(String),
    
    #[error("Change is not reversible: {0}")]
    IrreversibleChange(String),
    
    #[error("Invariant violation: {0}")]
    InvariantViolation(String),
    
    #[error("Contract violation: {0}")]
    ContractViolation(String),
    
    #[error("Critical file modification attempted: {0}")]
    CriticalFileViolation(PathBuf),
    
    #[error("Multiple gate failures: {0:?}")]
    MultipleFailures(Vec<String>),
}

pub type Result<T> = std::result::Result<T, SafetyError>;

/// Configuration for safety validation gates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    /// Enable static analysis checks
    pub static_analysis: bool,
    
    /// Enable semantic validation
    pub semantic_validation: bool,
    
    /// Enable security gates
    pub security_gates: bool,
    
    /// Enable performance validation
    pub performance_validation: bool,
    
    /// Require reversibility for all changes
    pub require_reversibility: bool,
    
    /// Fail on first gate failure
    pub fail_fast: bool,
    
    /// Require all gates to pass
    pub require_all_gates: bool,
    
    /// Maximum validation time in seconds
    pub max_validation_time: u64,
    
    /// Critical files that must never be modified
    pub critical_files: Vec<PathBuf>,
    
    /// Allowed modification paths
    pub allowed_paths: Vec<PathBuf>,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            static_analysis: true,
            semantic_validation: true,
            security_gates: true,
            performance_validation: true,
            require_reversibility: true,
            fail_fast: false,
            require_all_gates: true,
            max_validation_time: 30,
            critical_files: vec![
                PathBuf::from("src/main.rs"),
                PathBuf::from("src/safety"),
                PathBuf::from("Cargo.lock"),
                PathBuf::from(".git"),
            ],
            allowed_paths: vec![
                PathBuf::from("src"),
                PathBuf::from("tests"),
                PathBuf::from("docs"),
            ],
        }
    }
}

/// Represents a code modification to be validated
#[derive(Debug, Clone)]
pub struct CodeModification {
    /// File being modified
    pub file_path: PathBuf,
    
    /// Original content
    pub original: String,
    
    /// Modified content
    pub modified: String,
    
    /// Type of modification
    pub modification_type: ModificationType,
    
    /// Reason for modification
    pub reason: String,
    
    /// Associated PRP if any
    pub prp_reference: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModificationType {
    Create,
    Update,
    Delete,
    Refactor,
    BugFix,
    Feature,
}

/// Validation report from safety gates
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Overall validation result
    pub passed: bool,
    
    /// Individual gate results
    pub gate_results: Vec<GateResult>,
    
    /// Validation duration
    pub duration_ms: u64,
    
    /// Risk assessment
    pub risk_level: RiskLevel,
    
    /// Recommendations if validation failed
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Initialize the safety system
pub async fn initialize(config: SafetyConfig) -> Result<SafetyGatekeeper> {
    SafetyGatekeeper::new(config)
}

/// Quick validation for a single file
pub async fn validate_file(path: &Path, content: &str) -> Result<bool> {
    let config = SafetyConfig::default();
    let gatekeeper = SafetyGatekeeper::new(config)?;
    gatekeeper.validate_file(path, content).await
}

/// Check if a path is critical and should not be modified
pub fn is_critical_path(path: &Path) -> bool {
    let critical_paths = [
        "src/main.rs",
        "src/safety",
        "Cargo.lock",
        ".git",
        "target",
    ];
    
    critical_paths.iter().any(|critical| {
        path.starts_with(critical) || path.ends_with(critical)
    })
}