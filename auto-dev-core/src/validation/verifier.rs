//! Code verification using existing tools like cargo check, cargo test, etc.

use super::{
    ErrorCategory, QualityMetrics, Severity, SourceLocation, ValidationError, ValidationResult,
    ValidationWarning,
};
use anyhow::Result;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;

/// Code verifier that uses existing tools for validation
pub struct CodeVerifier {
    project_path: String,
    timeout_duration: Duration,
    parallel: bool,
}

impl CodeVerifier {
    pub fn new(project_path: impl Into<String>) -> Self {
        Self {
            project_path: project_path.into(),
            timeout_duration: Duration::from_secs(60),
            parallel: true,
        }
    }

    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.timeout_duration = duration;
        self
    }

    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    /// Run cargo check for syntax and type checking
    pub async fn verify_compilation(&self) -> Result<ValidationResult> {
        let output = Command::new("cargo")
            .arg("check")
            .arg("--all-targets")
            .arg("--all-features")
            .current_dir(&self.project_path)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()?;

        if output.status.success() {
            Ok(ValidationResult::new())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let errors = self.parse_compilation_errors(&stderr);

            let mut result = ValidationResult::new();
            result.passed = false;
            result.errors = errors;

            Ok(result)
        }
    }

    /// Parse compilation errors from cargo output
    fn parse_compilation_errors(&self, output: &str) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let error_regex = Regex::new(r"error\[([^\]]+)\]: (.+)").unwrap();
        let location_regex = Regex::new(r" --> ([^:]+):(\d+):(\d+)").unwrap();

        let lines: Vec<&str> = output.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            if let Some(cap) = error_regex.captures(lines[i]) {
                let error_code = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let message = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();

                // Look for location in next few lines
                let mut location = None;
                for j in 1..=3 {
                    if i + j < lines.len() {
                        if let Some(loc_cap) = location_regex.captures(lines[i + j]) {
                            location = Some(SourceLocation {
                                file: loc_cap
                                    .get(1)
                                    .map(|m| m.as_str().to_string())
                                    .unwrap_or_default(),
                                line: loc_cap.get(2).and_then(|m| m.as_str().parse().ok()),
                                column: loc_cap.get(3).and_then(|m| m.as_str().parse().ok()),
                            });
                            break;
                        }
                    }
                }

                errors.push(ValidationError {
                    severity: Severity::Error,
                    category: ErrorCategory::Compilation,
                    location,
                    message: format!("{}: {}", error_code, message),
                    fix_suggestion: self.suggest_fix_for_error(error_code),
                });
            }
            i += 1;
        }

        // If no specific errors were parsed, add a general one
        if errors.is_empty() && !output.trim().is_empty() {
            errors.push(ValidationError {
                severity: Severity::Error,
                category: ErrorCategory::Compilation,
                location: None,
                message: "Compilation failed".to_string(),
                fix_suggestion: Some("Run 'cargo check' to see detailed errors".to_string()),
            });
        }

        errors
    }

    /// Suggest fixes for common error codes
    fn suggest_fix_for_error(&self, error_code: &str) -> Option<String> {
        match error_code {
            "E0425" => {
                Some("Cannot find value in scope. Check imports and variable names.".to_string())
            }
            "E0433" => {
                Some("Failed to resolve. Add 'use' statement or check module path.".to_string())
            }
            "E0308" => Some("Type mismatch. Check expected vs actual types.".to_string()),
            "E0599" => {
                Some("No method found. Check if trait is in scope or method exists.".to_string())
            }
            "E0277" => {
                Some("Trait not satisfied. Implement required trait or check bounds.".to_string())
            }
            _ => None,
        }
    }

    /// Run cargo test with detailed parsing
    pub async fn run_tests(&self) -> Result<ValidationResult> {
        let mut cmd = Command::new("cargo");
        cmd.arg("test")
            .arg("--all")
            .arg("--")
            .arg("--nocapture")
            .current_dir(&self.project_path)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped());

        if !self.parallel {
            cmd.arg("--test-threads=1");
        }

        let output = cmd.output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            let mut result = ValidationResult::new();

            // Parse test results for metrics
            if let Some(cap) = Regex::new(r"test result: .+ (\d+) passed.+ (\d+) failed")
                .unwrap()
                .captures(&stdout)
            {
                let passed = cap.get(1).and_then(|m| m.as_str().parse::<f32>().ok()).unwrap_or(0.0);
                let failed = cap.get(2).and_then(|m| m.as_str().parse::<f32>().ok()).unwrap_or(0.0);
                let total = passed + failed;
                if total > 0.0 {
                    result.metrics.test_coverage = Some((passed / total) * 100.0);
                }
            }

            Ok(result)
        } else {
            let failures = self.parse_test_failures(&stdout);

            let mut result = ValidationResult::new();
            result.passed = false;
            result.errors = failures;

            Ok(result)
        }
    }

    /// Parse test failures from cargo test output
    fn parse_test_failures(&self, output: &str) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let failure_regex = Regex::new(r"---- ([^ ]+) stdout ----").unwrap();

        for cap in failure_regex.captures_iter(output) {
            let test_name = cap.get(1).map(|m| m.as_str()).unwrap_or("unknown");

            errors.push(ValidationError {
                severity: Severity::Error,
                category: ErrorCategory::Test,
                location: None,
                message: format!("Test '{}' failed", test_name),
                fix_suggestion: Some(format!(
                    "Run 'cargo test {}' to debug this specific test",
                    test_name
                )),
            });
        }

        if errors.is_empty() && !output.contains("test result: ok") {
            errors.push(ValidationError {
                severity: Severity::Error,
                category: ErrorCategory::Test,
                location: None,
                message: "Tests failed".to_string(),
                fix_suggestion: Some("Run 'cargo test' to see detailed failures".to_string()),
            });
        }

        errors
    }

    /// Run cargo clippy for linting with detailed parsing
    pub async fn run_linter(&self) -> Result<ValidationResult> {
        let output = Command::new("cargo")
            .args(&[
                "clippy",
                "--all-targets",
                "--all-features",
                "--",
                "-W",
                "clippy::all",
                "-W",
                "clippy::pedantic",
                "-W",
                "clippy::nursery",
                "-W",
                "clippy::cargo",
            ])
            .current_dir(&self.project_path)
            .stderr(Stdio::piped())
            .output()?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut result = ValidationResult::new();

        // Parse warnings and errors
        let warning_regex = Regex::new(r"warning: (.+)").unwrap();
        let error_regex = Regex::new(r"error: (.+)").unwrap();
        let location_regex = Regex::new(r" --> ([^:]+):(\d+):(\d+)").unwrap();

        let lines: Vec<&str> = stderr.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            if let Some(cap) = warning_regex.captures(lines[i]) {
                let message = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();

                // Look for location
                let mut location = None;
                for j in 1..=3 {
                    if i + j < lines.len() {
                        if let Some(loc_cap) = location_regex.captures(lines[i + j]) {
                            location = Some(SourceLocation {
                                file: loc_cap
                                    .get(1)
                                    .map(|m| m.as_str().to_string())
                                    .unwrap_or_default(),
                                line: loc_cap.get(2).and_then(|m| m.as_str().parse().ok()),
                                column: loc_cap.get(3).and_then(|m| m.as_str().parse().ok()),
                            });
                            break;
                        }
                    }
                }

                result.warnings.push(ValidationWarning {
                    category: ErrorCategory::Quality,
                    message,
                    location,
                });
            } else if let Some(cap) = error_regex.captures(lines[i]) {
                let message = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();

                // Look for location
                let mut location = None;
                for j in 1..=3 {
                    if i + j < lines.len() {
                        if let Some(loc_cap) = location_regex.captures(lines[i + j]) {
                            location = Some(SourceLocation {
                                file: loc_cap
                                    .get(1)
                                    .map(|m| m.as_str().to_string())
                                    .unwrap_or_default(),
                                line: loc_cap.get(2).and_then(|m| m.as_str().parse().ok()),
                                column: loc_cap.get(3).and_then(|m| m.as_str().parse().ok()),
                            });
                            break;
                        }
                    }
                }

                result.passed = false;
                result.errors.push(ValidationError {
                    severity: Severity::Error,
                    category: ErrorCategory::Quality,
                    location,
                    message,
                    fix_suggestion: Some("Fix clippy errors to improve code quality".to_string()),
                });
            }
            i += 1;
        }

        // Add quality suggestions based on clippy results
        if result.warnings.len() > 10 {
            result.suggestions.push(super::Improvement {
                category: "quality".to_string(),
                description: format!(
                    "Consider addressing {} clippy warnings to improve code quality",
                    result.warnings.len()
                ),
                priority: super::Priority::Medium,
            });
        }

        Ok(result)
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

        // Check file is valid UTF-8
        if std::fs::read_to_string(&path).is_err() {
            return Ok(ValidationResult::failed(ValidationError {
                severity: Severity::Error,
                category: ErrorCategory::Compilation,
                location: Some(SourceLocation {
                    file: file_path.to_string(),
                    line: None,
                    column: None,
                }),
                message: format!("File is not valid UTF-8: {}", file_path),
                fix_suggestion: Some("Ensure file encoding is UTF-8".to_string()),
            }));
        }

        Ok(ValidationResult::new())
    }

    /// Verify all code in the project
    pub async fn verify_all(&self) -> Result<ValidationResult> {
        let mut results = Vec::new();

        // Run all verifications
        results.push(self.verify_compilation().await?);
        results.push(self.run_tests().await?);
        results.push(self.run_linter().await?);
        results.push(self.check_formatting().await?);

        Ok(ValidationResult::aggregate(results))
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
