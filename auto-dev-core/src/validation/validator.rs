//! Specification validation and validation pipeline

use super::{
    ValidationResult, ValidationError, ValidationWarning, ValidationStage,
    ValidationConfig, Severity, ErrorCategory
};
use crate::parser::model::Specification;
use anyhow::Result;
use std::collections::HashMap;

/// Validates code against specifications
pub struct SpecValidator {
    spec: Specification,
}

impl SpecValidator {
    pub fn new(spec: Specification) -> Self {
        Self { spec }
    }
    
    /// Validate that implementation meets specification requirements
    pub async fn validate_compliance(&self, _code_path: &str) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();
        
        // Check that all requirements have corresponding tests
        for requirement in &self.spec.requirements {
            // This would check if tests exist for this requirement
            // For now, just add a warning
            result.warnings.push(ValidationWarning {
                category: ErrorCategory::Specification,
                message: format!("Requirement {} needs validation", requirement.id),
                location: None,
            });
        }
        
        Ok(result)
    }
}

/// Orchestrates validation pipeline
pub struct ValidationPipeline {
    config: ValidationConfig,
    stages: Vec<Box<dyn ValidationStageExecutor>>,
}

impl ValidationPipeline {
    pub fn new(config: ValidationConfig) -> Self {
        let mut stages: Vec<Box<dyn ValidationStageExecutor>> = Vec::new();
        
        // Add stages based on config
        for stage_config in &config.stages {
            if stage_config.enabled {
                match stage_config.stage {
                    ValidationStage::Compilation => {
                        stages.push(Box::new(CompilationStage));
                    }
                    ValidationStage::UnitTests => {
                        stages.push(Box::new(TestStage));
                    }
                    ValidationStage::Linting => {
                        stages.push(Box::new(LintStage));
                    }
                    _ => {}
                }
            }
        }
        
        Self { config, stages }
    }
    
    /// Run the validation pipeline
    pub async fn validate(&self, project_path: &str) -> Result<ValidationResult> {
        let mut results = Vec::new();
        
        for stage in &self.stages {
            let result = stage.execute(project_path).await?;
            
            // Fail fast if configured and stage failed
            if self.config.fail_fast && !result.passed {
                return Ok(result);
            }
            
            results.push(result);
        }
        
        Ok(ValidationResult::aggregate(results))
    }
}

/// Trait for validation stage executors
trait ValidationStageExecutor: Send + Sync {
    fn execute(&self, project_path: &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ValidationResult>> + Send + '_>>;
}

/// Compilation validation stage
struct CompilationStage;

impl ValidationStageExecutor for CompilationStage {
    fn execute(&self, project_path: &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ValidationResult>> + Send + '_>> {
        let project_path = project_path.to_string();
        Box::pin(async move {
            let verifier = super::CodeVerifier::new(project_path);
            verifier.verify_compilation().await
        })
    }
}

/// Test execution stage
struct TestStage;

impl ValidationStageExecutor for TestStage {
    fn execute(&self, project_path: &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ValidationResult>> + Send + '_>> {
        let project_path = project_path.to_string();
        Box::pin(async move {
            let verifier = super::CodeVerifier::new(project_path);
            verifier.run_tests().await
        })
    }
}

/// Linting stage
struct LintStage;

impl ValidationStageExecutor for LintStage {
    fn execute(&self, project_path: &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ValidationResult>> + Send + '_>> {
        let project_path = project_path.to_string();
        Box::pin(async move {
            let verifier = super::CodeVerifier::new(project_path);
            verifier.run_linter().await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_validator_creation() {
        let spec = Specification::new(std::path::PathBuf::from("test_spec.md"));
        let validator = SpecValidator::new(spec);
        assert!(validator.spec.source.to_string_lossy().contains("test_spec"));
    }

    #[test]
    fn test_pipeline_creation() {
        let config = ValidationConfig::default();
        let pipeline = ValidationPipeline::new(config);
        assert!(!pipeline.stages.is_empty());
    }
}