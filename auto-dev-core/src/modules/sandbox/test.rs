//! Integration tests for the sandbox module

#[cfg(test)]
mod integration_tests {
    use crate::modules::sandbox::{
        ModuleSandbox, Capability, CapabilitySet,
        capabilities::{FileSystemCapability, FileOperation, NetworkCapability, NetworkProtocol}
    };
    use std::path::PathBuf;
    
    #[tokio::test]
    async fn test_capability_enforcement() {
        // Create a capability set with limited permissions
        let mut capabilities = CapabilitySet::new();
        
        // Allow read access to /docs
        capabilities.add(Capability::FileSystem(FileSystemCapability {
            operation: FileOperation::Read,
            path: PathBuf::from("/docs"),
        }));
        
        // Allow HTTP access to localhost
        capabilities.add(Capability::Network(NetworkCapability {
            protocol: NetworkProtocol::Http,
            host: "localhost".to_string(),
            port: Some(8080),
        }));
        
        // Create sandbox
        let sandbox = ModuleSandbox::new("test_module".to_string(), capabilities).unwrap();
        
        // Test allowed capability
        let allowed_cap = Capability::FileSystem(FileSystemCapability {
            operation: FileOperation::Read,
            path: PathBuf::from("/docs/api.md"),
        });
        assert!(sandbox.check_capability(&allowed_cap).is_ok());
        
        // Test denied capability (write instead of read)
        let denied_cap = Capability::FileSystem(FileSystemCapability {
            operation: FileOperation::Write,
            path: PathBuf::from("/docs/api.md"),
        });
        assert!(sandbox.check_capability(&denied_cap).is_err());
        
        // Test denied capability (different path)
        let denied_path = Capability::FileSystem(FileSystemCapability {
            operation: FileOperation::Read,
            path: PathBuf::from("/etc/passwd"),
        });
        assert!(sandbox.check_capability(&denied_path).is_err());
    }
    
    #[tokio::test]
    async fn test_capability_from_spec() {
        let specs = vec![
            "filesystem:read:/docs",
            "filesystem:write:/tmp",
            "network:http:localhost",
            "memory:limit:100MB",
            "cpu:limit:50%",
            "module:call:parser",
        ];
        
        let mut capabilities = CapabilitySet::new();
        
        for spec in specs {
            let cap = CapabilitySet::from_spec(spec).unwrap();
            capabilities.add(cap);
        }
        
        // Create sandbox with these capabilities
        let sandbox = ModuleSandbox::new("spec_test_module".to_string(), capabilities).unwrap();
        
        // Test that sandbox was created successfully
        assert_eq!(sandbox.module_id(), "spec_test_module");
    }
    
    #[tokio::test]
    async fn test_resource_monitoring() {
        use crate::modules::sandbox::resource_limits::{ResourceMonitor, ResourceLimits};
        
        let limits = ResourceLimits {
            max_memory_bytes: Some(50 * 1024 * 1024), // 50MB
            max_cpu_time_ms: Some(1000), // 1 second
            max_threads: Some(5),
            max_file_handles: Some(10),
            max_network_bandwidth_bps: None,
        };
        
        let monitor = ResourceMonitor::with_limits(limits);
        monitor.start_monitoring();
        
        // Test within limits
        assert!(monitor.update_memory(10 * 1024 * 1024).await.is_ok());
        assert!(monitor.update_threads(3).await.is_ok());
        
        // Test exceeding limits
        assert!(monitor.update_memory(100 * 1024 * 1024).await.is_err());
        assert!(monitor.update_threads(10).await.is_err());
        
        monitor.stop_monitoring();
    }
    
    #[tokio::test]
    async fn test_violation_handling() {
        use crate::modules::sandbox::{
            ViolationHandler, ViolationType, AuditLogger,
            violations::ViolationResponse
        };
        use std::sync::Arc;
        
        let audit_logger = Arc::new(AuditLogger::new());
        let handler = ViolationHandler::new(audit_logger);
        
        // Test capability violation
        let cap_violation = ViolationType::CapabilityViolation(
            Capability::FileSystem(FileSystemCapability {
                operation: FileOperation::Write,
                path: PathBuf::from("/system"),
            })
        );
        
        let response = handler.handle_violation("test_module", cap_violation).unwrap();
        assert!(matches!(response, ViolationResponse::Deny));
        
        // Test sandbox escape attempt
        let escape_violation = ViolationType::SandboxEscape {
            attempt_description: "Attempted to access host memory".to_string(),
        };
        
        let response = handler.handle_violation("malicious_module", escape_violation).unwrap();
        assert!(matches!(response, ViolationResponse::Quarantine));
    }
    
    #[tokio::test]
    async fn test_audit_logging() {
        use crate::modules::sandbox::{AuditLogger, SecurityEvent, 
            audit::{SecurityEventType, Severity}};
        use chrono::Utc;
        
        let logger = AuditLogger::new();
        
        // Log various security events
        let events = vec![
            SecurityEvent {
                timestamp: Utc::now(),
                module_id: "test_module".to_string(),
                event_type: SecurityEventType::ModuleStarted,
                severity: Severity::Info,
                details: "Module started successfully".to_string(),
            },
            SecurityEvent {
                timestamp: Utc::now(),
                module_id: "test_module".to_string(),
                event_type: SecurityEventType::FileAccess {
                    path: PathBuf::from("/docs/api.md"),
                    operation: "read".to_string(),
                },
                severity: Severity::Debug,
                details: "File access granted".to_string(),
            },
            SecurityEvent {
                timestamp: Utc::now(),
                module_id: "test_module".to_string(),
                event_type: SecurityEventType::ViolationDetected {
                    violation_type: "capability".to_string(),
                },
                severity: Severity::Warning,
                details: "Capability violation detected".to_string(),
            },
        ];
        
        for event in events {
            logger.log_event(event).await.unwrap();
        }
        
        // Get recent events
        let recent = logger.get_recent_events(10).await;
        assert!(recent.len() >= 2); // Debug level might be filtered out
        
        // Get module events
        let module_events = logger.get_module_events("test_module").await;
        assert!(!module_events.is_empty());
        
        // Generate report
        let report = logger.generate_report().await;
        assert!(report.total_events > 0);
    }
}