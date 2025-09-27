#![allow(unused)]
use anyhow::{Result, bail};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Guards against unsafe modifications to the codebase
pub struct ModificationGuard {
    /// Critical files that should never be modified
    critical_files: HashSet<PathBuf>,
    /// Patterns for files that require special validation
    validation_patterns: Vec<ValidationRule>,
    /// Maximum file size that can be modified (in bytes)
    max_file_size: usize,
}

impl ModificationGuard {
    pub fn new() -> Self {
        let mut critical_files = HashSet::new();

        // Core files that should never be auto-modified
        critical_files.insert(PathBuf::from("Cargo.toml"));
        critical_files.insert(PathBuf::from("Cargo.lock"));
        critical_files.insert(PathBuf::from("rustfmt.toml"));
        critical_files.insert(PathBuf::from(".gitignore"));
        critical_files.insert(PathBuf::from("LICENSE-MIT"));
        critical_files.insert(PathBuf::from("LICENSE-APACHE"));

        Self {
            critical_files,
            validation_patterns: Self::default_validation_rules(),
            max_file_size: 1_000_000, // 1MB limit
        }
    }

    /// Check if a modification is safe to perform
    pub fn validate_modification(
        &self,
        path: &Path,
        content: Option<&str>,
    ) -> Result<ValidationResult> {
        // Check if file is in critical list
        if self.is_critical_file(path) {
            return Ok(ValidationResult::Denied(
                "Modification of critical file not allowed".to_string(),
            ));
        }

        // Check file extension
        if !self.is_allowed_extension(path) {
            return Ok(ValidationResult::Denied(
                "File type not allowed for modification".to_string(),
            ));
        }

        // If content provided, perform content validation
        if let Some(content) = content {
            if content.len() > self.max_file_size {
                return Ok(ValidationResult::Denied(format!(
                    "File size {} exceeds maximum {}",
                    content.len(),
                    self.max_file_size
                )));
            }

            // Check for dangerous patterns
            if let Some(danger) = self.detect_dangerous_patterns(content) {
                return Ok(ValidationResult::RequiresReview(danger));
            }
        }

        // Apply validation rules
        for rule in &self.validation_patterns {
            if rule.matches(path) {
                return Ok(ValidationResult::RequiresReview(format!(
                    "File matches validation pattern: {}",
                    rule.name
                )));
            }
        }

        Ok(ValidationResult::Allowed)
    }

    /// Check if file is in critical files list
    fn is_critical_file(&self, path: &Path) -> bool {
        // Check exact match
        if self.critical_files.contains(path) {
            return true;
        }

        // Check if it's a critical file in any subdirectory
        if let Some(file_name) = path.file_name() {
            let file_name = file_name.to_string_lossy();
            if file_name == "main.rs" || file_name == "lib.rs" {
                warn!("Attempting to modify critical file: {:?}", path);
                return true;
            }
        }

        false
    }

    /// Check if file extension is allowed for modification
    fn is_allowed_extension(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            matches!(ext.as_str(), "rs" | "toml" | "md" | "txt" | "json" | "yaml" | "yml")
        } else {
            false
        }
    }

    /// Detect potentially dangerous patterns in content
    fn detect_dangerous_patterns(&self, content: &str) -> Option<String> {
        // Check for unsafe code patterns
        if content.contains("unsafe ") && content.contains("ptr::") {
            return Some("Contains unsafe pointer manipulation".to_string());
        }

        // Check for file system operations that could be dangerous
        if content.contains("std::fs::remove") || content.contains("std::fs::delete") {
            return Some("Contains file deletion operations".to_string());
        }

        // Check for network operations
        if content.contains("std::net::") || content.contains("reqwest::") {
            return Some("Contains network operations".to_string());
        }

        // Check for process spawning
        if content.contains("std::process::Command") {
            return Some("Contains process execution".to_string());
        }

        // Check for environment variable manipulation
        if content.contains("std::env::set_var") {
            return Some("Contains environment variable modification".to_string());
        }

        None
    }

    /// Get default validation rules
    fn default_validation_rules() -> Vec<ValidationRule> {
        vec![
            ValidationRule {
                name: "Build Scripts".to_string(),
                pattern: "build.rs".to_string(),
                requires_human_review: true,
            },
            ValidationRule {
                name: "Configuration Files".to_string(),
                pattern: "*.toml".to_string(),
                requires_human_review: false,
            },
            ValidationRule {
                name: "Test Files".to_string(),
                pattern: "**/tests/**".to_string(),
                requires_human_review: false,
            },
        ]
    }

    /// Add a custom validation rule
    pub fn add_validation_rule(&mut self, rule: ValidationRule) {
        self.validation_patterns.push(rule);
    }

    /// Add a critical file that should never be modified
    pub fn add_critical_file(&mut self, path: PathBuf) {
        self.critical_files.insert(path);
    }
}

/// Rule for validating specific file patterns
#[derive(Debug, Clone)]
pub struct ValidationRule {
    pub name: String,
    pub pattern: String,
    pub requires_human_review: bool,
}

impl ValidationRule {
    /// Check if a path matches this rule's pattern
    fn matches(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Simple glob-like pattern matching
        if self.pattern.starts_with("**") {
            let suffix = &self.pattern[2..];
            return path_str.contains(suffix);
        }

        if self.pattern.starts_with("*") {
            let suffix = &self.pattern[1..];
            return path_str.ends_with(suffix);
        }

        path_str.contains(&self.pattern)
    }
}

/// Result of a modification validation
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    /// Modification is allowed
    Allowed,
    /// Modification requires human review
    RequiresReview(String),
    /// Modification is denied
    Denied(String),
}

impl ValidationResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, ValidationResult::Allowed)
    }

    pub fn requires_review(&self) -> bool {
        matches!(self, ValidationResult::RequiresReview(_))
    }

    pub fn is_denied(&self) -> bool {
        matches!(self, ValidationResult::Denied(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_critical_file_detection() {
        let guard = ModificationGuard::new();

        assert!(guard.is_critical_file(&PathBuf::from("Cargo.toml")));
        assert!(guard.is_critical_file(&PathBuf::from("src/main.rs")));
        assert!(guard.is_critical_file(&PathBuf::from("src/lib.rs")));
        assert!(!guard.is_critical_file(&PathBuf::from("src/module.rs")));
    }

    #[test]
    fn test_allowed_extensions() {
        let guard = ModificationGuard::new();

        assert!(guard.is_allowed_extension(&PathBuf::from("file.rs")));
        assert!(guard.is_allowed_extension(&PathBuf::from("config.toml")));
        assert!(guard.is_allowed_extension(&PathBuf::from("README.md")));
        assert!(!guard.is_allowed_extension(&PathBuf::from("binary.exe")));
        assert!(!guard.is_allowed_extension(&PathBuf::from("library.dll")));
    }

    #[test]
    fn test_dangerous_pattern_detection() {
        let guard = ModificationGuard::new();

        let unsafe_code = "unsafe { ptr::write(ptr, value); }";
        assert!(guard.detect_dangerous_patterns(unsafe_code).is_some());

        let delete_code = "std::fs::remove_dir_all(&path)?;";
        assert!(guard.detect_dangerous_patterns(delete_code).is_some());

        let safe_code = "let result = calculate_sum(a, b);";
        assert!(guard.detect_dangerous_patterns(safe_code).is_none());
    }

    #[test]
    fn test_validation_rules() {
        let rule = ValidationRule {
            name: "Test".to_string(),
            pattern: "**/tests/**".to_string(),
            requires_human_review: false,
        };

        assert!(rule.matches(&PathBuf::from("src/tests/unit.rs")));
        assert!(rule.matches(&PathBuf::from("module/tests/integration.rs")));
        assert!(!rule.matches(&PathBuf::from("src/main.rs")));
    }
}
