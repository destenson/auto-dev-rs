#![allow(unused)]
//! Increment validation and testing

use super::{ExpectedOutcome, IncrementalError, Result, SecurityCheck, TestCase, TestType};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

/// Validates increments through compilation and testing
pub struct IncrementValidator {
    project_root: PathBuf,
}

impl IncrementValidator {
    /// Create a new increment validator
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    /// Validate that the code compiles
    pub async fn validate_compilation(&self) -> Result<ValidationResult> {
        info!("Validating compilation...");

        let output = Command::new("cargo")
            .arg("check")
            .current_dir(&self.project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                IncrementalError::CompilationError(format!("Failed to run cargo check: {}", e))
            })?;

        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let message = if success {
            "Compilation successful".to_string()
        } else {
            format!("Compilation failed:\n{}", stderr)
        };

        debug!("Compilation result: success={}, stdout={}, stderr={}", success, stdout, stderr);

        Ok(ValidationResult {
            success,
            message,
            details: Some(format!("stdout:\n{}\nstderr:\n{}", stdout, stderr)),
        })
    }

    /// Run tests for the increment
    pub async fn run_tests(&self, tests: &[TestCase]) -> Result<Vec<TestResult>> {
        let mut results = Vec::new();

        for test in tests {
            info!("Running test: {}", test.name);
            let result = self.run_single_test(test).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Run a single test case
    async fn run_single_test(&self, test: &TestCase) -> Result<TestResult> {
        let (command, args) = self.parse_command(&test.command);

        let output = Command::new(&command)
            .args(&args)
            .current_dir(&self.project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                IncrementalError::ValidationError(format!("Failed to run test command: {}", e))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        let passed = match &test.expected_outcome {
            ExpectedOutcome::Success => output.status.success(),
            ExpectedOutcome::Failure(expected_msg) => {
                !output.status.success() && stderr.contains(expected_msg)
            }
            ExpectedOutcome::Output(expected) => stdout.contains(expected),
            ExpectedOutcome::Contains(text) => stdout.contains(text) || stderr.contains(text),
        };

        debug!("Test '{}' result: passed={}, exit_code={}", test.name, passed, exit_code);

        Ok(TestResult {
            test_id: test.id.clone(),
            test_name: test.name.clone(),
            passed,
            output: stdout,
            error: stderr,
            exit_code,
            duration: std::time::Duration::from_secs(0), // TODO: Track actual duration
        })
    }

    /// Parse command string into command and arguments
    fn parse_command(&self, command_str: &str) -> (String, Vec<String>) {
        let parts: Vec<String> = command_str.split_whitespace().map(|s| s.to_string()).collect();

        if parts.is_empty() {
            return ("echo".to_string(), vec!["empty command".to_string()]);
        }

        let command = parts[0].clone();
        let args = parts.into_iter().skip(1).collect();

        (command, args)
    }

    /// Run a security check
    pub async fn run_security_check(&self, check: &SecurityCheck) -> Result<SecurityCheckResult> {
        info!("Running security check: {}", check.name);

        let (command, args) = self.parse_command(&check.command);

        let output = Command::new(&command)
            .args(&args)
            .current_dir(&self.project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                IncrementalError::ValidationError(format!("Failed to run security check: {}", e))
            })?;

        let passed = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let message = if passed {
            format!("Security check '{}' passed", check.name)
        } else {
            format!("Security check '{}' failed:\n{}", check.name, stderr)
        };

        Ok(SecurityCheckResult {
            check_name: check.name.clone(),
            passed,
            message,
            details: format!("stdout:\n{}\nstderr:\n{}", stdout, stderr),
        })
    }

    /// Validate increment against all criteria
    pub async fn validate_increment(
        &self,
        increment: &super::Increment,
    ) -> Result<IncrementValidationResult> {
        let mut validation_result = IncrementValidationResult {
            increment_id: increment.id,
            compilation_passed: true,
            tests_passed: Vec::new(),
            tests_failed: Vec::new(),
            security_checks_passed: Vec::new(),
            security_checks_failed: Vec::new(),
            overall_success: true,
        };

        // Check compilation if required
        if increment.validation.must_compile {
            let compilation_result = self.validate_compilation().await?;
            validation_result.compilation_passed = compilation_result.success;
            if !compilation_result.success {
                validation_result.overall_success = false;
                return Ok(validation_result);
            }
        }

        // Run tests
        if !increment.tests.is_empty() {
            let test_results = self.run_tests(&increment.tests).await?;
            for result in test_results {
                if result.passed {
                    validation_result.tests_passed.push(result.test_name);
                } else {
                    validation_result.tests_failed.push(result.test_name);
                    validation_result.overall_success = false;
                }
            }
        }

        // Run security checks
        for check in &increment.validation.security_checks {
            let result = self.run_security_check(check).await?;
            if result.passed {
                validation_result.security_checks_passed.push(result.check_name);
            } else {
                validation_result.security_checks_failed.push(result.check_name);
                validation_result.overall_success = false;
            }
        }

        Ok(validation_result)
    }

    /// Quick validation check (compilation only)
    pub async fn quick_check(&self) -> Result<bool> {
        let result = self.validate_compilation().await?;
        Ok(result.success)
    }
}

/// Result of a validation operation
#[derive(Debug)]
pub struct ValidationResult {
    pub success: bool,
    pub message: String,
    pub details: Option<String>,
}

/// Result of running a test
#[derive(Debug)]
pub struct TestResult {
    pub test_id: String,
    pub test_name: String,
    pub passed: bool,
    pub output: String,
    pub error: String,
    pub exit_code: i32,
    pub duration: std::time::Duration,
}

/// Result of a security check
#[derive(Debug)]
pub struct SecurityCheckResult {
    pub check_name: String,
    pub passed: bool,
    pub message: String,
    pub details: String,
}

/// Complete validation result for an increment
#[derive(Debug)]
pub struct IncrementValidationResult {
    pub increment_id: uuid::Uuid,
    pub compilation_passed: bool,
    pub tests_passed: Vec<String>,
    pub tests_failed: Vec<String>,
    pub security_checks_passed: Vec<String>,
    pub security_checks_failed: Vec<String>,
    pub overall_success: bool,
}

impl IncrementValidationResult {
    /// Get a summary message
    pub fn summary(&self) -> String {
        if self.overall_success {
            format!(
                "Validation successful: {} tests passed, {} security checks passed",
                self.tests_passed.len(),
                self.security_checks_passed.len()
            )
        } else {
            let mut issues = Vec::new();
            if !self.compilation_passed {
                issues.push("compilation failed".to_string());
            }
            if !self.tests_failed.is_empty() {
                issues.push(format!("{} tests failed", self.tests_failed.len()));
            }
            if !self.security_checks_failed.is_empty() {
                issues
                    .push(format!("{} security checks failed", self.security_checks_failed.len()));
            }
            format!("Validation failed: {}", issues.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_parsing() {
        let validator = IncrementValidator::new(PathBuf::from("."));

        let (cmd, args) = validator.parse_command("cargo test --lib");
        assert_eq!(cmd, "cargo");
        assert_eq!(args, vec!["test", "--lib"]);

        let (cmd, args) = validator.parse_command("echo hello world");
        assert_eq!(cmd, "echo");
        assert_eq!(args, vec!["hello", "world"]);
    }
}
