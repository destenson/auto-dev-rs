//! Module Installer for Local Store
//!
//! Handles installation of modules from the local store to the active modules directory.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;

use crate::modules::store::manifest::{ModuleDependency, ModuleManifest};

/// Installation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InstallStatus {
    /// Module is installed and active
    Installed,
    /// Module is being installed
    Installing,
    /// Module installation failed
    Failed(String),
    /// Module is not installed
    NotInstalled,
}

/// Installation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallRecord {
    /// Module ID
    pub module_id: String,
    /// Module name
    pub name: String,
    /// Module version
    pub version: String,
    /// Installation path
    pub install_path: PathBuf,
    /// Installation status
    pub status: InstallStatus,
    /// Installation timestamp
    pub installed_at: chrono::DateTime<chrono::Utc>,
    /// Dependencies installed
    pub dependencies: Vec<String>,
}

/// Module installer
pub struct ModuleInstaller {
    /// Installation directory
    install_root: PathBuf,
    /// Installation records
    records: HashMap<String, InstallRecord>,
    /// Records file path
    records_path: PathBuf,
}

impl ModuleInstaller {
    /// Create a new installer
    pub fn new(install_root: PathBuf) -> Self {
        // Create install directory if it doesn't exist
        if !install_root.exists() {
            fs::create_dir_all(&install_root).ok();
        }

        let records_path = install_root.join("installed.json");
        let records = if records_path.exists() {
            Self::load_records(&records_path).unwrap_or_default()
        } else {
            HashMap::new()
        };

        Self { install_root, records, records_path }
    }

    /// Install a module
    pub async fn install(
        &mut self,
        source_path: &Path,
        manifest: &ModuleManifest,
    ) -> Result<PathBuf> {
        let module_id = Self::generate_module_id(&manifest.module.name, &manifest.module.version);

        // Check if already installed
        if let Some(record) = self.records.get(&module_id) {
            if record.status == InstallStatus::Installed {
                return Ok(record.install_path.clone());
            }
        }

        // Create installation record
        let install_path = self.install_root.join(&module_id);
        let mut record = InstallRecord {
            module_id: module_id.clone(),
            name: manifest.module.name.clone(),
            version: manifest.module.version.clone(),
            install_path: install_path.clone(),
            status: InstallStatus::Installing,
            installed_at: chrono::Utc::now(),
            dependencies: Vec::new(),
        };

        // Update status to installing
        self.records.insert(module_id.clone(), record.clone());
        self.save_records()?;

        // Install dependencies first
        if let Some(deps) = &manifest.dependencies {
            for dep in deps {
                if !dep.optional {
                    record.dependencies.push(dep.name.clone());
                }
            }
        }

        // Copy module files
        match self.copy_module_files(source_path, &install_path).await {
            Ok(_) => {
                record.status = InstallStatus::Installed;

                // Run post-install hooks if any
                self.run_post_install(&install_path).await?;
            }
            Err(e) => {
                record.status = InstallStatus::Failed(e.to_string());
                self.records.insert(module_id.clone(), record);
                self.save_records()?;
                return Err(e);
            }
        }

        // Update record
        self.records.insert(module_id, record);
        self.save_records()?;

        Ok(install_path)
    }

    /// Uninstall a module
    pub async fn uninstall(&mut self, module_id: &str) -> Result<()> {
        let record = self
            .records
            .remove(module_id)
            .ok_or_else(|| anyhow::anyhow!("Module not installed: {}", module_id))?;

        // Check for dependent modules
        let dependents = self.find_dependents(module_id);
        if !dependents.is_empty() {
            anyhow::bail!("Cannot uninstall: required by {:?}", dependents);
        }

        // Remove module files
        if record.install_path.exists() {
            async_fs::remove_dir_all(&record.install_path)
                .await
                .context("Failed to remove module files")?;
        }

        self.save_records()?;
        Ok(())
    }

    /// Update an installed module
    pub async fn update(
        &mut self,
        source_path: &Path,
        manifest: &ModuleManifest,
    ) -> Result<PathBuf> {
        let module_name = &manifest.module.name;

        // Find existing installation
        let old_record = self.records.values().find(|r| r.name == *module_name).cloned();

        if let Some(old) = old_record {
            // Backup old version
            let backup_path = self.install_root.join(format!("{}.backup", old.module_id));
            if old.install_path.exists() {
                self.copy_module_files(&old.install_path, &backup_path).await?;
            }

            // Install new version
            match self.install(source_path, manifest).await {
                Ok(path) => {
                    // Remove backup
                    if backup_path.exists() {
                        async_fs::remove_dir_all(&backup_path).await.ok();
                    }
                    Ok(path)
                }
                Err(e) => {
                    // Restore from backup
                    if backup_path.exists() {
                        self.copy_module_files(&backup_path, &old.install_path).await?;
                        async_fs::remove_dir_all(&backup_path).await.ok();
                    }
                    Err(e)
                }
            }
        } else {
            // No existing installation, just install
            self.install(source_path, manifest).await
        }
    }

    /// Check if a module is installed
    pub fn is_installed(&self, module_id: &str) -> bool {
        self.records.get(module_id).map(|r| r.status == InstallStatus::Installed).unwrap_or(false)
    }

    /// Get installation record
    pub fn get_record(&self, module_id: &str) -> Option<&InstallRecord> {
        self.records.get(module_id)
    }

    /// List all installed modules
    pub fn list_installed(&self) -> Vec<InstallRecord> {
        self.records.values().filter(|r| r.status == InstallStatus::Installed).cloned().collect()
    }

    /// Find modules that depend on the given module
    fn find_dependents(&self, module_id: &str) -> Vec<String> {
        self.records
            .values()
            .filter(|r| r.dependencies.contains(&module_id.to_string()))
            .map(|r| r.module_id.clone())
            .collect()
    }

    /// Copy module files
    async fn copy_module_files(&self, src: &Path, dst: &Path) -> Result<()> {
        // Remove destination if it exists
        if dst.exists() {
            async_fs::remove_dir_all(dst)
                .await
                .context("Failed to remove existing installation")?;
        }

        // Create destination directory
        async_fs::create_dir_all(dst).await.context("Failed to create installation directory")?;

        // Copy files
        for entry in walkdir::WalkDir::new(src) {
            let entry = entry.context("Failed to read source directory")?;
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

    /// Run post-install hooks
    async fn run_post_install(&self, install_path: &Path) -> Result<()> {
        let hook_path = install_path.join("post_install.sh");
        if hook_path.exists() {
            // TODO: Execute post-install script safely
            // For now, we skip this for security reasons
        }
        Ok(())
    }

    /// Generate module ID
    fn generate_module_id(name: &str, version: &str) -> String {
        format!("{}-{}", name, version.replace('.', "_"))
    }

    /// Load installation records
    fn load_records(path: &Path) -> Result<HashMap<String, InstallRecord>> {
        let content = fs::read_to_string(path).context("Failed to read installation records")?;
        serde_json::from_str(&content).context("Failed to parse installation records")
    }

    /// Save installation records
    fn save_records(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.records)
            .context("Failed to serialize installation records")?;
        fs::write(&self.records_path, content).context("Failed to write installation records")?;
        Ok(())
    }

    /// Verify module installation
    pub fn verify_installation(&self, module_id: &str) -> Result<bool> {
        let record = self
            .records
            .get(module_id)
            .ok_or_else(|| anyhow::anyhow!("Module not installed: {}", module_id))?;

        if record.status != InstallStatus::Installed {
            return Ok(false);
        }

        // Check if files exist
        if !record.install_path.exists() {
            return Ok(false);
        }

        // Check for manifest file
        let manifest_path = record.install_path.join("module.toml");
        if !manifest_path.exists() {
            return Ok(false);
        }

        Ok(true)
    }

    /// Clean up failed installations
    pub async fn cleanup_failed(&mut self) -> Result<Vec<String>> {
        let mut cleaned = Vec::new();

        let failed: Vec<String> = self
            .records
            .iter()
            .filter(|(_, r)| matches!(r.status, InstallStatus::Failed(_)))
            .map(|(id, _)| id.clone())
            .collect();

        for module_id in failed {
            if let Some(record) = self.records.remove(&module_id) {
                // Remove installation directory if it exists
                if record.install_path.exists() {
                    async_fs::remove_dir_all(&record.install_path)
                        .await
                        .context("Failed to remove failed installation")?;
                }
                cleaned.push(module_id);
            }
        }

        if !cleaned.is_empty() {
            self.save_records()?;
        }

        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_installer_creation() {
        let temp_dir = TempDir::new().unwrap();
        let installer = ModuleInstaller::new(temp_dir.path().to_path_buf());
        assert!(installer.records.is_empty());
    }

    #[test]
    fn test_generate_module_id() {
        let id = ModuleInstaller::generate_module_id("test-module", "1.2.3");
        assert_eq!(id, "test-module-1_2_3");
    }

    #[test]
    fn test_is_installed() {
        let temp_dir = TempDir::new().unwrap();
        let installer = ModuleInstaller::new(temp_dir.path().to_path_buf());
        assert!(!installer.is_installed("non-existent"));
    }
}
