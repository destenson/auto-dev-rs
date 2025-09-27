//! Security audit logging for sandboxed modules

use crate::modules::sandbox::capabilities::Capability;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Represents a security event that should be logged
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub timestamp: DateTime<Utc>,
    pub module_id: String,
    pub event_type: SecurityEventType,
    pub severity: Severity,
    pub details: String,
}

/// Types of security events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventType {
    CapabilityRequested(Capability),
    CapabilityGranted(Capability),
    CapabilityDenied(Capability),
    ResourceLimitExceeded { resource_type: String, limit: u64, actual: u64 },
    ViolationDetected { violation_type: String },
    SandboxCreated,
    SandboxDestroyed,
    ModuleStarted,
    ModuleStopped,
    FileAccess { path: PathBuf, operation: String },
    NetworkAccess { host: String, port: Option<u16> },
    SystemCallAttempt { syscall: String },
    AnomalousActivity { description: String },
}

/// Severity levels for security events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Configuration for the audit logger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    pub max_events_in_memory: usize,
    pub log_to_file: bool,
    pub log_file_path: Option<PathBuf>,
    pub min_severity: Severity,
    pub alert_on_critical: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            max_events_in_memory: 10000,
            log_to_file: false,
            log_file_path: None,
            min_severity: Severity::Info,
            alert_on_critical: true,
        }
    }
}

/// Audit logger for security events
pub struct AuditLogger {
    events: Arc<RwLock<VecDeque<SecurityEvent>>>,
    config: AuditConfig,
}

impl AuditLogger {
    /// Create a new audit logger with default configuration
    pub fn new() -> Self {
        Self::with_config(AuditConfig::default())
    }

    /// Create a new audit logger with specific configuration
    pub fn with_config(config: AuditConfig) -> Self {
        Self {
            events: Arc::new(RwLock::new(VecDeque::with_capacity(config.max_events_in_memory))),
            config,
        }
    }

    /// Log a security event
    pub async fn log_event(&self, event: SecurityEvent) -> Result<()> {
        // Check severity filter
        if event.severity < self.config.min_severity {
            return Ok(());
        }

        // Log to tracing system
        match event.severity {
            Severity::Debug => debug!("Security Event: {:?}", event),
            Severity::Info => info!("Security Event: {:?}", event),
            Severity::Warning => warn!("Security Event: {:?}", event),
            Severity::Error => tracing::error!("Security Event: {:?}", event),
            Severity::Critical => {
                tracing::error!("CRITICAL Security Event: {:?}", event);
                if self.config.alert_on_critical {
                    self.raise_alert(&event).await?;
                }
            }
        }

        // Store in memory
        let mut events = self.events.write().await;
        if events.len() >= self.config.max_events_in_memory {
            events.pop_front();
        }
        events.push_back(event.clone());

        // Log to file if configured
        if self.config.log_to_file {
            if let Some(ref path) = self.config.log_file_path {
                self.write_to_file(path, &event).await?;
            }
        }

        Ok(())
    }

    /// Log a capability access attempt
    pub fn log_access(&self, module_id: &str, capability: &Capability) {
        let event = SecurityEvent {
            timestamp: Utc::now(),
            module_id: module_id.to_string(),
            event_type: SecurityEventType::CapabilityRequested(capability.clone()),
            severity: Severity::Debug,
            details: format!("Module {} requested capability: {:?}", module_id, capability),
        };
        
        // Fire and forget - don't block on logging
        let logger = self.clone();
        tokio::spawn(async move {
            let _ = logger.log_event(event).await;
        });
    }

    /// Log a capability grant
    pub fn log_grant(&self, module_id: &str, capability: &Capability) {
        let event = SecurityEvent {
            timestamp: Utc::now(),
            module_id: module_id.to_string(),
            event_type: SecurityEventType::CapabilityGranted(capability.clone()),
            severity: Severity::Info,
            details: format!("Capability granted to {}: {:?}", module_id, capability),
        };
        
        let logger = self.clone();
        tokio::spawn(async move {
            let _ = logger.log_event(event).await;
        });
    }

    /// Log a capability denial
    pub fn log_denial(&self, module_id: &str, capability: &Capability) {
        let event = SecurityEvent {
            timestamp: Utc::now(),
            module_id: module_id.to_string(),
            event_type: SecurityEventType::CapabilityDenied(capability.clone()),
            severity: Severity::Warning,
            details: format!("Capability denied to {}: {:?}", module_id, capability),
        };
        
        let logger = self.clone();
        tokio::spawn(async move {
            let _ = logger.log_event(event).await;
        });
    }

    /// Log a violation
    pub fn log_violation(&self, module_id: &str, violation_type: &str, severity: Severity) {
        let event = SecurityEvent {
            timestamp: Utc::now(),
            module_id: module_id.to_string(),
            event_type: SecurityEventType::ViolationDetected {
                violation_type: violation_type.to_string(),
            },
            severity,
            details: format!("Violation detected in {}: {}", module_id, violation_type),
        };
        
        let logger = self.clone();
        tokio::spawn(async move {
            let _ = logger.log_event(event).await;
        });
    }

    /// Get recent events
    pub async fn get_recent_events(&self, count: usize) -> Vec<SecurityEvent> {
        let events = self.events.read().await;
        events.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Get events for a specific module
    pub async fn get_module_events(&self, module_id: &str) -> Vec<SecurityEvent> {
        let events = self.events.read().await;
        events.iter()
            .filter(|e| e.module_id == module_id)
            .cloned()
            .collect()
    }

    /// Get events by severity
    pub async fn get_events_by_severity(&self, min_severity: Severity) -> Vec<SecurityEvent> {
        let events = self.events.read().await;
        events.iter()
            .filter(|e| e.severity >= min_severity)
            .cloned()
            .collect()
    }

    /// Clear all events
    pub async fn clear_events(&self) {
        let mut events = self.events.write().await;
        events.clear();
    }

    /// Write event to file
    async fn write_to_file(&self, path: &PathBuf, event: &SecurityEvent) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        
        let json = serde_json::to_string(event)?;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
        
        file.write_all(format!("{}\n", json).as_bytes()).await?;
        file.flush().await?;
        
        Ok(())
    }

    /// Raise an alert for critical events
    async fn raise_alert(&self, event: &SecurityEvent) -> Result<()> {
        // In a real implementation, this could:
        // - Send notifications
        // - Trigger emergency shutdown
        // - Alert administrators
        // - Create incident tickets
        
        tracing::error!(
            "🚨 CRITICAL SECURITY ALERT 🚨\nModule: {}\nEvent: {:?}\nDetails: {}",
            event.module_id,
            event.event_type,
            event.details
        );
        
        Ok(())
    }

    /// Generate audit report
    pub async fn generate_report(&self) -> AuditReport {
        let events = self.events.read().await;
        
        let total_events = events.len();
        let critical_events = events.iter().filter(|e| e.severity == Severity::Critical).count();
        let error_events = events.iter().filter(|e| e.severity == Severity::Error).count();
        let warning_events = events.iter().filter(|e| e.severity == Severity::Warning).count();
        
        let mut module_stats = std::collections::HashMap::new();
        for event in events.iter() {
            *module_stats.entry(event.module_id.clone()).or_insert(0) += 1;
        }
        
        AuditReport {
            total_events,
            critical_events,
            error_events,
            warning_events,
            module_stats,
            time_range: if events.is_empty() {
                None
            } else {
                Some((
                    events.front().unwrap().timestamp,
                    events.back().unwrap().timestamp,
                ))
            },
        }
    }
}

impl Clone for AuditLogger {
    fn clone(&self) -> Self {
        Self {
            events: self.events.clone(),
            config: self.config.clone(),
        }
    }
}

/// Audit report summarizing security events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub total_events: usize,
    pub critical_events: usize,
    pub error_events: usize,
    pub warning_events: usize,
    pub module_stats: std::collections::HashMap<String, usize>,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_logger() {
        let logger = AuditLogger::new();
        
        let event = SecurityEvent {
            timestamp: Utc::now(),
            module_id: "test_module".to_string(),
            event_type: SecurityEventType::ModuleStarted,
            severity: Severity::Info,
            details: "Test module started".to_string(),
        };
        
        logger.log_event(event).await.unwrap();
        
        let recent = logger.get_recent_events(10).await;
        assert_eq!(recent.len(), 1);
    }

    #[tokio::test]
    async fn test_severity_filtering() {
        let config = AuditConfig {
            min_severity: Severity::Warning,
            ..Default::default()
        };
        
        let logger = AuditLogger::with_config(config);
        
        // Info event should be filtered out
        let info_event = SecurityEvent {
            timestamp: Utc::now(),
            module_id: "test".to_string(),
            event_type: SecurityEventType::ModuleStarted,
            severity: Severity::Info,
            details: "Info event".to_string(),
        };
        
        logger.log_event(info_event).await.unwrap();
        
        // Warning event should be logged
        let warning_event = SecurityEvent {
            timestamp: Utc::now(),
            module_id: "test".to_string(),
            event_type: SecurityEventType::ViolationDetected {
                violation_type: "test".to_string(),
            },
            severity: Severity::Warning,
            details: "Warning event".to_string(),
        };
        
        logger.log_event(warning_event).await.unwrap();
        
        let events = logger.get_recent_events(10).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].severity, Severity::Warning);
    }
}