#![allow(unused)]
//! Implementation validator

use super::{PipelineContext, PipelineStage};
use crate::{debug, info};
use crate::synthesis::{Result, SynthesisError};
use async_trait::async_trait;
use std::process::Command;

/// Validates generated implementation
pub struct ImplementationValidator {
    validators: Vec<Box<dyn Validator>>,
}

impl ImplementationValidator {
    /// Create a new implementation validator
    pub fn new() -> Self {
        Self {
            validators: vec![
                Box::new(CompilationValidator),
                Box::new(TestValidator),
                Box::new(RequirementValidator),
                Box::new(LintValidator),
            ],
        }
    }

    /// Run all validators
    async fn validate_all(&self, context: &PipelineContext) -> ValidationResult {
        let mut result = ValidationResult::new();

        for validator in &self.validators {
            let validation = validator.validate(context).await;
            result.merge(validation);

            // Stop on critical failures
            if result.has_critical_errors() && context.config.incremental {
                break;
            }
        }

        result
    }
}

#[async_trait]
impl PipelineStage for ImplementationValidator {
    fn name(&self) -> &'static str {
        "ImplementationValidator"
    }

    async fn execute(&self, mut context: PipelineContext) -> Result<PipelineContext> {
        info!("Validating implementation");

        context.metadata.current_stage = self.name().to_string();

        // Run validation
        let validation_result = self.validate_all(&context).await;

        // Update coverage based on validation
        if let Some(ref coverage) = validation_result.coverage_update {
            context.coverage = coverage.clone();
        }

        // Add warnings for validation issues
        for issue in &validation_result.issues {
            context.add_warning(format!("{:?}: {}", issue.severity, issue.message));
        }

        // Fail if critical errors
        if validation_result.has_critical_errors() {
            return Err(SynthesisError::ValidationError(format!(
                "{} critical validation errors found",
                validation_result.critical_error_count()
            )));
        }

        debug!(
            "Validation complete: {} passed, {} warnings, {} errors",
            validation_result.passed_count(),
            validation_result.warning_count(),
            validation_result.error_count()
        );

        Ok(context)
    }
}

/// Validator trait
#[async_trait]
trait Validator: Send + Sync {
    async fn validate(&self, context: &PipelineContext) -> ValidationResult;
}

/// Compilation validator
struct CompilationValidator;

#[async_trait]
impl Validator for CompilationValidator {
    async fn validate(&self, context: &PipelineContext) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Run cargo check
        let output = Command::new("cargo").arg("check").arg("--message-format=json").output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    result.add_success("Compilation check passed");
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    result.add_error(format!("Compilation failed: {}", stderr));
                }
            }
            Err(e) => {
                result.add_warning(format!("Could not run cargo check: {}", e));
            }
        }

        result
    }
}

/// Test validator
struct TestValidator;

#[async_trait]
impl Validator for TestValidator {
    async fn validate(&self, context: &PipelineContext) -> ValidationResult {
        let mut result = ValidationResult::new();

        if !context.config.test_first {
            result.add_info("Test validation skipped (test_first disabled)");
            return result;
        }

        // Run cargo test
        let output = Command::new("cargo").arg("test").arg("--quiet").output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    result.add_success("All tests passed");
                } else {
                    result.add_warning("Some tests failed");
                }
            }
            Err(e) => {
                result.add_warning(format!("Could not run tests: {}", e));
            }
        }

        result
    }
}

/// Requirement validator
struct RequirementValidator;

#[async_trait]
impl Validator for RequirementValidator {
    async fn validate(&self, context: &PipelineContext) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Check requirement coverage
        let total_reqs = context.spec.requirements.len();
        let completed_tasks = context.completed_tasks.len();

        if completed_tasks >= total_reqs {
            result.add_success(format!("All {} requirements implemented", total_reqs));
        } else {
            let percentage = (completed_tasks as f32 / total_reqs as f32) * 100.0;
            result.add_info(format!(
                "Requirement coverage: {:.1}% ({}/{})",
                percentage, completed_tasks, total_reqs
            ));
        }

        // Update coverage report
        let mut coverage = context.coverage.clone();
        if let Some(spec_coverage) = coverage.specifications.get_mut(&context.spec.source) {
            spec_coverage.implemented_requirements = completed_tasks;
        }
        result.coverage_update = Some(coverage);

        result
    }
}

/// Lint validator
struct LintValidator;

#[async_trait]
impl Validator for LintValidator {
    async fn validate(&self, _context: &PipelineContext) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Run clippy
        let output = Command::new("cargo")
            .arg("clippy")
            .arg("--quiet")
            .arg("--")
            .arg("-D")
            .arg("warnings")
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    result.add_success("No clippy warnings");
                } else {
                    result.add_warning("Clippy found issues");
                }
            }
            Err(e) => {
                result.add_info(format!("Could not run clippy: {}", e));
            }
        }

        result
    }
}

/// Validation result
struct ValidationResult {
    issues: Vec<ValidationIssue>,
    coverage_update: Option<crate::synthesis::coverage::CoverageReport>,
}

impl ValidationResult {
    fn new() -> Self {
        Self { issues: Vec::new(), coverage_update: None }
    }

    fn add_success(&mut self, message: impl Into<String>) {
        self.issues.push(ValidationIssue { severity: Severity::Success, message: message.into() });
    }

    fn add_info(&mut self, message: impl Into<String>) {
        self.issues.push(ValidationIssue { severity: Severity::Info, message: message.into() });
    }

    fn add_warning(&mut self, message: impl Into<String>) {
        self.issues.push(ValidationIssue { severity: Severity::Warning, message: message.into() });
    }

    fn add_error(&mut self, message: impl Into<String>) {
        self.issues.push(ValidationIssue { severity: Severity::Error, message: message.into() });
    }

    fn merge(&mut self, other: ValidationResult) {
        self.issues.extend(other.issues);
        if let Some(coverage) = other.coverage_update {
            self.coverage_update = Some(coverage);
        }
    }

    fn has_critical_errors(&self) -> bool {
        self.issues.iter().any(|i| matches!(i.severity, Severity::Error))
    }

    fn critical_error_count(&self) -> usize {
        self.issues.iter().filter(|i| matches!(i.severity, Severity::Error)).count()
    }

    fn passed_count(&self) -> usize {
        self.issues.iter().filter(|i| matches!(i.severity, Severity::Success)).count()
    }

    fn warning_count(&self) -> usize {
        self.issues.iter().filter(|i| matches!(i.severity, Severity::Warning)).count()
    }

    fn error_count(&self) -> usize {
        self.issues.iter().filter(|i| matches!(i.severity, Severity::Error)).count()
    }
}

/// Validation issue
struct ValidationIssue {
    severity: Severity,
    message: String,
}

/// Issue severity
#[derive(Debug)]
enum Severity {
    Success,
    Info,
    Warning,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::new();

        result.add_success("Test passed");
        result.add_warning("Minor issue");
        result.add_error("Critical problem");

        assert_eq!(result.passed_count(), 1);
        assert_eq!(result.warning_count(), 1);
        assert_eq!(result.error_count(), 1);
        assert!(result.has_critical_errors());
    }
}
