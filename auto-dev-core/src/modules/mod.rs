#![allow(unused)]
//! Dynamic Module System
//!
//! This module provides runtime loading, unloading, and hot-reload capabilities
//! for auto-dev-rs, enabling safe self-modification through modular updates.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod hot_reload;
pub mod interface;
pub mod loader;
pub mod messages;
pub mod native_host;
pub mod registry;
pub mod runtime;
pub mod sandbox;
pub mod store;
pub mod wasm_host;

#[cfg(test)]
mod tests;

pub use hot_reload::{HotReloadConfig, ReloadCoordinator, ReloadResult};
pub use interface::{ModuleCapability, ModuleInterface, ModuleState, ModuleVersion};
pub use loader::{ModuleFormat, ModuleLoader};
pub use messages::{Message, MessageBus, MessageHandler};
pub use registry::{ModuleInfo, ModuleRegistry, ModuleStatus};
pub use runtime::{ExecutionContext, ModuleRuntime};
pub use store::{ModuleStore, StoreConfig};

/// Build configuration for modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleBuildConfig {
    /// Build profile to use
    pub build_profile: crate::self_upgrade::BuildProfile,
    /// Whether to enable debug symbols
    pub debug_symbols: bool,
    /// Additional cargo features to enable
    pub features: Vec<String>,
    /// Environment variables for build
    pub env_vars: HashMap<String, String>,
}

impl Default for ModuleBuildConfig {
    fn default() -> Self {
        Self {
            build_profile: crate::self_upgrade::BuildProfile::Debug,
            debug_symbols: true,
            features: Vec::new(),
            env_vars: HashMap::new(),
        }
    }
}

/// Main module system that coordinates all module operations
pub struct ModuleSystem {
    registry: Arc<RwLock<ModuleRegistry>>,
    loader: Arc<ModuleLoader>,
    message_bus: Arc<MessageBus>,
    runtime: Arc<ModuleRuntime>,
    reload_coordinator: Arc<ReloadCoordinator>,
}

impl ModuleSystem {
    /// Create a new module system instance
    pub fn new() -> Result<Self> {
        Self::with_config(HotReloadConfig::default())
    }
    
    /// Create a new module system with custom hot-reload configuration
    pub fn with_config(hot_reload_config: HotReloadConfig) -> Result<Self> {
        let registry = Arc::new(RwLock::new(ModuleRegistry::new()));
        let loader = Arc::new(ModuleLoader::new()?);
        let message_bus = Arc::new(MessageBus::new());
        let runtime = Arc::new(ModuleRuntime::new());
        let reload_coordinator = Arc::new(ReloadCoordinator::new(hot_reload_config));

        Ok(Self { 
            registry, 
            loader, 
            message_bus, 
            runtime,
            reload_coordinator,
        })
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
    pub async fn reload_module(&self, module_id: &str, new_path: PathBuf) -> Result<ReloadResult> {
        // Use the hot-reload coordinator for safe, atomic reload with all phases
        let result = self.reload_coordinator
            .reload_module(
                module_id,
                new_path,
                self.registry.clone(),
                self.loader.clone(),
                self.runtime.clone(),
            )
            .await;
        
        match result {
            Ok(reload_result) => Ok(reload_result),
            Err(e) => Err(anyhow::anyhow!("Hot-reload failed: {}", e)),
        }
    }
    
    /// Hot-reload a module with the simple interface (for backwards compatibility)
    pub async fn reload_module_simple(&self, module_id: &str, new_path: PathBuf) -> Result<()> {
        let result = self.reload_module(module_id, new_path).await?;
        if result.success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Reload failed: {:?}", result.error))
        }
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
    
    /// Get hot-reload metrics
    pub async fn get_reload_metrics(&self) -> hot_reload::ReloadMetrics {
        self.reload_coordinator.get_metrics().await
    }
    
    /// Check if a module is currently being reloaded
    pub async fn is_reloading(&self, module_id: &str) -> bool {
        self.reload_coordinator.is_reloading(module_id).await
    }
    
    /// Get the current reload phase for a module
    pub async fn get_reload_phase(&self, module_id: &str) -> Option<hot_reload::ReloadPhase> {
        self.reload_coordinator.get_reload_phase(module_id).await
    }
}
