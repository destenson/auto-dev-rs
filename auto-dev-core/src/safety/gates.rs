//! Safety gate definitions and orchestration

use super::{CodeModification, Result, RiskLevel, SafetyConfig, SafetyError, ValidationReport};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Main safety coordinator that manages all validation gates
pub struct SafetyGatekeeper {
    config: Arc<SafetyConfig>,
    gates: Vec<Box<dyn ValidationGate>>,
    validation_history: Arc<RwLock<Vec<ValidationReport>>>,
}

impl SafetyGatekeeper {
    pub fn new(config: SafetyConfig) -> Result<Self> {
        let mut gates: Vec<Box<dyn ValidationGate>> = Vec::new();

        if config.static_analysis {
            gates.push(Box::new(super::validators::StaticValidator::new()));
        }

        if config.semantic_validation {
            gates.push(Box::new(super::validators::SemanticValidator::new()));
        }

        if config.security_gates {
            gates.push(Box::new(super::validators::SecurityValidator::new()));
        }

        if config.performance_validation {
            gates.push(Box::new(super::validators::PerformanceValidator::new()));
        }

        if config.require_reversibility {
            gates.push(Box::new(super::validators::ReversibilityValidator::new()));
        }

        Ok(Self {
            config: Arc::new(config),
            gates,
            validation_history: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Validate a code modification through all gates
    pub async fn validate(&self, modification: &CodeModification) -> Result<ValidationReport> {
        info!("Validating modification to {}", modification.file_path.display());
        let start = Instant::now();

        // First check: Is this a critical file?
        if self.is_critical_file(&modification.file_path) {
            error!("Attempted to modify critical file: {}", modification.file_path.display());
            return Err(SafetyError::CriticalFileViolation(modification.file_path.clone()));
        }

        // Second check: Is this in an allowed path?
        if !self.is_allowed_path(&modification.file_path) {
            warn!("Modification outside allowed paths: {}", modification.file_path.display());
            return Err(SafetyError::SecurityViolation(format!(
                "Path {} is not in allowed modification list",
                modification.file_path.display()
            )));
        }

        let mut gate_results = Vec::new();
        let mut failures = Vec::new();
        let mut highest_risk = RiskLevel::Low;

        // Run through all gates
        for gate in &self.gates {
            debug!("Running gate: {}", gate.name());

            let timeout = Duration::from_secs(self.config.max_validation_time);
            let gate_future = gate.validate(modification);

            let result = match tokio::time::timeout(timeout, gate_future).await {
                Ok(gate_result) => gate_result,
                Err(_) => {
                    error!("Gate {} timed out", gate.name());
                    GateResult {
                        gate_name: gate.name(),
                        passed: false,
                        risk_level: RiskLevel::High,
                        issues: vec![format!(
                            "Gate timed out after {} seconds",
                            self.config.max_validation_time
                        )],
                        suggestions: vec![
                            "Consider breaking down the modification into smaller pieces"
                                .to_string(),
                        ],
                    }
                }
            };

            if !result.passed {
                failures.push(format!("{}: {:?}", result.gate_name, result.issues));
                if self.config.fail_fast {
                    error!("Gate {} failed (fail-fast enabled)", result.gate_name);
                    break;
                }
            }

            if result.risk_level as u8 > highest_risk as u8 {
                highest_risk = result.risk_level;
            }

            gate_results.push(result);
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let passed = failures.is_empty()
            || (!self.config.require_all_gates && failures.len() < self.gates.len());

        let report = ValidationReport {
            passed,
            gate_results,
            duration_ms,
            risk_level: highest_risk,
            recommendations: if !passed {
                vec![
                    "Review the specific gate failures".to_string(),
                    "Consider splitting the modification into smaller, safer changes".to_string(),
                    "Ensure all tests pass before modification".to_string(),
                ]
            } else {
                vec![]
            },
        };

        // Store in history
        self.validation_history.write().await.push(report.clone());

        if !passed && self.config.require_all_gates {
            return Err(SafetyError::MultipleFailures(failures));
        }

        Ok(report)
    }

    /// Quick validation for a single file
    pub async fn validate_file(&self, path: &Path, content: &str) -> Result<bool> {
        let modification = CodeModification {
            file_path: path.to_path_buf(),
            original: String::new(),
            modified: content.to_string(),
            modification_type: super::ModificationType::Update,
            reason: "Quick validation".to_string(),
            prp_reference: None,
        };

        let report = self.validate(&modification).await?;
        Ok(report.passed)
    }

    /// Check if a path is a critical file
    fn is_critical_file(&self, path: &Path) -> bool {
        self.config
            .critical_files
            .iter()
            .any(|critical| path == critical || path.starts_with(critical))
    }

    /// Check if a path is in allowed modification list
    fn is_allowed_path(&self, path: &Path) -> bool {
        if self.config.allowed_paths.is_empty() {
            return true; // No restrictions
        }

        self.config.allowed_paths.iter().any(|allowed| path.starts_with(allowed))
    }

    /// Get validation history
    pub async fn get_history(&self) -> Vec<ValidationReport> {
        self.validation_history.read().await.clone()
    }

    /// Clear validation history
    pub async fn clear_history(&self) {
        self.validation_history.write().await.clear();
    }
}

/// Individual validation gate interface
#[async_trait]
pub trait ValidationGate: Send + Sync {
    /// Name of this gate
    fn name(&self) -> String;

    /// Validate a code modification
    async fn validate(&self, modification: &CodeModification) -> GateResult;

    /// Get gate configuration
    fn get_config(&self) -> GateConfig {
        GateConfig::default()
    }
}

/// Result from a single validation gate
#[derive(Debug, Clone)]
pub struct GateResult {
    /// Name of the gate
    pub gate_name: String,

    /// Whether validation passed
    pub passed: bool,

    /// Risk level assessment
    pub risk_level: RiskLevel,

    /// Specific issues found
    pub issues: Vec<String>,

    /// Suggestions for fixing issues
    pub suggestions: Vec<String>,
}

/// Configuration for a single gate
#[derive(Debug, Clone)]
pub struct GateConfig {
    /// Is this gate enabled
    pub enabled: bool,

    /// Should this gate block on failure
    pub blocking: bool,

    /// Maximum time allowed for this gate
    pub timeout_seconds: u64,
}

impl Default for GateConfig {
    fn default() -> Self {
        Self { enabled: true, blocking: true, timeout_seconds: 10 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_critical_file_protection() {
        let config = SafetyConfig::default();
        let gatekeeper = SafetyGatekeeper::new(config).unwrap();

        let modification = CodeModification {
            file_path: PathBuf::from("src/main.rs"),
            original: String::new(),
            modified: String::new(),
            modification_type: super::super::ModificationType::Update,
            reason: "Test".to_string(),
            prp_reference: None,
        };

        let result = gatekeeper.validate(&modification).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SafetyError::CriticalFileViolation(_)));
    }

    #[tokio::test]
    async fn test_allowed_path_validation() {
        let mut config = SafetyConfig::default();
        config.allowed_paths = vec![PathBuf::from("src/modules")];
        let gatekeeper = SafetyGatekeeper::new(config).unwrap();

        let modification = CodeModification {
            file_path: PathBuf::from("tests/test.rs"),
            original: String::new(),
            modified: String::new(),
            modification_type: super::super::ModificationType::Create,
            reason: "Test".to_string(),
            prp_reference: None,
        };

        let result = gatekeeper.validate(&modification).await;
        assert!(result.is_err());
    }
}
