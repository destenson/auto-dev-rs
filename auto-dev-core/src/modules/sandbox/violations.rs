//! Violation detection and handling for sandboxed modules

use crate::modules::sandbox::capabilities::Capability;
use crate::modules::sandbox::resource_limits::ResourceUsage;
use crate::modules::sandbox::audit::{AuditLogger, Severity};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, warn};

/// Types of violations that can occur in the sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationType {
    CapabilityViolation(Capability),
    ResourceViolation(ResourceUsage),
    RepeatedViolation { count: usize, violation: Box<ViolationType> },
    RateLimitViolation { requests_per_second: f64 },
    SuspiciousActivity { description: String },
    SandboxEscape { attempt_description: String },
}

/// Response actions for violations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationResponse {
    Warn,
    Deny,
    Throttle { duration: Duration },
    Terminate,
    Quarantine,
}

/// Violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationRecord {
    pub module_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub violation_type: ViolationType,
    pub response: ViolationResponse,
    pub details: String,
}

/// Configuration for violation handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationConfig {
    pub max_warnings_before_termination: usize,
    pub auto_quarantine_on_escape_attempt: bool,
    pub throttle_duration_ms: u64,
    pub track_violation_patterns: bool,
}

impl Default for ViolationConfig {
    fn default() -> Self {
        Self {
            max_warnings_before_termination: 3,
            auto_quarantine_on_escape_attempt: true,
            throttle_duration_ms: 5000,
            track_violation_patterns: true,
        }
    }
}

/// Handles security violations for sandboxed modules
pub struct ViolationHandler {
    config: ViolationConfig,
    violations: Arc<RwLock<Vec<ViolationRecord>>>,
    warning_counts: Arc<RwLock<HashMap<String, usize>>>,
    quarantined_modules: Arc<RwLock<Vec<String>>>,
    audit_logger: Arc<AuditLogger>,
}

impl ViolationHandler {
    /// Create a new violation handler
    pub fn new(audit_logger: Arc<AuditLogger>) -> Self {
        Self::with_config(ViolationConfig::default(), audit_logger)
    }

    /// Create a new violation handler with specific configuration
    pub fn with_config(config: ViolationConfig, audit_logger: Arc<AuditLogger>) -> Self {
        Self {
            config,
            violations: Arc::new(RwLock::new(Vec::new())),
            warning_counts: Arc::new(RwLock::new(HashMap::new())),
            quarantined_modules: Arc::new(RwLock::new(Vec::new())),
            audit_logger,
        }
    }

    /// Handle a violation
    pub fn handle_violation(
        &self,
        module_id: &str,
        violation_type: ViolationType,
    ) -> Result<ViolationResponse> {
        let response = self.determine_response(module_id, &violation_type);
        
        // Log the violation
        let severity = match response {
            ViolationResponse::Warn => Severity::Warning,
            ViolationResponse::Deny => Severity::Error,
            ViolationResponse::Throttle { .. } => Severity::Warning,
            ViolationResponse::Terminate => Severity::Critical,
            ViolationResponse::Quarantine => Severity::Critical,
        };
        
        self.audit_logger.log_violation(
            module_id,
            &format!("{:?}", violation_type),
            severity,
        );
        
        // Record the violation
        let record = ViolationRecord {
            module_id: module_id.to_string(),
            timestamp: chrono::Utc::now(),
            violation_type: violation_type.clone(),
            response: response.clone(),
            details: self.format_violation_details(&violation_type),
        };
        
        let handler = self.clone();
        let module_id = module_id.to_string();
        let response_clone = response.clone();
        
        tokio::spawn(async move {
            handler.record_violation(record).await;
            handler.apply_response(&module_id, &response_clone).await;
        });
        
        Ok(response)
    }

    /// Determine appropriate response based on violation type and history
    fn determine_response(&self, module_id: &str, violation_type: &ViolationType) -> ViolationResponse {
        match violation_type {
            ViolationType::SandboxEscape { .. } => {
                if self.config.auto_quarantine_on_escape_attempt {
                    ViolationResponse::Quarantine
                } else {
                    ViolationResponse::Terminate
                }
            }
            ViolationType::RepeatedViolation { count, .. } => {
                if *count >= self.config.max_warnings_before_termination {
                    ViolationResponse::Terminate
                } else {
                    ViolationResponse::Throttle {
                        duration: Duration::from_millis(self.config.throttle_duration_ms),
                    }
                }
            }
            ViolationType::ResourceViolation(_) => {
                ViolationResponse::Throttle {
                    duration: Duration::from_millis(self.config.throttle_duration_ms),
                }
            }
            ViolationType::CapabilityViolation(_) => {
                ViolationResponse::Deny
            }
            ViolationType::RateLimitViolation { .. } => {
                ViolationResponse::Throttle {
                    duration: Duration::from_millis(self.config.throttle_duration_ms * 2),
                }
            }
            ViolationType::SuspiciousActivity { .. } => {
                ViolationResponse::Warn
            }
        }
    }

    /// Record a violation
    async fn record_violation(&self, record: ViolationRecord) {
        let mut violations = self.violations.write().await;
        violations.push(record.clone());
        
        // Update warning counts
        if matches!(record.response, ViolationResponse::Warn) {
            let mut counts = self.warning_counts.write().await;
            *counts.entry(record.module_id.clone()).or_insert(0) += 1;
        }
        
        // Detect patterns if configured
        if self.config.track_violation_patterns {
            self.detect_patterns(&record.module_id, &violations).await;
        }
    }

    /// Apply the response action
    async fn apply_response(&self, module_id: &str, response: &ViolationResponse) {
        match response {
            ViolationResponse::Warn => {
                warn!("Warning issued to module: {}", module_id);
            }
            ViolationResponse::Deny => {
                warn!("Request denied for module: {}", module_id);
            }
            ViolationResponse::Throttle { duration } => {
                warn!("Throttling module {} for {:?}", module_id, duration);
                // In a real implementation, would actually throttle the module
            }
            ViolationResponse::Terminate => {
                error!("Terminating module due to violations: {}", module_id);
                // In a real implementation, would terminate the module
            }
            ViolationResponse::Quarantine => {
                error!("Quarantining module: {}", module_id);
                let mut quarantined = self.quarantined_modules.write().await;
                quarantined.push(module_id.to_string());
            }
        }
    }

    /// Detect violation patterns
    async fn detect_patterns(&self, module_id: &str, violations: &[ViolationRecord]) {
        let module_violations: Vec<_> = violations.iter()
            .filter(|v| v.module_id == module_id)
            .collect();
        
        if module_violations.len() < 3 {
            return;
        }
        
        // Check for repeated violations in short time
        let recent_violations: Vec<_> = module_violations.iter()
            .filter(|v| {
                let age = chrono::Utc::now() - v.timestamp;
                age.num_seconds() < 60
            })
            .collect();
        
        if recent_violations.len() >= 3 {
            warn!("Pattern detected: Module {} has {} violations in the last minute",
                module_id, recent_violations.len());
            
            // Escalate response
            let violation = ViolationType::RepeatedViolation {
                count: recent_violations.len(),
                violation: Box::new(recent_violations.last().unwrap().violation_type.clone()),
            };
            
            let _ = self.handle_violation(module_id, violation);
        }
    }

    /// Format violation details for logging
    fn format_violation_details(&self, violation_type: &ViolationType) -> String {
        match violation_type {
            ViolationType::CapabilityViolation(cap) => {
                format!("Attempted to use capability without permission: {:?}", cap)
            }
            ViolationType::ResourceViolation(usage) => {
                format!("Exceeded resource limits: {}", usage.summary())
            }
            ViolationType::RepeatedViolation { count, violation } => {
                format!("Repeated violation ({} times): {:?}", count, violation)
            }
            ViolationType::RateLimitViolation { requests_per_second } => {
                format!("Rate limit exceeded: {} requests/second", requests_per_second)
            }
            ViolationType::SuspiciousActivity { description } => {
                format!("Suspicious activity detected: {}", description)
            }
            ViolationType::SandboxEscape { attempt_description } => {
                format!("Sandbox escape attempt: {}", attempt_description)
            }
        }
    }

    /// Check if a module is quarantined
    pub async fn is_quarantined(&self, module_id: &str) -> bool {
        let quarantined = self.quarantined_modules.read().await;
        quarantined.contains(&module_id.to_string())
    }

    /// Get violation history for a module
    pub async fn get_module_violations(&self, module_id: &str) -> Vec<ViolationRecord> {
        let violations = self.violations.read().await;
        violations.iter()
            .filter(|v| v.module_id == module_id)
            .cloned()
            .collect()
    }

    /// Clear violation history
    pub async fn clear_violations(&self, module_id: Option<&str>) {
        let mut violations = self.violations.write().await;
        
        if let Some(id) = module_id {
            violations.retain(|v| v.module_id != id);
            
            let mut counts = self.warning_counts.write().await;
            counts.remove(id);
        } else {
            violations.clear();
            let mut counts = self.warning_counts.write().await;
            counts.clear();
        }
    }

    /// Release a module from quarantine
    pub async fn release_from_quarantine(&self, module_id: &str) -> Result<()> {
        let mut quarantined = self.quarantined_modules.write().await;
        quarantined.retain(|id| id != module_id);
        Ok(())
    }
}

impl Clone for ViolationHandler {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            violations: self.violations.clone(),
            warning_counts: self.warning_counts.clone(),
            quarantined_modules: self.quarantined_modules.clone(),
            audit_logger: self.audit_logger.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::sandbox::capabilities::{Capability, FileSystemCapability, FileOperation};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_violation_handler() {
        let audit_logger = Arc::new(AuditLogger::new());
        let handler = ViolationHandler::new(audit_logger);
        
        let violation = ViolationType::CapabilityViolation(
            Capability::FileSystem(FileSystemCapability {
                operation: FileOperation::Write,
                path: PathBuf::from("/etc/passwd"),
            })
        );
        
        let response = handler.handle_violation("test_module", violation).unwrap();
        assert!(matches!(response, ViolationResponse::Deny));
    }

    #[tokio::test]
    async fn test_sandbox_escape_quarantine() {
        let config = ViolationConfig {
            auto_quarantine_on_escape_attempt: true,
            ..Default::default()
        };
        
        let audit_logger = Arc::new(AuditLogger::new());
        let handler = ViolationHandler::with_config(config, audit_logger);
        
        let violation = ViolationType::SandboxEscape {
            attempt_description: "Attempted to access host filesystem".to_string(),
        };
        
        let response = handler.handle_violation("malicious_module", violation).unwrap();
        assert!(matches!(response, ViolationResponse::Quarantine));
        
        // Give async task time to complete
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        assert!(handler.is_quarantined("malicious_module").await);
    }
}