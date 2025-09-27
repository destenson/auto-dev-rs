//! Module Sandboxing and Isolation
//!
//! Provides capability-based security for dynamically loaded modules

pub mod audit;
pub mod capabilities;
pub mod resource_limits;
pub mod violations;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub use audit::{AuditLogger, SecurityEvent};
pub use capabilities::{Capability, CapabilityManager, CapabilitySet};
pub use resource_limits::{ResourceMonitor, ResourceUsage};
pub use violations::{ViolationHandler, ViolationType};

/// Main sandbox environment for modules
pub struct ModuleSandbox {
    capability_manager: CapabilityManager,
    resource_monitor: ResourceMonitor,
    audit_logger: Arc<AuditLogger>,
    violation_handler: ViolationHandler,
    module_id: String,
}

impl ModuleSandbox {
    /// Create a new sandbox for a module
    pub fn new(module_id: String, capabilities: CapabilitySet) -> Result<Self> {
        let audit_logger = Arc::new(AuditLogger::new());

        Ok(Self {
            capability_manager: CapabilityManager::new(capabilities),
            resource_monitor: ResourceMonitor::new(),
            audit_logger: audit_logger.clone(),
            violation_handler: ViolationHandler::new(audit_logger.clone()),
            module_id,
        })
    }

    /// Check if a capability is allowed
    pub fn check_capability(&self, capability: &Capability) -> Result<()> {
        if !self.capability_manager.is_allowed(capability) {
            self.violation_handler.handle_violation(
                &self.module_id,
                ViolationType::CapabilityViolation(capability.clone()),
            )?;
            return Err(anyhow::anyhow!("Capability not allowed: {:?}", capability));
        }

        self.audit_logger.log_access(&self.module_id, capability);
        Ok(())
    }

    /// Check resource usage
    pub async fn check_resources(&self) -> Result<ResourceUsage> {
        let usage = self.resource_monitor.get_usage().await?;

        if usage.exceeds_limits() {
            self.violation_handler.handle_violation(
                &self.module_id,
                ViolationType::ResourceViolation(usage.clone()),
            )?;
            return Err(anyhow::anyhow!("Resource limits exceeded"));
        }

        Ok(usage)
    }

    /// Execute a function with sandboxing
    pub async fn execute_sandboxed<F, T>(&self, func: F) -> Result<T>
    where
        F: FnOnce() -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        // Start resource monitoring
        self.resource_monitor.start_monitoring();

        // Execute the function
        let result = tokio::task::spawn_blocking(func).await?;

        // Stop monitoring and check usage
        self.resource_monitor.stop_monitoring();
        self.check_resources().await?;

        result
    }

    /// Get the module ID
    pub fn module_id(&self) -> &str {
        &self.module_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sandbox_creation() {
        let capabilities = CapabilitySet::new();
        let sandbox = ModuleSandbox::new("test_module".to_string(), capabilities);
        assert!(sandbox.is_ok());
    }
}

#[cfg(test)]
mod test;
