// Module Runtime
//
// Manages module execution, state, and isolation

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, Mutex};
use tokio::time::timeout;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::modules::interface::{ModuleState, ModuleInterface};
use crate::modules::registry::ModuleRegistry;

/// Execution context passed to modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub input: Value,
    pub config: HashMap<String, Value>,
    pub timeout_ms: Option<u64>,
    pub trace_enabled: bool,
}

impl ExecutionContext {
    pub fn new(input: Value) -> Self {
        Self {
            input,
            config: HashMap::new(),
            timeout_ms: Some(5000), // Default 5 second timeout
            trace_enabled: false,
        }
    }

    pub fn with_config(mut self, key: String, value: Value) -> Self {
        self.config.insert(key, value);
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn with_tracing(mut self, enabled: bool) -> Self {
        self.trace_enabled = enabled;
        self
    }
}

/// Execution result with metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub output: Value,
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
    pub metrics: ExecutionMetrics,
}

/// Execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub memory_used: u64,
    pub cpu_time_ms: u64,
    pub messages_sent: u32,
    pub messages_received: u32,
}

impl Default for ExecutionMetrics {
    fn default() -> Self {
        Self {
            memory_used: 0,
            cpu_time_ms: 0,
            messages_sent: 0,
            messages_received: 0,
        }
    }
}

/// Module execution state
struct ModuleExecution {
    state: ModuleState,
    metrics: ExecutionMetrics,
    last_execution: Option<chrono::DateTime<chrono::Utc>>,
    is_running: bool,
}

/// Module runtime that manages execution
pub struct ModuleRuntime {
    registry: Arc<RwLock<ModuleRegistry>>,
    executions: Arc<RwLock<HashMap<String, ModuleExecution>>>,
    execution_lock: Arc<RwLock<HashMap<String, Arc<Mutex<()>>>>>,
}

impl ModuleRuntime {
    /// Create a new module runtime
    pub fn new() -> Self {
        Self {
            registry: Arc::new(RwLock::new(ModuleRegistry::new())),
            executions: Arc::new(RwLock::new(HashMap::new())),
            execution_lock: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set the registry for this runtime
    pub fn set_registry(&mut self, registry: Arc<RwLock<ModuleRegistry>>) {
        self.registry = registry;
    }

    /// Initialize a module
    pub async fn initialize_module(&self, module_id: &str) -> Result<()> {
        let mut registry = self.registry.write().await;
        
        let module = registry.get_mut(module_id)
            .ok_or_else(|| anyhow::anyhow!("Module not found: {}", module_id))?;

        // Initialize the module
        module.as_interface_mut().initialize().await
            .context("Failed to initialize module")?;

        // Create execution state
        let state = module.as_interface().get_state()?;
        let execution = ModuleExecution {
            state,
            metrics: ExecutionMetrics::default(),
            last_execution: None,
            is_running: false,
        };

        self.executions.write().await.insert(module_id.to_string(), execution);

        // Create execution lock
        self.execution_lock.write().await.insert(
            module_id.to_string(),
            Arc::new(Mutex::new(())),
        );

        Ok(())
    }

    /// Execute a module
    pub async fn execute(&self, module_id: &str, context: ExecutionContext) -> Result<Value> {
        let start = std::time::Instant::now();

        // Get execution lock to prevent concurrent execution
        let lock = {
            let locks = self.execution_lock.read().await;
            locks.get(module_id)
                .ok_or_else(|| anyhow::anyhow!("Module not initialized: {}", module_id))?
                .clone()
        };

        let _guard = lock.lock().await;

        // Mark as running
        {
            let mut executions = self.executions.write().await;
            if let Some(exec) = executions.get_mut(module_id) {
                if exec.is_running {
                    anyhow::bail!("Module is already running: {}", module_id);
                }
                exec.is_running = true;
            }
        }

        // Execute with timeout
        let result = if let Some(timeout_ms) = context.timeout_ms {
            timeout(
                Duration::from_millis(timeout_ms),
                self.execute_internal(module_id, context.input.clone()),
            )
            .await
            .map_err(|_| anyhow::anyhow!("Module execution timed out after {}ms", timeout_ms))?
        } else {
            self.execute_internal(module_id, context.input.clone()).await
        };

        // Update execution state
        {
            let mut executions = self.executions.write().await;
            if let Some(exec) = executions.get_mut(module_id) {
                exec.is_running = false;
                exec.last_execution = Some(chrono::Utc::now());
                exec.metrics.cpu_time_ms = start.elapsed().as_millis() as u64;
            }
        }

        // Update registry execution count
        {
            let mut registry = self.registry.write().await;
            registry.increment_execution_count(module_id)?;
        }

        result
    }

    /// Internal execution without locks
    async fn execute_internal(&self, module_id: &str, input: Value) -> Result<Value> {
        let registry = self.registry.read().await;
        
        let module = registry.get(module_id)
            .ok_or_else(|| anyhow::anyhow!("Module not found: {}", module_id))?;

        module.as_interface().execute(input).await
    }

    /// Shutdown a module
    pub async fn shutdown_module(&self, module_id: &str) -> Result<()> {
        // Wait for any running execution to complete
        let lock = {
            let locks = self.execution_lock.read().await;
            locks.get(module_id).cloned()
        };

        if let Some(lock) = lock {
            let _guard = lock.lock().await;
            
            // Shutdown the module
            let mut registry = self.registry.write().await;
            if let Some(module) = registry.get_mut(module_id) {
                module.as_interface_mut().shutdown().await?;
            }
        }

        // Remove execution state
        self.executions.write().await.remove(module_id);
        self.execution_lock.write().await.remove(module_id);

        Ok(())
    }

    /// Get module state
    pub async fn get_module_state(&self, module_id: &str) -> Result<ModuleState> {
        let registry = self.registry.read().await;
        
        let module = registry.get(module_id)
            .ok_or_else(|| anyhow::anyhow!("Module not found: {}", module_id))?;

        module.as_interface().get_state()
    }

    /// Restore module state
    pub async fn restore_module_state(&self, module_id: &str, state: ModuleState) -> Result<()> {
        let mut registry = self.registry.write().await;
        
        let module = registry.get_mut(module_id)
            .ok_or_else(|| anyhow::anyhow!("Module not found: {}", module_id))?;

        module.as_interface_mut().restore_state(state)?;

        // Update execution state
        if let Some(exec) = self.executions.write().await.get_mut(module_id) {
            exec.state = module.as_interface().get_state()?;
        }

        Ok(())
    }

    /// Health check for a module
    pub async fn health_check(&self, module_id: &str) -> Result<bool> {
        let registry = self.registry.read().await;
        
        let module = registry.get(module_id)
            .ok_or_else(|| anyhow::anyhow!("Module not found: {}", module_id))?;

        module.as_interface().health_check().await
    }

    /// Get execution metrics for a module
    pub async fn get_metrics(&self, module_id: &str) -> Result<ExecutionMetrics> {
        let executions = self.executions.read().await;
        
        executions.get(module_id)
            .map(|exec| exec.metrics.clone())
            .ok_or_else(|| anyhow::anyhow!("Module not found: {}", module_id))
    }

    /// Check if a module is currently running
    pub async fn is_running(&self, module_id: &str) -> bool {
        self.executions.read().await
            .get(module_id)
            .map(|exec| exec.is_running)
            .unwrap_or(false)
    }

    /// Stop a running module (force stop)
    pub async fn stop_module(&self, module_id: &str) -> Result<()> {
        let mut executions = self.executions.write().await;
        
        if let Some(exec) = executions.get_mut(module_id) {
            if exec.is_running {
                exec.is_running = false;
                // In a real implementation, we would interrupt the execution
                Ok(())
            } else {
                anyhow::bail!("Module is not running: {}", module_id)
            }
        } else {
            anyhow::bail!("Module not found: {}", module_id)
        }
    }

    /// Get all running modules
    pub async fn get_running_modules(&self) -> Vec<String> {
        self.executions.read().await
            .iter()
            .filter(|(_, exec)| exec.is_running)
            .map(|(id, _)| id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_context() {
        let context = ExecutionContext::new(Value::String("test".to_string()))
            .with_config("key".to_string(), Value::Bool(true))
            .with_timeout(1000)
            .with_tracing(true);

        assert_eq!(context.timeout_ms, Some(1000));
        assert!(context.trace_enabled);
        assert_eq!(context.config.get("key"), Some(&Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_runtime_creation() {
        let runtime = ModuleRuntime::new();
        let running = runtime.get_running_modules().await;
        assert_eq!(running.len(), 0);
    }
}