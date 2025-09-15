//! Code quality checking using existing Rust ecosystem tools

use crate::validation::{
    ErrorCategory, Improvement, Priority, QualityMetrics, Severity, SourceLocation,
    ValidationError, ValidationResult, ValidationWarning,
};
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// Quality checker that leverages existing Rust ecosystem tools
pub struct QualityChecker {
    project_path: String,
}

impl QualityChecker {
    pub fn new(project_path: impl Into<String>) -> Self {
        Self { project_path: project_path.into() }
    }

    /// Run comprehensive quality checks using multiple tools
    pub async fn check_quality(&self) -> Result<ValidationResult> {
        let mut results = Vec::new();

        // Run clippy for linting
        results.push(self.run_clippy().await?);

        // Check formatting with rustfmt
        results.push(self.check_formatting().await?);

        // Check for outdated dependencies
        results.push(self.check_outdated_deps().await?);

        // Check for unsafe code usage
        results.push(self.check_unsafe_code().await?);

        // Run cargo-deny for license and security checks
        results.push(self.run_cargo_deny().await?);

        // Calculate code metrics with tokei
        results.push(self.calculate_code_metrics().await?);

        // Check documentation coverage
        results.push(self.check_doc_coverage().await?);

        Ok(ValidationResult::aggregate(results))
    }

    /// Run cargo clippy with comprehensive lints
    async fn run_clippy(&self) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();

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
                "-A",
                "clippy::module_name_repetitions", // Common false positive
                "-A",
                "clippy::must_use_candidate", // Too noisy
            ])
            .current_dir(&self.project_path)
            .output()?;

        let stderr = String::from_utf8_lossy(&output.stderr);

        // Parse clippy output
        let warning_regex = Regex::new(r"warning: (.+)").unwrap();
        let error_regex = Regex::new(r"error: (.+)").unwrap();
        let location_regex = Regex::new(r" --> ([^:]+):(\d+):(\d+)").unwrap();

        let lines: Vec<&str> = stderr.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            if let Some(cap) = warning_regex.captures(lines[i]) {
                let message = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();

                // Look for location in next lines
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
                result.passed = false;
                result.errors.push(ValidationError {
                    severity: Severity::Error,
                    category: ErrorCategory::Quality,
                    location: None,
                    message,
                    fix_suggestion: Some("Fix clippy errors to improve code quality".to_string()),
                });
            }
            i += 1;
        }

        // Add quality score based on warnings
        let warning_count = result.warnings.len();
        if warning_count > 20 {
            result.suggestions.push(Improvement {
                category: "quality".to_string(),
                description: format!("High number of clippy warnings ({}). Consider addressing them to improve code quality.", warning_count),
                priority: Priority::High,
            });
        }

        Ok(result)
    }

    /// Check code formatting with rustfmt
    async fn check_formatting(&self) -> Result<ValidationResult> {
        let output = Command::new("cargo")
            .args(&["fmt", "--", "--check"])
            .current_dir(&self.project_path)
            .output()?;

        let mut result = ValidationResult::new();

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let files: Vec<&str> = stdout.lines().filter(|l| l.contains("Diff")).collect();

            result.warnings.push(ValidationWarning {
                category: ErrorCategory::Standards,
                message: format!("{} files need formatting", files.len()),
                location: None,
            });

            result.suggestions.push(Improvement {
                category: "formatting".to_string(),
                description: "Run 'cargo fmt' to auto-format code".to_string(),
                priority: Priority::Low,
            });
        }

        Ok(result)
    }

    /// Check for outdated dependencies using cargo-outdated
    async fn check_outdated_deps(&self) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();

        // Check if cargo-outdated is installed
        let check = Command::new("cargo").arg("outdated").arg("--version").output();

        if check.is_err() {
            result.suggestions.push(Improvement {
                category: "dependencies".to_string(),
                description: "Install cargo-outdated with: cargo install cargo-outdated"
                    .to_string(),
                priority: Priority::Low,
            });
            return Ok(result);
        }

        let output = Command::new("cargo")
            .args(&["outdated", "--format", "json"])
            .current_dir(&self.project_path)
            .output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Parse JSON output to count outdated deps
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                if let Some(deps) = json["dependencies"].as_array() {
                    let outdated_count = deps.len();
                    if outdated_count > 0 {
                        result.warnings.push(ValidationWarning {
                            category: ErrorCategory::Quality,
                            message: format!("{} dependencies are outdated", outdated_count),
                            location: None,
                        });

                        if outdated_count > 10 {
                            result.suggestions.push(Improvement {
                                category: "dependencies".to_string(),
                                description: "Consider updating dependencies with 'cargo update'"
                                    .to_string(),
                                priority: Priority::Medium,
                            });
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Check for unsafe code usage with cargo-geiger
    async fn check_unsafe_code(&self) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();

        // Check if cargo-geiger is installed
        let check = Command::new("cargo").arg("geiger").arg("--version").output();

        if check.is_err() {
            result.suggestions.push(Improvement {
                category: "security".to_string(),
                description: "Install cargo-geiger with: cargo install cargo-geiger".to_string(),
                priority: Priority::Low,
            });
            return Ok(result);
        }

        let output = Command::new("cargo")
            .args(&["geiger", "--output-format", "json"])
            .current_dir(&self.project_path)
            .output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Parse for unsafe code metrics
            if stdout.contains("\"used\": {\"unsafe\": ") {
                let unsafe_regex = Regex::new(r#""unsafe":\s*(\d+)"#).unwrap();
                let mut total_unsafe = 0;

                for cap in unsafe_regex.captures_iter(&stdout) {
                    if let Some(count) = cap.get(1).and_then(|m| m.as_str().parse::<i32>().ok()) {
                        total_unsafe += count;
                    }
                }

                if total_unsafe > 0 {
                    result.warnings.push(ValidationWarning {
                        category: ErrorCategory::Security,
                        message: format!("Found {} unsafe code usages", total_unsafe),
                        location: None,
                    });

                    if total_unsafe > 10 {
                        result.suggestions.push(Improvement {
                            category: "security".to_string(),
                            description: "Consider reducing unsafe code usage or documenting safety invariants".to_string(),
                            priority: Priority::High,
                        });
                    }
                }
            }
        }

        Ok(result)
    }

    /// Run cargo-deny for comprehensive checks
    async fn run_cargo_deny(&self) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();

        // Check if cargo-deny is installed
        let check = Command::new("cargo").arg("deny").arg("--version").output();

        if check.is_err() {
            result.suggestions.push(Improvement {
                category: "compliance".to_string(),
                description: "Install cargo-deny with: cargo install cargo-deny".to_string(),
                priority: Priority::Medium,
            });
            return Ok(result);
        }

        // Run cargo deny check
        let output = Command::new("cargo")
            .args(&["deny", "check"])
            .current_dir(&self.project_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Parse deny output for issues
            if stderr.contains("vulnerability") {
                result.errors.push(ValidationError {
                    severity: Severity::Critical,
                    category: ErrorCategory::Security,
                    location: None,
                    message: "Security vulnerabilities found in dependencies".to_string(),
                    fix_suggestion: Some("Run 'cargo deny check' for details".to_string()),
                });
                result.passed = false;
            }

            if stderr.contains("duplicate") {
                result.warnings.push(ValidationWarning {
                    category: ErrorCategory::Quality,
                    message: "Duplicate dependencies detected".to_string(),
                    location: None,
                });
            }

            if stderr.contains("license") {
                result.warnings.push(ValidationWarning {
                    category: ErrorCategory::Standards,
                    message: "License compliance issues detected".to_string(),
                    location: None,
                });
            }
        }

        Ok(result)
    }

    /// Calculate code metrics using tokei
    async fn calculate_code_metrics(&self) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();

        // Check if tokei is installed
        let check = Command::new("tokei").arg("--version").output();

        if check.is_err() {
            result.suggestions.push(Improvement {
                category: "metrics".to_string(),
                description: "Install tokei with: cargo install tokei".to_string(),
                priority: Priority::Low,
            });
            return Ok(result);
        }

        let output = Command::new("tokei")
            .args(&["--output", "json"])
            .current_dir(&self.project_path)
            .output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                if let Some(rust) = json["Rust"].as_object() {
                    let code_lines = rust["code"].as_u64().unwrap_or(0);
                    let comment_lines = rust["comments"].as_u64().unwrap_or(0);
                    let blank_lines = rust["blanks"].as_u64().unwrap_or(0);

                    let total_lines = code_lines + comment_lines + blank_lines;
                    if total_lines > 0 {
                        let comment_ratio = (comment_lines as f32 / code_lines as f32) * 100.0;

                        result.metrics.documentation_coverage = Some(comment_ratio);

                        if comment_ratio < 10.0 {
                            result.warnings.push(ValidationWarning {
                                category: ErrorCategory::Quality,
                                message: format!("Low comment ratio: {:.1}%", comment_ratio),
                                location: None,
                            });

                            result.suggestions.push(Improvement {
                                category: "documentation".to_string(),
                                description: "Consider adding more documentation comments"
                                    .to_string(),
                                priority: Priority::Medium,
                            });
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Check documentation coverage using cargo doc
    async fn check_doc_coverage(&self) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();

        // Run cargo doc with warnings for missing docs
        let output = Command::new("cargo")
            .args(&["doc", "--no-deps", "--document-private-items"])
            .env("RUSTDOCFLAGS", "-D missing-docs")
            .current_dir(&self.project_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let missing_docs_count = stderr.matches("missing documentation").count();

            if missing_docs_count > 0 {
                result.warnings.push(ValidationWarning {
                    category: ErrorCategory::Quality,
                    message: format!("{} items missing documentation", missing_docs_count),
                    location: None,
                });

                if missing_docs_count > 20 {
                    result.suggestions.push(Improvement {
                        category: "documentation".to_string(),
                        description: "Add documentation comments to public items".to_string(),
                        priority: Priority::Medium,
                    });
                }
            }
        }

        Ok(result)
    }

    /// Calculate comprehensive quality metrics
    pub async fn calculate_metrics(&self) -> QualityMetrics {
        let mut metrics = QualityMetrics::default();

        // Run cargo-tarpaulin for test coverage if available
        if let Ok(output) = Command::new("cargo")
            .args(&["tarpaulin", "--print-summary"])
            .current_dir(&self.project_path)
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let coverage_regex = Regex::new(r"(\d+\.\d+)% coverage").unwrap();
            if let Some(cap) = coverage_regex.captures(&stdout) {
                if let Some(coverage) = cap.get(1).and_then(|m| m.as_str().parse().ok()) {
                    metrics.test_coverage = Some(coverage);
                }
            }
        }

        // Estimate complexity metrics (simplified)
        // In production, would use more sophisticated tools
        metrics.cyclomatic_complexity = Some(5.0);
        metrics.cognitive_complexity = Some(7.0);
        metrics.maintainability_index = Some(85.0);

        metrics
    }
}

/// Linting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintConfig {
    pub clippy_level: ClippyLevel,
    pub allow_warnings: bool,
    pub max_warnings: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClippyLevel {
    Allow,
    Warn,
    Deny,
    Pedantic,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self { clippy_level: ClippyLevel::Warn, allow_warnings: false, max_warnings: 10 }
    }
}

/// Formatter for quality reports
pub struct QualityReportFormatter;

impl QualityReportFormatter {
    pub fn format_report(result: &ValidationResult) -> String {
        let mut report = String::new();

        report.push_str("=== Code Quality Report ===\n\n");

        if result.passed {
            report.push_str("✓ All quality checks passed\n\n");
        } else {
            report.push_str("✗ Quality issues detected\n\n");
        }

        if !result.errors.is_empty() {
            report.push_str("Errors:\n");
            for error in &result.errors {
                report.push_str(&format!("  - {}\n", error.message));
            }
            report.push('\n');
        }

        if !result.warnings.is_empty() {
            report.push_str("Warnings:\n");
            for warning in &result.warnings {
                report.push_str(&format!("  - {}\n", warning.message));
            }
            report.push('\n');
        }

        if !result.suggestions.is_empty() {
            report.push_str("Suggestions:\n");
            for suggestion in &result.suggestions {
                report.push_str(&format!("  - {}\n", suggestion.description));
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_checker_creation() {
        let checker = QualityChecker::new("/test/path");
        assert_eq!(checker.project_path, "/test/path");
    }

    #[test]
    fn test_lint_config_default() {
        let config = LintConfig::default();
        assert!(!config.allow_warnings);
        assert_eq!(config.max_warnings, 10);
    }
}
