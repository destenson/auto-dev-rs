#![allow(unused)]
//! Safety monitoring for self-development activities

use super::orchestrator::{PendingChange, RiskLevel};
use super::{Result, SafetyLevel, SelfDevError};
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

pub struct SafetyMonitor {
    safety_level: Arc<RwLock<SafetyLevel>>,
    blocked_patterns: Arc<RwLock<HashSet<String>>>,
    allowed_directories: Arc<RwLock<HashSet<String>>>,
    validation_history: Arc<RwLock<Vec<ValidationRecord>>>,
}

#[derive(Debug, Clone)]
struct ValidationRecord {
    change_id: String,
    timestamp: std::time::SystemTime,
    passed: bool,
    reason: Option<String>,
}

impl SafetyMonitor {
    pub fn new(safety_level: SafetyLevel) -> Self {
        let (blocked, allowed) = Self::default_rules(&safety_level);

        Self {
            safety_level: Arc::new(RwLock::new(safety_level)),
            blocked_patterns: Arc::new(RwLock::new(blocked)),
            allowed_directories: Arc::new(RwLock::new(allowed)),
            validation_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn set_safety_level(&self, safety_level: SafetyLevel) {
        let (blocked, allowed) = Self::default_rules(&safety_level);
        *self.safety_level.write().await = safety_level;
        *self.blocked_patterns.write().await = blocked;
        *self.allowed_directories.write().await = allowed;
    }

    pub async fn validate_change(&self, change: &PendingChange) -> Result<bool> {
        debug!("Validating change: {}", change.id);

        let validation_checks = vec![
            ("file_patterns", self.check_file_patterns(change).await),
            ("directory_permissions", self.check_directory_permissions(change).await),
            ("risk_profile", self.check_risk_profile(change).await),
            ("resource_limits", self.check_resource_limits(change).await),
            ("dependency_safety", self.check_dependency_safety(change).await),
        ];

        let mut all_passed = true;
        let mut failure_reasons: Vec<String> = Vec::new();

        for (name, result) in validation_checks {
            match result {
                Ok(true) => debug!("{} check passed for {}", name, change.id),
                Ok(false) => {
                    warn!("{} check failed for {}", name, change.id);
                    all_passed = false;
                    failure_reasons.push(name.to_string());
                }
                Err(e) => {
                    error!("{} check error for {}: {}", name, change.id, e);
                    all_passed = false;
                    failure_reasons.push(format!("{} (error)", name));
                }
            }
        }

        let record = ValidationRecord {
            change_id: change.id.clone(),
            timestamp: std::time::SystemTime::now(),
            passed: all_passed,
            reason: if failure_reasons.is_empty() {
                None
            } else {
                Some(failure_reasons.join(", "))
            },
        };

        self.validation_history.write().await.push(record);

        if all_passed {
            info!("Change {} passed all safety validations", change.id);
        } else {
            warn!("Change {} failed safety validations", change.id);
        }

        Ok(all_passed)
    }

    async fn check_file_patterns(&self, change: &PendingChange) -> Result<bool> {
        let blocked = self.blocked_patterns.read().await.clone();
        if blocked.is_empty() {
            return Ok(true);
        }

        let mut builder = GlobSetBuilder::new();
        for pattern in blocked {
            if let Ok(glob) = Glob::new(&pattern) {
                builder.add(glob);
            }
        }
        let glob_set = builder.build().map_err(|e| {
            SelfDevError::SafetyViolation(format!("Invalid blocked pattern: {}", e))
        })?;

        for path in &change.target_files {
            if glob_set.is_match(path) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn check_directory_permissions(&self, change: &PendingChange) -> Result<bool> {
        let allowed = self.allowed_directories.read().await.clone();
        if allowed.is_empty() {
            return Ok(true);
        }

        for path in &change.target_files {
            if let Some(component) = path.components().next() {
                let component_str = component.as_os_str().to_string_lossy().to_string();
                if !allowed.contains(&component_str) {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    async fn check_risk_profile(&self, change: &PendingChange) -> Result<bool> {
        let safety_level = self.safety_level.read().await.clone();
        match (safety_level, &change.risk_level) {
            (SafetyLevel::Strict, RiskLevel::Critical) => Ok(false),
            (SafetyLevel::Strict, RiskLevel::High) => {
                Ok(!change.target_files.iter().any(|path| path.starts_with("docs")))
            }
            (SafetyLevel::Permissive, _) => Ok(true),
            _ => Ok(true),
        }
    }

    async fn check_resource_limits(&self, change: &PendingChange) -> Result<bool> {
        let limit = match self.safety_level.read().await.clone() {
            SafetyLevel::Strict => 4,
            SafetyLevel::Standard => 8,
            SafetyLevel::Permissive => 12,
        };

        Ok(change.target_files.len() <= limit)
    }

    async fn check_dependency_safety(&self, change: &PendingChange) -> Result<bool> {
        let risky_paths = ["Cargo.lock", ".git", "target", "node_modules"];
        for path in &change.target_files {
            if let Some(path_str) = path.to_str() {
                if risky_paths.iter().any(|blocked| path_str.contains(blocked)) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    pub async fn add_blocked_pattern(&self, pattern: String) {
        self.blocked_patterns.write().await.insert(pattern);
    }

    pub async fn remove_blocked_pattern(&self, pattern: &str) {
        self.blocked_patterns.write().await.remove(pattern);
    }

    pub async fn add_allowed_directory(&self, directory: String) {
        self.allowed_directories.write().await.insert(directory);
    }

    pub async fn remove_allowed_directory(&self, directory: &str) {
        self.allowed_directories.write().await.remove(directory);
    }

    pub async fn get_validation_history(&self) -> Vec<(String, bool, Option<String>)> {
        self.validation_history
            .read()
            .await
            .iter()
            .map(|record| (record.change_id.clone(), record.passed, record.reason.clone()))
            .collect()
    }

    pub async fn clear_validation_history(&self) {
        self.validation_history.write().await.clear();
    }

    pub async fn safety_level(&self) -> SafetyLevel {
        self.safety_level.read().await.clone()
    }

    fn default_rules(level: &SafetyLevel) -> (HashSet<String>, HashSet<String>) {
        let mut blocked = HashSet::new();
        let mut allowed = HashSet::new();

        match level {
            SafetyLevel::Strict => {
                blocked.extend([
                    "**/*.exe".to_string(),
                    "**/*.dll".to_string(),
                    "**/*.so".to_string(),
                    "**/*.dylib".to_string(),
                    "**/Cargo.lock".to_string(),
                    ".git/**".to_string(),
                    "target/**".to_string(),
                ]);

                allowed.extend([
                    "src".to_string(),
                    "auto-dev-core/src".to_string(),
                    "auto-dev/src".to_string(),
                    "docs".to_string(),
                    "tests".to_string(),
                ]);
            }
            SafetyLevel::Standard => {
                blocked.extend(["**/*.exe".to_string(), ".git/**".to_string()]);
                allowed.extend([
                    "src".to_string(),
                    "docs".to_string(),
                    "tests".to_string(),
                    "examples".to_string(),
                    "benches".to_string(),
                ]);
            }
            SafetyLevel::Permissive => {
                blocked.insert(".git/**".to_string());
            }
        }

        (blocked, allowed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_dev::orchestrator::{
        ChangeMetrics, ChangeStatus, ChangeType, PlanDigest, PlanStep,
    };
    use std::path::PathBuf;

    fn sample_change(target: &str, risk: RiskLevel) -> PendingChange {
        PendingChange {
            id: "test_change".to_string(),
            description: "Test change".to_string(),
            summary: None,
            file_path: "PRPs/999-test.md".to_string(),
            change_type: ChangeType::Modify,
            risk_level: risk,
            status: ChangeStatus::ReadyForReview,
            plan: Some(PlanDigest {
                steps: vec![PlanStep {
                    id: "1".into(),
                    description: "Step".into(),
                    depends_on: vec![],
                    tests: vec![],
                }],
                estimated_duration: std::time::Duration::from_secs(60),
                critical_path: vec![],
            }),
            target_files: vec![PathBuf::from(target)],
            required_components: vec!["testing".to_string()],
            last_updated: std::time::SystemTime::now(),
            metrics: ChangeMetrics::default(),
        }
    }

    #[tokio::test]
    async fn test_safety_monitor_creation() {
        let monitor = SafetyMonitor::new(SafetyLevel::Standard);
        assert!(matches!(monitor.safety_level().await, SafetyLevel::Standard));
    }

    #[tokio::test]
    async fn test_validation_pass() {
        let monitor = SafetyMonitor::new(SafetyLevel::Standard);
        let change = sample_change("src/lib.rs", RiskLevel::Low);
        let result = monitor.validate_change(&change).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_blocked_pattern() {
        let monitor = SafetyMonitor::new(SafetyLevel::Strict);
        let change = sample_change("Cargo.lock", RiskLevel::Medium);
        let result = monitor.validate_change(&change).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_strict_risk_failure() {
        let monitor = SafetyMonitor::new(SafetyLevel::Strict);
        let change = sample_change("auto-dev-core/src/lib.rs", RiskLevel::Critical);
        let result = monitor.validate_change(&change).await.unwrap();
        assert!(!result);
    }
}
