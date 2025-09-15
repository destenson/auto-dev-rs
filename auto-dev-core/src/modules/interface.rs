// Module Interface Definition
//
// Defines the trait that all modules must implement, along with
// versioning, capabilities, and state management interfaces.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Version information for a module
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
}

impl ModuleVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch, pre_release: None }
    }

    pub fn is_compatible_with(&self, other: &ModuleVersion) -> bool {
        // Major version must match for compatibility
        self.major == other.major
    }

    pub fn to_string(&self) -> String {
        match &self.pre_release {
            Some(pre) => format!("{}.{}.{}-{}", self.major, self.minor, self.patch, pre),
            None => format!("{}.{}.{}", self.major, self.minor, self.patch),
        }
    }
}

/// Capabilities that a module can provide
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleCapability {
    Parser { language: String },
    Formatter { language: String },
    SynthesisStrategy { name: String },
    Monitor { type_name: String },
    LLMProvider { model: String },
    TestGenerator { framework: String },
    Custom { name: String, description: String },
}

/// Module state for hot-reload support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleState {
    pub version: ModuleVersion,
    pub data: HashMap<String, Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ModuleState {
    pub fn new(version: ModuleVersion) -> Self {
        Self { version, data: HashMap::new(), timestamp: chrono::Utc::now() }
    }

    pub fn set(&mut self, key: String, value: Value) {
        self.data.insert(key, value);
        self.timestamp = chrono::Utc::now();
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }
}

/// Module metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetadata {
    pub name: String,
    pub version: ModuleVersion,
    pub author: String,
    pub description: String,
    pub capabilities: Vec<ModuleCapability>,
    pub dependencies: Vec<ModuleDependency>,
}

/// Module dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDependency {
    pub name: String,
    pub version_requirement: String,
    pub optional: bool,
}

/// Main trait that all modules must implement
#[async_trait]
pub trait ModuleInterface: Send + Sync {
    /// Get module metadata
    fn metadata(&self) -> ModuleMetadata;

    /// Initialize the module
    async fn initialize(&mut self) -> Result<()>;

    /// Execute the module's main functionality
    async fn execute(&self, input: Value) -> Result<Value>;

    /// Get the capabilities this module provides
    fn get_capabilities(&self) -> Vec<ModuleCapability>;

    /// Handle incoming messages
    async fn handle_message(&mut self, message: Value) -> Result<Option<Value>>;

    /// Graceful shutdown
    async fn shutdown(&mut self) -> Result<()>;

    /// Get current module state for hot-reload
    fn get_state(&self) -> Result<ModuleState>;

    /// Restore module state after hot-reload
    fn restore_state(&mut self, state: ModuleState) -> Result<()>;

    /// Health check
    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }

    /// Get module-specific configuration
    fn get_config(&self) -> Result<Value> {
        Ok(Value::Object(serde_json::Map::new()))
    }

    /// Update module configuration at runtime
    fn update_config(&mut self, config: Value) -> Result<()> {
        Ok(())
    }
}

/// Trait for modules that support sandboxing
#[async_trait]
pub trait SandboxedModule: ModuleInterface {
    /// Get resource limits for this module
    fn resource_limits(&self) -> ResourceLimits;

    /// Check if an operation is allowed
    async fn check_permission(&self, operation: &str) -> Result<bool>;
}

/// Resource limits for sandboxed modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory_bytes: u64,
    pub max_cpu_time_ms: u64,
    pub max_file_handles: u32,
    pub allowed_paths: Vec<String>,
    pub network_access: bool,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 100 * 1024 * 1024, // 100MB
            max_cpu_time_ms: 5000,               // 5 seconds
            max_file_handles: 10,
            allowed_paths: vec![],
            network_access: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compatibility() {
        let v1 = ModuleVersion::new(1, 0, 0);
        let v2 = ModuleVersion::new(1, 1, 0);
        let v3 = ModuleVersion::new(2, 0, 0);

        assert!(v1.is_compatible_with(&v2));
        assert!(!v1.is_compatible_with(&v3));
    }

    #[test]
    fn test_module_state() {
        let mut state = ModuleState::new(ModuleVersion::new(1, 0, 0));
        state.set("key".to_string(), Value::String("value".to_string()));

        assert_eq!(state.get("key"), Some(&Value::String("value".to_string())));
    }
}
