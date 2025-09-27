//! Local Module Store
//!
//! Manages a local repository of modules that can be discovered, installed,
//! and reused by auto-dev-rs.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod discovery;
pub mod installer;
pub mod manifest;
pub mod storage;

#[cfg(test)]
mod tests;

pub use discovery::ModuleDiscovery;
pub use installer::ModuleInstaller;
pub use manifest::{ManifestParser, ModuleManifest};
pub use storage::StorageManager;

/// Local module store configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreConfig {
    /// Base directory for module storage
    pub store_path: PathBuf,
    /// Directory for installed modules
    pub install_path: PathBuf,
    /// Directory for module cache
    pub cache_path: PathBuf,
    /// Enable module verification
    pub verify_signatures: bool,
    /// Maximum module size in bytes
    pub max_module_size: u64,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            store_path: PathBuf::from("./module_store"),
            install_path: PathBuf::from("./modules"),
            cache_path: PathBuf::from("./module_cache"),
            verify_signatures: false,
            max_module_size: 100 * 1024 * 1024, // 100MB
        }
    }
}

/// Main interface for the local module store
pub struct ModuleStore {
    config: StoreConfig,
    storage: StorageManager,
    discovery: ModuleDiscovery,
    installer: ModuleInstaller,
}

impl ModuleStore {
    /// Create a new module store
    pub fn new(config: StoreConfig) -> Result<Self> {
        let storage = StorageManager::new(config.store_path.clone())?;
        let discovery = ModuleDiscovery::new(config.store_path.clone());
        let installer = ModuleInstaller::new(config.install_path.clone());

        Ok(Self { config, storage, discovery, installer })
    }

    /// Add a module to the store
    pub async fn add_module(&mut self, module_path: &PathBuf) -> Result<String> {
        // Parse manifest
        let manifest = ManifestParser::parse_from_path(&module_path.join("module.toml"))?;

        // Validate module
        self.validate_module(&manifest, module_path)?;

        // Store module
        let module_id = self.storage.store_module(module_path, &manifest).await?;

        // Index for discovery
        self.discovery.index_module(&module_id, &manifest)?;

        Ok(module_id)
    }

    /// Search for modules
    pub fn search(&self, query: &str) -> Result<Vec<ModuleManifest>> {
        self.discovery.search(query)
    }

    /// Get module by ID
    pub fn get_module(&self, module_id: &str) -> Result<ModuleManifest> {
        self.storage.get_manifest(module_id)
    }

    /// Install a module from the store
    pub async fn install(&mut self, module_id: &str) -> Result<PathBuf> {
        // Get module manifest
        let manifest = self.storage.get_manifest(module_id)?;

        // Get module path in store
        let store_path = self.storage.get_module_path(module_id)?;

        // Install module
        let install_path = self.installer.install(&store_path, &manifest).await?;

        Ok(install_path)
    }

    /// List all modules in the store
    pub fn list_all(&self) -> Result<Vec<ModuleManifest>> {
        self.discovery.list_all()
    }

    /// List modules by category
    pub fn list_by_category(&self, category: &str) -> Result<Vec<ModuleManifest>> {
        self.discovery.list_by_category(category)
    }

    /// Remove a module from the store
    pub async fn remove(&mut self, module_id: &str) -> Result<()> {
        // Remove from storage
        self.storage.remove_module(module_id).await?;

        // Remove from index
        self.discovery.remove_from_index(module_id)?;

        Ok(())
    }

    /// Validate a module before adding
    fn validate_module(&self, manifest: &ModuleManifest, path: &PathBuf) -> Result<()> {
        // Check module size
        let size = self.calculate_module_size(path)?;
        if size > self.config.max_module_size {
            anyhow::bail!("Module exceeds maximum size limit");
        }

        // Verify required files exist
        if !path.join("module.toml").exists() {
            anyhow::bail!("Missing module.toml manifest");
        }

        // Verify signature if enabled
        if self.config.verify_signatures {
            // TODO: Implement signature verification
        }

        Ok(())
    }

    /// Calculate total size of a module
    fn calculate_module_size(&self, path: &PathBuf) -> Result<u64> {
        let mut total_size = 0;
        for entry in walkdir::WalkDir::new(path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                total_size += entry.metadata()?.len();
            }
        }
        Ok(total_size)
    }
}
