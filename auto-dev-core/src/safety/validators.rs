//! Individual validation gate implementations

use super::gates::{GateResult, ValidationGate};
use super::{CodeModification, ModificationType, RiskLevel};
use async_trait::async_trait;
use regex::Regex;
use std::collections::HashSet;
use tracing::{debug, warn};

/// Static analysis validation gate
pub struct StaticValidator {
    unsafe_patterns: Vec<Regex>,
    panic_patterns: Vec<Regex>,
}

impl StaticValidator {
    pub fn new() -> Self {
        Self {
            unsafe_patterns: vec![
                Regex::new(r"\bunsafe\s*\{").unwrap(),
                Regex::new(r"std::mem::transmute").unwrap(),
                Regex::new(r"std::ptr::\w+").unwrap(),
            ],
            panic_patterns: vec![
                Regex::new(r"\bpanic!\s*\(").unwrap(),
                Regex::new(r"\bunwrap\(\)").unwrap(),
                Regex::new(r"\bexpect\(").unwrap(),
                Regex::new(r"\bunreachable!\s*\(").unwrap(),
            ],
        }
    }

    fn check_unsafe_code(&self, content: &str) -> Vec<String> {
        let mut issues = Vec::new();

        for pattern in &self.unsafe_patterns {
            if pattern.is_match(content) {
                issues.push(format!("Unsafe code pattern detected: {}", pattern.as_str()));
            }
        }

        issues
    }

    fn check_panic_patterns(&self, content: &str) -> Vec<String> {
        let mut issues = Vec::new();

        for pattern in &self.panic_patterns {
            let matches: Vec<_> = pattern.find_iter(content).collect();
            if !matches.is_empty() {
                issues.push(format!(
                    "Panic pattern '{}' found {} times",
                    pattern.as_str(),
                    matches.len()
                ));
            }
        }

        issues
    }
}

#[async_trait]
impl ValidationGate for StaticValidator {
    fn name(&self) -> String {
        "StaticAnalysis".to_string()
    }

    async fn validate(&self, modification: &CodeModification) -> GateResult {
        debug!("Running static analysis on {}", modification.file_path.display());

        let mut issues = Vec::new();
        let mut suggestions = Vec::new();

        // Check for unsafe code
        let unsafe_issues = self.check_unsafe_code(&modification.modified);
        if !unsafe_issues.is_empty() {
            issues.extend(unsafe_issues);
            suggestions.push("Consider safe alternatives to unsafe code".to_string());
        }

        // Check for panic patterns
        let panic_issues = self.check_panic_patterns(&modification.modified);
        if !panic_issues.is_empty() {
            issues.extend(panic_issues);
            suggestions.push("Use Result<T, E> instead of panicking".to_string());
        }

        // Basic syntax check (simple heuristic)
        let open_braces = modification.modified.matches('{').count();
        let close_braces = modification.modified.matches('}').count();
        if open_braces != close_braces {
            issues.push(format!("Brace mismatch: {} open, {} close", open_braces, close_braces));
            suggestions.push("Check for missing or extra braces".to_string());
        }

        let risk_level = if issues.is_empty() {
            RiskLevel::Low
        } else if issues.len() <= 2 {
            RiskLevel::Medium
        } else {
            RiskLevel::High
        };

        GateResult {
            gate_name: self.name(),
            passed: issues.is_empty(),
            risk_level,
            issues,
            suggestions,
        }
    }
}

/// Semantic validation gate
pub struct SemanticValidator {
    breaking_patterns: Vec<Regex>,
}

impl SemanticValidator {
    pub fn new() -> Self {
        Self {
            breaking_patterns: vec![
                Regex::new(r"pub\s+fn\s+\w+.*?->").unwrap(),
                Regex::new(r"pub\s+struct\s+\w+").unwrap(),
                Regex::new(r"pub\s+enum\s+\w+").unwrap(),
                Regex::new(r"pub\s+trait\s+\w+").unwrap(),
            ],
        }
    }

    fn check_api_changes(&self, original: &str, modified: &str) -> Vec<String> {
        let mut issues = Vec::new();

        // Extract public items from original
        let mut original_items = HashSet::new();
        for pattern in &self.breaking_patterns {
            for cap in pattern.find_iter(original) {
                original_items.insert(cap.as_str().to_string());
            }
        }

        // Check if any public items were removed
        for item in original_items {
            if !modified.contains(&item) {
                issues.push(format!("Public API removed or changed: {}", item));
            }
        }

        issues
    }
}

#[async_trait]
impl ValidationGate for SemanticValidator {
    fn name(&self) -> String {
        "SemanticValidation".to_string()
    }

    async fn validate(&self, modification: &CodeModification) -> GateResult {
        debug!("Running semantic validation on {}", modification.file_path.display());

        let mut issues = Vec::new();
        let mut suggestions = Vec::new();

        // Check for breaking API changes
        if !modification.original.is_empty() {
            let api_issues = self.check_api_changes(&modification.original, &modification.modified);
            if !api_issues.is_empty() {
                issues.extend(api_issues);
                suggestions
                    .push("Maintain backwards compatibility or version appropriately".to_string());
            }
        }

        // Check module structure preservation
        let original_mods: Vec<_> = Regex::new(r"mod\s+\w+;")
            .unwrap()
            .find_iter(&modification.original)
            .map(|m| m.as_str())
            .collect();

        for module in original_mods {
            if !modification.modified.contains(module) {
                issues.push(format!("Module declaration removed: {}", module));
                suggestions.push("Preserve module structure".to_string());
            }
        }

        let risk_level = if issues.is_empty() {
            RiskLevel::Low
        } else if issues.iter().any(|i| i.contains("Public API")) {
            RiskLevel::High
        } else {
            RiskLevel::Medium
        };

        GateResult {
            gate_name: self.name(),
            passed: issues.is_empty(),
            risk_level,
            issues,
            suggestions,
        }
    }
}

/// Security validation gate
pub struct SecurityValidator {
    dangerous_patterns: Vec<Regex>,
    credential_patterns: Vec<Regex>,
}

impl SecurityValidator {
    pub fn new() -> Self {
        Self {
            dangerous_patterns: vec![
                Regex::new(r"std::process::Command").unwrap(),
                Regex::new(r"std::env::var").unwrap(),
                Regex::new(r"std::fs::remove").unwrap(),
                Regex::new(r"eval\(").unwrap(),
                Regex::new(r"exec\(").unwrap(),
            ],
            credential_patterns: vec![
                Regex::new(r#"(?i)(api[_-]?key|apikey)\s*=\s*["'][\w]+["']"#).unwrap(),
                Regex::new(r#"(?i)(secret|password|passwd|pwd)\s*=\s*["'][\w]+["']"#).unwrap(),
                Regex::new(r#"(?i)token\s*=\s*["'][\w]+["']"#).unwrap(),
                Regex::new(r#"(?i)bearer\s+[\w]+"#).unwrap(),
            ],
        }
    }
}

#[async_trait]
impl ValidationGate for SecurityValidator {
    fn name(&self) -> String {
        "SecurityGate".to_string()
    }

    async fn validate(&self, modification: &CodeModification) -> GateResult {
        debug!("Running security validation on {}", modification.file_path.display());

        let mut issues = Vec::new();
        let mut suggestions = Vec::new();

        // Check for dangerous patterns
        for pattern in &self.dangerous_patterns {
            if pattern.is_match(&modification.modified) {
                warn!("Dangerous pattern detected: {}", pattern.as_str());
                issues.push(format!("Potentially dangerous pattern: {}", pattern.as_str()));
                suggestions.push("Review security implications of this code".to_string());
            }
        }

        // Check for hardcoded credentials
        for pattern in &self.credential_patterns {
            if pattern.is_match(&modification.modified) {
                issues.push("Possible hardcoded credential detected".to_string());
                suggestions
                    .push("Use environment variables or secure credential storage".to_string());
            }
        }

        // Check for network operations in core modules
        if modification.file_path.starts_with("src/core")
            || modification.file_path.starts_with("src/safety")
        {
            if modification.modified.contains("reqwest")
                || modification.modified.contains("hyper")
                || modification.modified.contains("TcpStream")
            {
                issues.push("Network operation in core module".to_string());
                suggestions.push("Core modules should not perform network operations".to_string());
            }
        }

        let risk_level = if issues.is_empty() {
            RiskLevel::Low
        } else if issues.iter().any(|i| i.contains("credential")) {
            RiskLevel::Critical
        } else {
            RiskLevel::High
        };

        GateResult {
            gate_name: self.name(),
            passed: issues.is_empty(),
            risk_level,
            issues,
            suggestions,
        }
    }
}

/// Performance validation gate
pub struct PerformanceValidator {
    complexity_patterns: Vec<Regex>,
}

impl PerformanceValidator {
    pub fn new() -> Self {
        Self {
            complexity_patterns: vec![
                Regex::new(r#"for\s+.*?\s+in\s+.*?\s*\{[^}]*for\s+.*?\s+in"#).unwrap(),
                Regex::new(r#"while\s+.*?\s*\{[^}]*while\s+"#).unwrap(),
                Regex::new(r#"\.clone\(\)"#).unwrap(),
            ],
        }
    }

    fn estimate_complexity(&self, content: &str) -> usize {
        let mut complexity = 0;

        // Count nested loops
        for pattern in &self.complexity_patterns {
            complexity += pattern.find_iter(content).count() * 2;
        }

        // Count excessive cloning
        let clone_count = content.matches(".clone()").count();
        if clone_count > 5 {
            complexity += clone_count / 5;
        }

        // Check for recursive patterns (simple heuristic)
        let lines: Vec<&str> = content.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if let Some(func_name) = extract_function_name(line) {
                // Check if function calls itself
                for j in i + 1..lines.len() {
                    if lines[j].contains(&func_name) && lines[j].contains('(') {
                        complexity += 3;
                        break;
                    }
                }
            }
        }

        complexity
    }
}

fn extract_function_name(line: &str) -> Option<String> {
    if line.trim_start().starts_with("fn ") {
        line.split_whitespace().nth(1).and_then(|s| s.split('(').next()).map(|s| s.to_string())
    } else {
        None
    }
}

#[async_trait]
impl ValidationGate for PerformanceValidator {
    fn name(&self) -> String {
        "PerformanceGate".to_string()
    }

    async fn validate(&self, modification: &CodeModification) -> GateResult {
        debug!("Running performance validation on {}", modification.file_path.display());

        let mut issues = Vec::new();
        let mut suggestions = Vec::new();

        let complexity = self.estimate_complexity(&modification.modified);

        if complexity > 10 {
            issues.push(format!("High complexity score: {}", complexity));
            suggestions.push("Consider breaking down complex functions".to_string());
        }

        // Check for blocking I/O in async context
        if modification.modified.contains("async fn") {
            if modification.modified.contains("std::fs::")
                && !modification.modified.contains("tokio::fs")
            {
                issues.push("Blocking I/O in async function".to_string());
                suggestions.push("Use tokio::fs for async file operations".to_string());
            }
        }

        // Check for excessive allocations
        let vec_new_count = modification.modified.matches("Vec::new()").count();
        let hashmap_new_count = modification.modified.matches("HashMap::new()").count();
        if vec_new_count + hashmap_new_count > 10 {
            issues.push("Excessive allocations detected".to_string());
            suggestions.push("Consider reusing collections or using with_capacity".to_string());
        }

        let risk_level = if issues.is_empty() {
            RiskLevel::Low
        } else if complexity > 20 {
            RiskLevel::High
        } else {
            RiskLevel::Medium
        };

        GateResult {
            gate_name: self.name(),
            passed: issues.is_empty() || complexity < 15,
            risk_level,
            issues,
            suggestions,
        }
    }
}

/// Reversibility validation gate
pub struct ReversibilityValidator;

impl ReversibilityValidator {
    pub fn new() -> Self {
        Self
    }

    fn check_destructive_operations(&self, content: &str) -> Vec<String> {
        let mut issues = Vec::new();

        let destructive_patterns =
            ["std::fs::remove", "drop(", "truncate(", ".clear()", "std::mem::forget"];

        for pattern in &destructive_patterns {
            if content.contains(pattern) {
                issues.push(format!("Potentially destructive operation: {}", pattern));
            }
        }

        issues
    }
}

#[async_trait]
impl ValidationGate for ReversibilityValidator {
    fn name(&self) -> String {
        "ReversibilityCheck".to_string()
    }

    async fn validate(&self, modification: &CodeModification) -> GateResult {
        debug!("Running reversibility check on {}", modification.file_path.display());

        let mut issues = Vec::new();
        let mut suggestions = Vec::new();

        // Check for destructive operations
        let destructive = self.check_destructive_operations(&modification.modified);
        if !destructive.is_empty() {
            issues.extend(destructive);
            suggestions.push("Ensure data can be recovered after operation".to_string());
        }

        // Check if original content is preserved somewhere
        if modification.original.is_empty()
            && matches!(modification.modification_type, ModificationType::Delete)
        {
            issues.push("Deletion without backup".to_string());
            suggestions.push("Create backup before deletion".to_string());
        }

        // Check for state modifications without rollback
        if modification.modified.contains("self.")
            && modification.modified.contains(" = ")
            && !modification.modified.contains("backup")
            && !modification.modified.contains("previous")
        {
            issues.push("State modification without apparent backup".to_string());
            suggestions.push("Consider implementing rollback mechanism".to_string());
        }

        let risk_level = if issues.is_empty() {
            RiskLevel::Low
        } else if issues.iter().any(|i| i.contains("Deletion")) {
            RiskLevel::High
        } else {
            RiskLevel::Medium
        };

        GateResult {
            gate_name: self.name(),
            passed: issues.is_empty() || issues.len() == 1,
            risk_level,
            issues,
            suggestions,
        }
    }
}
