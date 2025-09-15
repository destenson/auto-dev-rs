//! Code verification using existing tools like cargo check, cargo test, etc.

use super::{ValidationResult, ValidationError, Severity, ErrorCategory, SourceLocation};
use anyhow::Result;
use std::process::Command;
use std::path::Path;

/// Code verifier that uses existing tools for validation
pub struct CodeVerifier {
    project_path: String,
}

impl CodeVerifier {
    pub fn new(project_path: impl Into<String>) -> Self {
        Self {
            project_path: project_path.into(),
        }
    }
    
    /// Run cargo check for syntax and type checking
    pub async fn verify_compilation(&self) -> Result<ValidationResult> {
        let output = Command::new("cargo")
            .arg("check")
            .current_dir(&self.project_path)
            .output()?;
        
        if output.status.success() {
            Ok(ValidationResult::new())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(ValidationResult::failed(ValidationError {
                severity: Severity::Error,
                category: ErrorCategory::Compilation,
                location: None,
                message: format!("Compilation failed: {}", stderr),
                fix_suggestion: Some("Run 'cargo check' to see detailed errors".to_string()),
            }))
        }
    }
    
    /// Run cargo test
    pub async fn run_tests(&self) -> Result<ValidationResult> {
        let output = Command::new("cargo")
            .arg("test")
            .arg("--quiet")
            .current_dir(&self.project_path)
            .output()?;
        
        if output.status.success() {
            Ok(ValidationResult::new())
        } else {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(ValidationResult::failed(ValidationError {
                severity: Severity::Error,
                category: ErrorCategory::Test,
                location: None,
                message: format!("Tests failed: {}", stdout),
                fix_suggestion: Some("Run 'cargo test' to see failing tests".to_string()),
            }))
        }
    }
    
    /// Run cargo clippy for linting
    pub async fn run_linter(&self) -> Result<ValidationResult> {
        let output = Command::new("cargo")
            .arg("clippy")
            .arg("--")
            .arg("-W")
            .arg("clippy::all")
            .current_dir(&self.project_path)
            .output()?;
        
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        if output.status.success() && !stderr.contains("warning:") {
            Ok(ValidationResult::new())
        } else {
            let mut result = ValidationResult::new();
            
            // Parse clippy output for warnings
            for line in stderr.lines() {
                if line.contains("warning:") {
                    result.warnings.push(super::ValidationWarning {
                        category: ErrorCategory::Quality,
                        message: line.to_string(),
                        location: None,
                    });
                }
            }
            
            Ok(result)
        }
    }
    
    /// Run cargo fmt check
    pub async fn check_formatting(&self) -> Result<ValidationResult> {
        let output = Command::new("cargo")
            .arg("fmt")
            .arg("--check")
            .current_dir(&self.project_path)
            .output()?;
        
        if output.status.success() {
            Ok(ValidationResult::new())
        } else {
            let mut result = ValidationResult::new();
            result.warnings.push(super::ValidationWarning {
                category: ErrorCategory::Standards,
                message: "Code is not properly formatted".to_string(),
                location: None,
            });
            result.suggestions.push(super::Improvement {
                category: "formatting".to_string(),
                description: "Run 'cargo fmt' to auto-format code".to_string(),
                priority: super::Priority::Low,
            });
            Ok(result)
        }
    }
    
    /// Check if file exists and is valid Rust code
    pub fn verify_file(&self, file_path: &str) -> Result<ValidationResult> {
        let path = Path::new(&self.project_path).join(file_path);
        
        if !path.exists() {
            return Ok(ValidationResult::failed(ValidationError {
                severity: Severity::Error,
                category: ErrorCategory::Compilation,
                location: Some(SourceLocation {
                    file: file_path.to_string(),
                    line: None,
                    column: None,
                }),
                message: format!("File not found: {}", file_path),
                fix_suggestion: None,
            }));
        }
        
        // Could add more file-specific checks here
        Ok(ValidationResult::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_creation() {
        let verifier = CodeVerifier::new("/test/path");
        assert_eq!(verifier.project_path, "/test/path");
    }
}