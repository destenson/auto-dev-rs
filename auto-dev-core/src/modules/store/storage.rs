//! Storage Manager for Local Module Store
//!
//! Handles the physical storage of modules on the local filesystem.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;

use crate::modules::store::manifest::{ManifestParser, ModuleManifest};

/// Storage layout version
const STORAGE_VERSION: &str = "1.0";

/// Module storage metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleStorageInfo {
    /// Unique module ID
    pub id: String,
    /// Module name from manifest
    pub name: String,
    /// Module version
    pub version: String,
    /// Storage path relative to store root
    pub path: PathBuf,
    /// Size in bytes
    pub size: u64,
    /// SHA256 checksum
    pub checksum: String,
    /// Timestamp when stored
    pub stored_at: chrono::DateTime<chrono::Utc>,
    /// Number of times accessed
    pub access_count: u64,
    /// Last access time
    pub last_accessed: Option<chrono::DateTime<chrono::Utc>>,
}

/// Manages physical storage of modules
pub struct StorageManager {
    /// Root directory for module storage
    store_root: PathBuf,
    /// Index of stored modules
    index: HashMap<String, ModuleStorageInfo>,
    /// Index file path
    index_path: PathBuf,
}

impl StorageManager {
    /// Create a new storage manager
    pub fn new(store_root: PathBuf) -> Result<Self> {
        // Create store directory if it doesn't exist
        if !store_root.exists() {
            fs::create_dir_all(&store_root)
                .with_context(|| format!("Failed to create store directory: {:?}", store_root))?;
        }

        let index_path = store_root.join("modules.index");
        let index =
            if index_path.exists() { Self::load_index(&index_path)? } else { HashMap::new() };

        Ok(Self { store_root, index, index_path })
    }

    /// Store a module
    pub async fn store_module(
        &mut self,
        module_path: &PathBuf,
        manifest: &ModuleManifest,
    ) -> Result<String> {
        // Generate module ID
        let module_id = Self::generate_module_id(&manifest.module.name, &manifest.module.version);

        // Check if module already exists
        if self.index.contains_key(&module_id) {
            anyhow::bail!("Module already exists: {}", module_id);
        }

        // Create module directory in store
        let store_path = self.store_root.join(&module_id);
        if store_path.exists() {
            async_fs::remove_dir_all(&store_path)
                .await
                .context("Failed to remove existing module directory")?;
        }
        async_fs::create_dir_all(&store_path).await.context("Failed to create module directory")?;

        // Copy module files
        self.copy_module_files(module_path, &store_path).await?;

        // Calculate checksum
        let checksum = self.calculate_checksum(&store_path).await?;

        // Calculate size
        let size = self.calculate_size(&store_path)?;

        // Create storage info
        let info = ModuleStorageInfo {
            id: module_id.clone(),
            name: manifest.module.name.clone(),
            version: manifest.module.version.clone(),
            path: PathBuf::from(&module_id),
            size,
            checksum,
            stored_at: chrono::Utc::now(),
            access_count: 0,
            last_accessed: None,
        };

        // Add to index
        self.index.insert(module_id.clone(), info);

        // Save index
        self.save_index()?;

        Ok(module_id)
    }

    /// Get module manifest
    pub fn get_manifest(&self, module_id: &str) -> Result<ModuleManifest> {
        let info = self
            .index
            .get(module_id)
            .ok_or_else(|| anyhow::anyhow!("Module not found: {}", module_id))?;

        let manifest_path = self.store_root.join(&info.path).join("module.toml");
        ManifestParser::parse_from_path(&manifest_path)
    }

    /// Get module storage path
    pub fn get_module_path(&self, module_id: &str) -> Result<PathBuf> {
        let info = self
            .index
            .get(module_id)
            .ok_or_else(|| anyhow::anyhow!("Module not found: {}", module_id))?;

        Ok(self.store_root.join(&info.path))
    }

    /// Remove a module
    pub async fn remove_module(&mut self, module_id: &str) -> Result<()> {
        let info = self
            .index
            .remove(module_id)
            .ok_or_else(|| anyhow::anyhow!("Module not found: {}", module_id))?;

        let module_path = self.store_root.join(&info.path);
        if module_path.exists() {
            async_fs::remove_dir_all(&module_path)
                .await
                .context("Failed to remove module directory")?;
        }

        self.save_index()?;
        Ok(())
    }

    /// List all stored modules
    pub fn list_modules(&self) -> Vec<ModuleStorageInfo> {
        self.index.values().cloned().collect()
    }

    /// Get module info
    pub fn get_info(&mut self, module_id: &str) -> Option<&ModuleStorageInfo> {
        // Update access info
        if let Some(info) = self.index.get_mut(module_id) {
            info.access_count += 1;
            info.last_accessed = Some(chrono::Utc::now());
            // Note: We should save the index here, but doing it on every access might be expensive
        }

        self.index.get(module_id)
    }

    /// Check if module exists
    pub fn exists(&self, module_id: &str) -> bool {
        self.index.contains_key(module_id)
    }

    /// Generate module ID from name and version
    fn generate_module_id(name: &str, version: &str) -> String {
        format!("{}-{}", name, version.replace('.', "_"))
    }

    /// Copy module files to store
    async fn copy_module_files(&self, src: &Path, dst: &Path) -> Result<()> {
        for entry in walkdir::WalkDir::new(src) {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();
            let relative = path.strip_prefix(src).unwrap();
            let target = dst.join(relative);

            if entry.file_type().is_dir() {
                async_fs::create_dir_all(&target).await.context("Failed to create directory")?;
            } else {
                if let Some(parent) = target.parent() {
                    async_fs::create_dir_all(parent)
                        .await
                        .context("Failed to create parent directory")?;
                }
                async_fs::copy(path, &target)
                    .await
                    .with_context(|| format!("Failed to copy file: {:?}", path))?;
            }
        }
        Ok(())
    }

    /// Calculate SHA256 checksum of module
    async fn calculate_checksum(&self, path: &Path) -> Result<String> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();

        for entry in walkdir::WalkDir::new(path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let content = async_fs::read(entry.path()).await?;
                hasher.update(&content);
            }
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Calculate total size of module
    fn calculate_size(&self, path: &Path) -> Result<u64> {
        let mut total = 0;
        for entry in walkdir::WalkDir::new(path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                total += entry.metadata()?.len();
            }
        }
        Ok(total)
    }

    /// Load index from file
    fn load_index(path: &Path) -> Result<HashMap<String, ModuleStorageInfo>> {
        let content = fs::read_to_string(path).context("Failed to read index file")?;
        serde_json::from_str(&content).context("Failed to parse index file")
    }

    /// Save index to file
    fn save_index(&self) -> Result<()> {
        let content =
            serde_json::to_string_pretty(&self.index).context("Failed to serialize index")?;
        fs::write(&self.index_path, content).context("Failed to write index file")?;
        Ok(())
    }

    /// Clean up orphaned modules (in store but not in index)
    pub async fn cleanup_orphaned(&mut self) -> Result<Vec<String>> {
        let mut orphaned = Vec::new();

        let entries = fs::read_dir(&self.store_root).context("Failed to read store directory")?;

        for entry in entries {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip index file and other non-module files
            if name.ends_with(".index") || name.starts_with('.') {
                continue;
            }

            // Check if module is in index
            if !self.index.contains_key(&name) {
                orphaned.push(name.clone());

                // Remove orphaned directory
                let path = entry.path();
                if path.is_dir() {
                    async_fs::remove_dir_all(&path)
                        .await
                        .context("Failed to remove orphaned module")?;
                }
            }
        }

        Ok(orphaned)
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> StorageStats {
        let total_modules = self.index.len();
        let total_size: u64 = self.index.values().map(|info| info.size).sum();
        let avg_size = if total_modules > 0 { total_size / total_modules as u64 } else { 0 };

        StorageStats { total_modules, total_size, avg_size, store_path: self.store_root.clone() }
    }
}

/// Storage statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_modules: usize,
    pub total_size: u64,
    pub avg_size: u64,
    pub store_path: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_module_id() {
        let id = StorageManager::generate_module_id("test-module", "1.0.0");
        assert_eq!(id, "test-module-1_0_0");
    }

    #[tokio::test]
    async fn test_storage_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf()).unwrap();
        assert!(manager.index.is_empty());
    }

    #[test]
    fn test_storage_stats() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StorageManager::new(temp_dir.path().to_path_buf()).unwrap();
        let stats = manager.get_stats();
        assert_eq!(stats.total_modules, 0);
        assert_eq!(stats.total_size, 0);
    }
}
