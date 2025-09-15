// Dynamic Module System
//
// This module provides runtime loading, unloading, and hot-reload capabilities
// for auto-dev-rs, enabling safe self-modification through modular updates.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod interface;
pub mod loader;
pub mod messages;
pub mod native_host;
pub mod registry;
pub mod runtime;
pub mod wasm_host;

#[cfg(test)]
mod tests;

pub use interface::{ModuleCapability, ModuleInterface, ModuleState, ModuleVersion};
pub use loader::{ModuleFormat, ModuleLoader};
pub use messages::{Message, MessageBus, MessageHandler};
pub use registry::{ModuleInfo, ModuleRegistry, ModuleStatus};
pub use runtime::{ExecutionContext, ModuleRuntime};

/// Main module system that coordinates all module operations
pub struct ModuleSystem {
    registry: Arc<RwLock<ModuleRegistry>>,
    loader: Arc<ModuleLoader>,
    message_bus: Arc<MessageBus>,
    runtime: Arc<ModuleRuntime>,
}

impl ModuleSystem {
    /// Create a new module system instance
    pub fn new() -> Result<Self> {
        let registry = Arc::new(RwLock::new(ModuleRegistry::new()));
        let loader = Arc::new(ModuleLoader::new()?);
        let message_bus = Arc::new(MessageBus::new());
        let runtime = Arc::new(ModuleRuntime::new());

        Ok(Self { registry, loader, message_bus, runtime })
    }

    /// Load a module from the specified path
    pub async fn load_module(&self, path: PathBuf, format: ModuleFormat) -> Result<String> {
        // Load the module
        let module = self.loader.load(path.clone(), format).await?;

        // Register the module
        let module_id = self.registry.write().await.register(module).await?;

        // Initialize the module
        self.runtime.initialize_module(&module_id).await?;

        Ok(module_id)
    }

    /// Unload a module
    pub async fn unload_module(&self, module_id: &str) -> Result<()> {
        // Shutdown the module gracefully
        self.runtime.shutdown_module(module_id).await?;

        // Unregister the module
        self.registry.write().await.unregister(module_id).await?;

        // Clean up resources
        self.loader.unload(module_id).await?;

        Ok(())
    }

    /// Hot-reload a module while preserving state
    pub async fn reload_module(&self, module_id: &str, new_path: PathBuf) -> Result<()> {
        // Get current module state
        let state = self.runtime.get_module_state(module_id).await?;

        // Load new version
        let format = self.loader.get_format(module_id)?;
        let new_module = self.loader.load(new_path, format).await?;

        // Swap modules atomically
        self.registry.write().await.update(module_id, new_module).await?;

        // Restore state
        self.runtime.restore_module_state(module_id, state).await?;

        Ok(())
    }

    /// Send a message to a module
    pub async fn send_message(&self, target: &str, message: Message) -> Result<()> {
        self.message_bus.send(target, message).await
    }

    /// Execute a module's main functionality
    pub async fn execute_module(
        &self,
        module_id: &str,
        context: ExecutionContext,
    ) -> Result<serde_json::Value> {
        self.runtime.execute(module_id, context).await
    }

    /// Get capabilities provided by a module
    pub async fn get_capabilities(&self, module_id: &str) -> Result<Vec<ModuleCapability>> {
        self.registry.read().await.get_capabilities(module_id)
    }

    /// List all loaded modules
    pub async fn list_modules(&self) -> Result<Vec<ModuleInfo>> {
        Ok(self.registry.read().await.list_all())
    }

    /// Check module health
    pub async fn health_check(&self, module_id: &str) -> Result<bool> {
        self.runtime.health_check(module_id).await
    }
}
