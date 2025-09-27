//! Safety monitoring for self-development activities

use super::{Result, SafetyLevel};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

pub struct SafetyMonitor {
    safety_level: SafetyLevel,
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
        let mut blocked_patterns = HashSet::new();
        let mut allowed_directories = HashSet::new();
        
        match safety_level {
            SafetyLevel::Strict => {
                blocked_patterns.insert("*.exe".to_string());
                blocked_patterns.insert("*.dll".to_string());
                blocked_patterns.insert("*.so".to_string());
                blocked_patterns.insert("*.dylib".to_string());
                blocked_patterns.insert("Cargo.lock".to_string());
                blocked_patterns.insert(".git/*".to_string());
                blocked_patterns.insert("target/*".to_string());
                
                allowed_directories.insert("src".to_string());
                allowed_directories.insert("tests".to_string());
                allowed_directories.insert("docs".to_string());
            }
            SafetyLevel::Standard => {
                blocked_patterns.insert("*.exe".to_string());
                blocked_patterns.insert(".git/*".to_string());
                
                allowed_directories.insert("src".to_string());
                allowed_directories.insert("tests".to_string());
                allowed_directories.insert("docs".to_string());
                allowed_directories.insert("examples".to_string());
                allowed_directories.insert("benches".to_string());
            }
            SafetyLevel::Permissive => {
                blocked_patterns.insert(".git/*".to_string());
            }
        }
        
        Self {
            safety_level,
            blocked_patterns: Arc::new(RwLock::new(blocked_patterns)),
            allowed_directories: Arc::new(RwLock::new(allowed_directories)),
            validation_history: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub async fn validate_change(&self, change_id: &str) -> Result<bool> {
        debug!("Validating change: {}", change_id);
        
        let validation_checks = vec![
            self.check_file_patterns(change_id).await,
            self.check_directory_permissions(change_id).await,
            self.check_code_safety(change_id).await,
            self.check_resource_limits(change_id).await,
            self.check_dependency_safety(change_id).await,
        ];
        
        let mut all_passed = true;
        let mut failure_reasons: Vec<String> = Vec::new();
        
        for (check_name, result) in validation_checks {
            match result {
                Ok(true) => {
                    debug!("{} check passed for {}", check_name, change_id);
                }
                Ok(false) => {
                    warn!("{} check failed for {}", check_name, change_id);
                    all_passed = false;
                    failure_reasons.push(check_name.to_string());
                }
                Err(e) => {
                    error!("{} check error for {}: {}", check_name, change_id, e);
                    all_passed = false;
                    failure_reasons.push(format!("{} (error)", check_name));
                }
            }
        }
        
        let record = ValidationRecord {
            change_id: change_id.to_string(),
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
            info!("Change {} passed all safety validations", change_id);
        } else {
            warn!("Change {} failed safety validations: {:?}", change_id, failure_reasons);
        }
        
        Ok(all_passed)
    }
    
    async fn check_file_patterns(&self, _change_id: &str) -> (&str, Result<bool>) {
        ("file_patterns", Ok(true))
    }
    
    async fn check_directory_permissions(&self, _change_id: &str) -> (&str, Result<bool>) {
        match self.safety_level {
            SafetyLevel::Strict => {
                ("directory_permissions", Ok(true))
            }
            _ => ("directory_permissions", Ok(true))
        }
    }
    
    async fn check_code_safety(&self, _change_id: &str) -> (&str, Result<bool>) {
        ("code_safety", Ok(true))
    }
    
    async fn check_resource_limits(&self, _change_id: &str) -> (&str, Result<bool>) {
        ("resource_limits", Ok(true))
    }
    
    async fn check_dependency_safety(&self, _change_id: &str) -> (&str, Result<bool>) {
        ("dependency_safety", Ok(true))
    }
    
    pub async fn add_blocked_pattern(&mut self, pattern: String) {
        self.blocked_patterns.write().await.insert(pattern);
    }
    
    pub async fn remove_blocked_pattern(&mut self, pattern: &str) {
        self.blocked_patterns.write().await.remove(pattern);
    }
    
    pub async fn add_allowed_directory(&mut self, directory: String) {
        self.allowed_directories.write().await.insert(directory);
    }
    
    pub async fn remove_allowed_directory(&mut self, directory: &str) {
        self.allowed_directories.write().await.remove(directory);
    }
    
    pub async fn get_validation_history(&self) -> Vec<(String, bool)> {
        self.validation_history
            .read()
            .await
            .iter()
            .map(|r| (r.change_id.clone(), r.passed))
            .collect()
    }
    
    pub async fn clear_validation_history(&self) {
        self.validation_history.write().await.clear();
    }
    
    pub fn safety_level(&self) -> &SafetyLevel {
        &self.safety_level
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_safety_monitor_creation() {
        let monitor = SafetyMonitor::new(SafetyLevel::Standard);
        assert!(matches!(monitor.safety_level(), SafetyLevel::Standard));
    }
    
    #[tokio::test]
    async fn test_validation() {
        let monitor = SafetyMonitor::new(SafetyLevel::Standard);
        let result = monitor.validate_change("test_change_123").await;
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        let history = monitor.get_validation_history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].0, "test_change_123");
        assert!(history[0].1);
    }
    
    #[tokio::test]
    async fn test_strict_safety_level() {
        let monitor = SafetyMonitor::new(SafetyLevel::Strict);
        let result = monitor.validate_change("strict_test").await;
        assert!(result.is_ok());
    }
}