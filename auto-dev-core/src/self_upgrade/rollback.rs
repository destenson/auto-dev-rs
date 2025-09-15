//! Rollback mechanism for failed upgrades

use anyhow::{Result, Context};
use std::path::PathBuf;
use tracing::{info, warn};

/// Manages rollback of failed upgrades
pub struct RollbackManager {
    binary_path: PathBuf,
    backup_dir: PathBuf,
    keep_versions: usize,
}

impl RollbackManager {
    pub fn new(binary_path: PathBuf, keep_versions: usize) -> Self {
        Self {
            binary_path,
            backup_dir: PathBuf::from(".auto-dev/backups"),
            keep_versions,
        }
    }
    
    /// Create a backup of the current binary
    pub async fn create_backup(&self) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.backup_dir)?;
        
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_name = format!("auto-dev_{}", timestamp);
        let backup_path = self.backup_dir.join(&backup_name);
        
        if self.binary_path.exists() {
            std::fs::copy(&self.binary_path, &backup_path)
                .context("Failed to create backup")?;
            info!("Created backup at {:?}", backup_path);
            
            // Clean old backups
            self.clean_old_backups().await?;
        }
        
        Ok(backup_path)
    }
    
    /// Rollback to the previous version
    pub async fn rollback(&self) -> Result<()> {
        warn!("Initiating rollback");
        
        let latest_backup = self.get_latest_backup()?;
        if let Some(backup) = latest_backup {
            info!("Rolling back to {:?}", backup);
            std::fs::copy(&backup, &self.binary_path)?;
            info!("Rollback completed");
            Ok(())
        } else {
            Err(anyhow::anyhow!("No backup available for rollback"))
        }
    }
    
    /// Get the latest backup
    fn get_latest_backup(&self) -> Result<Option<PathBuf>> {
        if !self.backup_dir.exists() {
            return Ok(None);
        }
        
        let mut backups: Vec<_> = std::fs::read_dir(&self.backup_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_name().to_string_lossy().starts_with("auto-dev_")
            })
            .collect();
        
        backups.sort_by_key(|entry| entry.metadata().ok().and_then(|m| m.modified().ok()));
        backups.reverse();
        
        Ok(backups.first().map(|entry| entry.path()))
    }
    
    /// Clean old backups beyond keep_versions
    async fn clean_old_backups(&self) -> Result<()> {
        if !self.backup_dir.exists() {
            return Ok(());
        }
        
        let mut backups: Vec<_> = std::fs::read_dir(&self.backup_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_name().to_string_lossy().starts_with("auto-dev_")
            })
            .collect();
        
        if backups.len() <= self.keep_versions {
            return Ok(());
        }
        
        backups.sort_by_key(|entry| entry.metadata().ok().and_then(|m| m.modified().ok()));
        
        let to_remove = backups.len() - self.keep_versions;
        for entry in backups.iter().take(to_remove) {
            if let Err(e) = std::fs::remove_file(entry.path()) {
                warn!("Failed to remove old backup: {}", e);
            }
        }
        
        Ok(())
    }
}