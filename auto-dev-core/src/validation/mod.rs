//! Validation module for ensuring code quality and correctness
//! Leverages existing tools like cargo check, clippy, rustfmt, etc.

pub mod verifier;
pub mod validator;
pub mod quality;
pub mod security;
pub mod performance;
pub mod contracts;

use serde::{Deserialize, Serialize};
use std::time::Duration;
use anyhow::Result;

pub use verifier::CodeVerifier;
pub use validator::{SpecValidator, ValidationPipeline};

/// Result of validation checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub passed: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub metrics: QualityMetrics,
    pub suggestions: Vec<Improvement>,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub severity: Severity,
    pub category: ErrorCategory,
    pub location: Option<SourceLocation>,
    pub message: String,
    pub fix_suggestion: Option<String>,
}

/// Validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub category: ErrorCategory,
    pub message: String,
    pub location: Option<SourceLocation>,
}

/// Error severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Error categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorCategory {
    Compilation,
    Specification,
    Security,
    Performance,
    Quality,
    Standards,
    Test,
}

/// Source code location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

/// Quality metrics from analysis
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QualityMetrics {
    pub cyclomatic_complexity: Option<f32>,
    pub cognitive_complexity: Option<f32>,
    pub maintainability_index: Option<f32>,
    pub test_coverage: Option<f32>,
    pub documentation_coverage: Option<f32>,
    pub code_duplication: Option<f32>,
}

/// Improvement suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Improvement {
    pub category: String,
    pub description: String,
    pub priority: Priority,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// Validation stage in the pipeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationStage {
    SyntaxCheck,
    Compilation,
    UnitTests,
    Linting,
    Security,
    Performance,
    Specification,
    Integration,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            passed: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            metrics: QualityMetrics::default(),
            suggestions: Vec::new(),
        }
    }
    
    pub fn failed(error: ValidationError) -> Self {
        Self {
            passed: false,
            errors: vec![error],
            warnings: Vec::new(),
            metrics: QualityMetrics::default(),
            suggestions: Vec::new(),
        }
    }
    
    pub fn warning(warning: ValidationWarning) -> Self {
        Self {
            passed: true,
            errors: Vec::new(),
            warnings: vec![warning],
            metrics: QualityMetrics::default(),
            suggestions: Vec::new(),
        }
    }
    
    pub fn aggregate(results: Vec<ValidationResult>) -> Self {
        let mut aggregated = Self::new();
        
        for result in results {
            aggregated.passed = aggregated.passed && result.passed;
            aggregated.errors.extend(result.errors);
            aggregated.warnings.extend(result.warnings);
            aggregated.suggestions.extend(result.suggestions);
            
            // Merge metrics (take the worst values)
            if let Some(v) = result.metrics.cyclomatic_complexity {
                aggregated.metrics.cyclomatic_complexity = Some(
                    aggregated.metrics.cyclomatic_complexity
                        .map(|existing| existing.max(v))
                        .unwrap_or(v)
                );
            }
        }
        
        aggregated
    }
    
    pub fn has_critical_errors(&self) -> bool {
        self.errors.iter().any(|e| e.severity == Severity::Critical)
    }
    
    pub fn summary(&self) -> String {
        format!(
            "Validation {}: {} errors, {} warnings",
            if self.passed { "PASSED" } else { "FAILED" },
            self.errors.len(),
            self.warnings.len()
        )
    }
}

/// Configuration for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub enabled: bool,
    pub fail_fast: bool,
    pub parallel: bool,
    pub stages: Vec<StageConfig>,
    pub quality_rules: QualityRules,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageConfig {
    pub stage: ValidationStage,
    pub enabled: bool,
    pub timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityRules {
    pub max_function_length: usize,
    pub max_cyclomatic_complexity: u32,
    pub max_cognitive_complexity: u32,
    pub min_test_coverage: f32,
    pub max_duplication: f32,
    pub required_documentation: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            fail_fast: true,
            parallel: true,
            stages: vec![
                StageConfig {
                    stage: ValidationStage::SyntaxCheck,
                    enabled: true,
                    timeout: Duration::from_secs(5),
                },
                StageConfig {
                    stage: ValidationStage::Compilation,
                    enabled: true,
                    timeout: Duration::from_secs(30),
                },
                StageConfig {
                    stage: ValidationStage::UnitTests,
                    enabled: true,
                    timeout: Duration::from_secs(60),
                },
                StageConfig {
                    stage: ValidationStage::Linting,
                    enabled: true,
                    timeout: Duration::from_secs(10),
                },
            ],
            quality_rules: QualityRules::default(),
        }
    }
}

impl Default for QualityRules {
    fn default() -> Self {
        Self {
            max_function_length: 50,
            max_cyclomatic_complexity: 10,
            max_cognitive_complexity: 15,
            min_test_coverage: 80.0,
            max_duplication: 5.0,
            required_documentation: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_creation() {
        let result = ValidationResult::new();
        assert!(result.passed);
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_validation_result_failed() {
        let error = ValidationError {
            severity: Severity::Error,
            category: ErrorCategory::Compilation,
            location: None,
            message: "Compilation failed".to_string(),
            fix_suggestion: None,
        };
        
        let result = ValidationResult::failed(error);
        assert!(!result.passed);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_result_aggregation() {
        let result1 = ValidationResult::new();
        let result2 = ValidationResult::failed(ValidationError {
            severity: Severity::Error,
            category: ErrorCategory::Quality,
            location: None,
            message: "Quality check failed".to_string(),
            fix_suggestion: None,
        });
        
        let aggregated = ValidationResult::aggregate(vec![result1, result2]);
        assert!(!aggregated.passed);
        assert_eq!(aggregated.errors.len(), 1);
    }

    #[test]
    fn test_critical_error_detection() {
        let mut result = ValidationResult::new();
        result.errors.push(ValidationError {
            severity: Severity::Critical,
            category: ErrorCategory::Security,
            location: None,
            message: "Critical security issue".to_string(),
            fix_suggestion: None,
        });
        
        assert!(result.has_critical_errors());
    }
}