//! Specification validation and validation pipeline

use super::{
    ErrorCategory, Improvement, Priority, QualityMetrics, Severity, SourceLocation,
    ValidationConfig, ValidationError, ValidationResult, ValidationStage, ValidationWarning,
};
use crate::parser::model::{Requirement, Specification};
use anyhow::Result;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;

/// Validates code against specifications
pub struct SpecValidator {
    spec: Specification,
    project_path: PathBuf,
}

impl SpecValidator {
    pub fn new(spec: Specification, project_path: impl AsRef<Path>) -> Self {
        Self { spec, project_path: project_path.as_ref().to_path_buf() }
    }

    /// Validate that implementation meets specification requirements
    pub async fn validate_compliance(&self, code_path: &str) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();

        // Validate each requirement
        for requirement in &self.spec.requirements {
            let req_result = self.validate_requirement(requirement, code_path).await?;
            result = ValidationResult::aggregate(vec![result, req_result]);
        }

        // Check for missing implementations
        let missing = self.find_missing_implementations(code_path).await?;
        for missing_item in missing {
            result.errors.push(ValidationError {
                severity: Severity::Error,
                category: ErrorCategory::Specification,
                location: None,
                message: format!("Missing implementation: {}", missing_item),
                fix_suggestion: Some(format!("Implement {}", missing_item)),
            });
        }

        // Calculate specification coverage
        let coverage = self.calculate_spec_coverage(&result);
        result.metrics.test_coverage = Some(coverage);

        if coverage < 90.0 {
            result.warnings.push(ValidationWarning {
                category: ErrorCategory::Specification,
                message: format!("Specification coverage is {:.1}% (target: 90%)", coverage),
                location: None,
            });
        }

        Ok(result)
    }

    /// Validate a single requirement
    async fn validate_requirement(
        &self,
        requirement: &Requirement,
        code_path: &str,
    ) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();

        // Check if tests exist for this requirement
        let test_exists = self.check_test_exists(&requirement.id, code_path).await?;
        if !test_exists {
            result.warnings.push(ValidationWarning {
                category: ErrorCategory::Specification,
                message: format!("No test found for requirement: {}", requirement.id),
                location: None,
            });
        }

        // Check if implementation exists
        let impl_exists = self.check_implementation_exists(&requirement, code_path).await?;
        if !impl_exists {
            result.errors.push(ValidationError {
                severity: Severity::Error,
                category: ErrorCategory::Specification,
                location: None,
                message: format!("Requirement not implemented: {}", requirement.id),
                fix_suggestion: Some(format!("Implement requirement: {}", requirement.description)),
            });
            result.passed = false;
        }

        // Validate acceptance criteria if present
        if !requirement.acceptance_criteria.is_empty() {
            for (i, criteria) in requirement.acceptance_criteria.iter().enumerate() {
                if !self.validate_acceptance_criteria(criteria, code_path).await? {
                    result.warnings.push(ValidationWarning {
                        category: ErrorCategory::Specification,
                        message: format!(
                            "Acceptance criteria {} for requirement {} not verifiable",
                            i + 1,
                            requirement.id
                        ),
                        location: None,
                    });
                }
            }
        }

        Ok(result)
    }

    /// Check if tests exist for a requirement
    async fn check_test_exists(&self, requirement_id: &str, code_path: &str) -> Result<bool> {
        let test_path = Path::new(code_path).join("tests");
        if !test_path.exists() {
            return Ok(false);
        }

        // Search for test files mentioning the requirement
        let pattern = format!("test.*{}", requirement_id.to_lowercase().replace(".", "_"));
        let regex = Regex::new(&pattern).unwrap_or_else(|_| Regex::new("test").unwrap());

        let mut entries = fs::read_dir(test_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                if regex.is_match(name) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Check if implementation exists for a requirement
    async fn check_implementation_exists(
        &self,
        requirement: &Requirement,
        code_path: &str,
    ) -> Result<bool> {
        // This is a simplified check - in reality would do more sophisticated analysis
        let src_path = Path::new(code_path).join("src");
        if !src_path.exists() {
            return Ok(false);
        }

        // For now, just check if any source files exist
        let mut entries = fs::read_dir(src_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            if let Some(ext) = entry.path().extension() {
                if ext == "rs" {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Validate acceptance criteria
    async fn validate_acceptance_criteria(&self, criteria: &str, _code_path: &str) -> Result<bool> {
        // This would implement actual acceptance criteria validation
        // For now, return true if criteria is not empty
        Ok(!criteria.trim().is_empty())
    }

    /// Find missing implementations
    async fn find_missing_implementations(&self, _code_path: &str) -> Result<Vec<String>> {
        let mut missing = Vec::new();

        // Check for required components based on specification
        for requirement in &self.spec.requirements {
            if requirement.priority == crate::parser::model::Priority::Critical {
                // Simplified check - would be more sophisticated in practice
                if requirement.description.contains("API") {
                    // Check if API exists
                    // For now, just placeholder
                }
            }
        }

        Ok(missing)
    }

    /// Calculate specification coverage percentage
    fn calculate_spec_coverage(&self, result: &ValidationResult) -> f32 {
        let total_requirements = self.spec.requirements.len() as f32;
        if total_requirements == 0.0 {
            return 100.0;
        }

        let failed_requirements =
            result.errors.iter().filter(|e| e.category == ErrorCategory::Specification).count()
                as f32;

        ((total_requirements - failed_requirements) / total_requirements) * 100.0
    }

    /// Validate behavioral requirements
    pub async fn validate_behavior(
        &self,
        implementation: &str,
        spec: &Specification,
    ) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();

        // Extract behavioral requirements from specification
        for requirement in &spec.requirements {
            if let Some(behavior) = self.extract_behavior(&requirement) {
                let scenario_result =
                    self.validate_behavior_scenario(&behavior, implementation).await?;
                result = ValidationResult::aggregate(vec![result, scenario_result]);
            }
        }

        Ok(result)
    }

    /// Extract behavioral requirement from a requirement
    fn extract_behavior(&self, requirement: &Requirement) -> Option<BehaviorScenario> {
        // Parse Given/When/Then from requirement description or acceptance criteria
        for criteria in &requirement.acceptance_criteria {
            if criteria.contains("Given") && criteria.contains("When") && criteria.contains("Then")
            {
                return Some(BehaviorScenario {
                    given: self.extract_section(criteria, "Given"),
                    when: self.extract_section(criteria, "When"),
                    then: self.extract_section(criteria, "Then"),
                });
            }
        }
        None
    }

    /// Extract a section from Gherkin-style text
    fn extract_section(&self, text: &str, section: &str) -> String {
        let regex = Regex::new(&format!(r"{} ([^\n]+)", section)).unwrap();
        regex
            .captures(text)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default()
    }

    /// Validate a behavior scenario
    async fn validate_behavior_scenario(
        &self,
        scenario: &BehaviorScenario,
        _implementation: &str,
    ) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();

        // This would actually execute the scenario against the implementation
        // For now, just add a suggestion
        result.suggestions.push(Improvement {
            category: "behavior".to_string(),
            description: format!(
                "Validate scenario: Given {} When {} Then {}",
                scenario.given, scenario.when, scenario.then
            ),
            priority: Priority::Medium,
        });

        Ok(result)
    }
}

/// Represents a behavioral scenario
#[derive(Debug, Clone)]
struct BehaviorScenario {
    given: String,
    when: String,
    then: String,
}

/// Represents generated code to be validated
#[derive(Debug, Clone)]
pub struct GeneratedCode {
    pub file_path: String,
    pub content: String,
    pub language: String,
}

/// Orchestrates validation pipeline
pub struct ValidationPipeline {
    config: ValidationConfig,
    stages: Vec<Box<dyn ValidationStageExecutor>>,
    project_path: PathBuf,
}

impl ValidationPipeline {
    pub fn new(config: ValidationConfig, project_path: impl AsRef<Path>) -> Self {
        let project_path = project_path.as_ref().to_path_buf();
        let mut stages: Vec<Box<dyn ValidationStageExecutor>> = Vec::new();

        // Add stages based on config
        for stage_config in &config.stages {
            if stage_config.enabled {
                match stage_config.stage {
                    ValidationStage::SyntaxCheck => {
                        stages.push(Box::new(SyntaxStage { timeout: stage_config.timeout }));
                    }
                    ValidationStage::Compilation => {
                        stages.push(Box::new(CompilationStage { timeout: stage_config.timeout }));
                    }
                    ValidationStage::UnitTests => {
                        stages.push(Box::new(TestStage { timeout: stage_config.timeout }));
                    }
                    ValidationStage::Linting => {
                        stages.push(Box::new(LintStage { timeout: stage_config.timeout }));
                    }
                    ValidationStage::Security => {
                        stages.push(Box::new(SecurityStage { timeout: stage_config.timeout }));
                    }
                    ValidationStage::Performance => {
                        stages.push(Box::new(PerformanceStage { timeout: stage_config.timeout }));
                    }
                    ValidationStage::Specification => {
                        stages.push(Box::new(SpecificationStage { timeout: stage_config.timeout }));
                    }
                    ValidationStage::Integration => {
                        stages.push(Box::new(IntegrationStage { timeout: stage_config.timeout }));
                    }
                }
            }
        }

        Self { config, stages, project_path }
    }

    /// Run the validation pipeline
    pub async fn validate(&self, code: &GeneratedCode) -> Result<ValidationResult> {
        let mut results = Vec::new();
        let start_time = std::time::Instant::now();

        // Execute stages in parallel if configured
        if self.config.parallel {
            // Note: Parallel execution is not yet implemented
            // For now, fall through to sequential execution
        }

        {
            // Sequential execution
            for stage in &self.stages {
                let result = stage.execute(&self.project_path.to_string_lossy()).await?;

                if self.config.fail_fast && !result.passed {
                    return Ok(result);
                }

                results.push(result);
            }
        }

        let mut final_result = ValidationResult::aggregate(results);

        // Add execution time metric
        let duration = start_time.elapsed();
        final_result.suggestions.push(Improvement {
            category: "performance".to_string(),
            description: format!("Validation completed in {:.2}s", duration.as_secs_f64()),
            priority: Priority::Low,
        });

        Ok(final_result)
    }

    /// Validate a specific file
    pub async fn validate_file(&self, file_path: &str) -> Result<ValidationResult> {
        let code = GeneratedCode {
            file_path: file_path.to_string(),
            content: String::new(),
            language: "rust".to_string(),
        };

        self.validate(&code).await
    }
}

/// Trait for validation stage executors
trait ValidationStageExecutor: Send + Sync {
    fn execute(
        &self,
        project_path: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ValidationResult>> + Send + '_>>;
    fn name(&self) -> &str;
}

/// Syntax checking stage
struct SyntaxStage {
    timeout: Duration,
}

impl ValidationStageExecutor for SyntaxStage {
    fn execute(
        &self,
        project_path: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ValidationResult>> + Send + '_>>
    {
        let project_path = project_path.to_string();
        Box::pin(async move {
            // Quick syntax check using cargo check --no-deps
            let verifier = super::CodeVerifier::new(project_path);
            verifier.verify_compilation().await
        })
    }

    fn name(&self) -> &str {
        "SyntaxCheck"
    }
}

/// Compilation validation stage
struct CompilationStage {
    timeout: Duration,
}

impl ValidationStageExecutor for CompilationStage {
    fn execute(
        &self,
        project_path: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ValidationResult>> + Send + '_>>
    {
        let project_path = project_path.to_string();
        let timeout = self.timeout;
        Box::pin(async move {
            let verifier = super::CodeVerifier::new(project_path).with_timeout(timeout);
            verifier.verify_compilation().await
        })
    }

    fn name(&self) -> &str {
        "Compilation"
    }
}

/// Test execution stage
struct TestStage {
    timeout: Duration,
}

impl ValidationStageExecutor for TestStage {
    fn execute(
        &self,
        project_path: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ValidationResult>> + Send + '_>>
    {
        let project_path = project_path.to_string();
        let timeout = self.timeout;
        Box::pin(async move {
            let verifier = super::CodeVerifier::new(project_path).with_timeout(timeout);
            verifier.run_tests().await
        })
    }

    fn name(&self) -> &str {
        "UnitTests"
    }
}

/// Linting stage
struct LintStage {
    timeout: Duration,
}

impl ValidationStageExecutor for LintStage {
    fn execute(
        &self,
        project_path: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ValidationResult>> + Send + '_>>
    {
        let project_path = project_path.to_string();
        Box::pin(async move {
            let verifier = super::CodeVerifier::new(project_path);
            verifier.run_linter().await
        })
    }

    fn name(&self) -> &str {
        "Linting"
    }
}

/// Security validation stage
struct SecurityStage {
    timeout: Duration,
}

impl ValidationStageExecutor for SecurityStage {
    fn execute(
        &self,
        project_path: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ValidationResult>> + Send + '_>>
    {
        let project_path = project_path.to_string();
        Box::pin(async move {
            let scanner = super::security::SecurityScanner::new(project_path);
            scanner.scan_dependencies().await
        })
    }

    fn name(&self) -> &str {
        "Security"
    }
}

/// Performance validation stage
struct PerformanceStage {
    timeout: Duration,
}

impl ValidationStageExecutor for PerformanceStage {
    fn execute(
        &self,
        project_path: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ValidationResult>> + Send + '_>>
    {
        let project_path = project_path.to_string();
        Box::pin(async move {
            let validator = super::performance::PerformanceValidator::new(project_path);
            validator.run_benchmarks().await
        })
    }

    fn name(&self) -> &str {
        "Performance"
    }
}

/// Specification validation stage
struct SpecificationStage {
    timeout: Duration,
}

impl ValidationStageExecutor for SpecificationStage {
    fn execute(
        &self,
        project_path: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ValidationResult>> + Send + '_>>
    {
        let project_path = project_path.to_string();
        Box::pin(async move {
            // Would load specification and validate
            // For now, return empty result
            Ok(ValidationResult::new())
        })
    }

    fn name(&self) -> &str {
        "Specification"
    }
}

/// Integration test stage
struct IntegrationStage {
    timeout: Duration,
}

impl ValidationStageExecutor for IntegrationStage {
    fn execute(
        &self,
        project_path: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ValidationResult>> + Send + '_>>
    {
        let project_path = project_path.to_string();
        Box::pin(async move {
            // Would run integration tests
            // For now, return empty result
            Ok(ValidationResult::new())
        })
    }

    fn name(&self) -> &str {
        "Integration"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_validator_creation() {
        let spec = Specification::new(std::path::PathBuf::from("test_spec.md"));
        let validator = SpecValidator::new(spec, "/test/path");
        assert!(validator.spec.source.to_string_lossy().contains("test_spec"));
    }

    #[test]
    fn test_pipeline_creation() {
        let config = ValidationConfig::default();
        let pipeline = ValidationPipeline::new(config, "/test/path");
        assert!(!pipeline.stages.is_empty());
    }
}
