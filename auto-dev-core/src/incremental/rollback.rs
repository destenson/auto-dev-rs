//! Rollback and checkpoint management

use super::{Result, IncrementalError};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tokio::fs;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use tracing::{info, debug, warn, error};

/// Manages checkpoints and rollback operations
pub struct RollbackManager {
    project_root: PathBuf,
    checkpoint_dir: PathBuf,
    use_git: bool,
}

impl RollbackManager {
    /// Create a new rollback manager
    pub fn new(project_root: PathBuf) -> Result<Self> {
        let checkpoint_dir = project_root.join(".auto-dev").join("checkpoints");
        
        // Check if Git is available and initialized
        let use_git = Self::is_git_available(&project_root);
        
        Ok(Self {
            project_root,
            checkpoint_dir,
            use_git,
        })
    }
    
    /// Check if Git is available in the project
    fn is_git_available(project_root: &Path) -> bool {
        project_root.join(".git").exists()
    }
    
    /// Create a new checkpoint
    pub async fn create_checkpoint(&self) -> Result<CheckpointId> {
        let checkpoint_id = CheckpointId::new();
        info!("Creating checkpoint: {}", checkpoint_id.id);
        
        if self.use_git {
            self.create_git_checkpoint(&checkpoint_id).await?;
        } else {
            self.create_file_checkpoint(&checkpoint_id).await?;
        }
        
        // Save checkpoint metadata
        self.save_checkpoint_metadata(&checkpoint_id).await?;
        
        Ok(checkpoint_id)
    }
    
    /// Create a Git-based checkpoint
    async fn create_git_checkpoint(&self, checkpoint: &CheckpointId) -> Result<()> {
        debug!("Creating Git checkpoint: {}", checkpoint.id);
        
        // Stage all changes
        let output = Command::new("git")
            .args(&["add", "-A"])
            .current_dir(&self.project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| IncrementalError::RollbackError(format!("Failed to stage changes: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to stage changes: {}", stderr);
        }
        
        // Create a stash with checkpoint ID
        let stash_message = format!("auto-dev-checkpoint-{}", checkpoint.id);
        let output = Command::new("git")
            .args(&["stash", "push", "-m", &stash_message, "--include-untracked"])
            .current_dir(&self.project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| IncrementalError::RollbackError(format!("Failed to create stash: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // It's okay if there's nothing to stash
            if !stderr.contains("No local changes") {
                return Err(IncrementalError::RollbackError(format!("Failed to create stash: {}", stderr)));
            }
        }
        
        Ok(())
    }
    
    /// Create a file-based checkpoint (fallback when Git is not available)
    async fn create_file_checkpoint(&self, checkpoint: &CheckpointId) -> Result<()> {
        debug!("Creating file-based checkpoint: {}", checkpoint.id);
        
        let checkpoint_path = self.checkpoint_dir.join(&checkpoint.id.to_string());
        fs::create_dir_all(&checkpoint_path).await?;
        
        // Copy all source files to checkpoint directory
        self.copy_directory_recursive(&self.project_root.join("src"), &checkpoint_path.join("src")).await?;
        
        // Save Cargo.toml if it exists
        let cargo_toml = self.project_root.join("Cargo.toml");
        if cargo_toml.exists() {
            let dest = checkpoint_path.join("Cargo.toml");
            fs::copy(&cargo_toml, &dest).await?;
        }
        
        Ok(())
    }
    
    /// Copy directory recursively
    async fn copy_directory_recursive(&self, src: &Path, dest: &Path) -> Result<()> {
        fs::create_dir_all(dest).await?;
        
        let mut entries = fs::read_dir(src).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_name = entry.file_name();
            let dest_path = dest.join(&file_name);
            
            if path.is_dir() {
                self.copy_directory_recursive(&path, &dest_path).await?;
            } else {
                fs::copy(&path, &dest_path).await?;
            }
        }
        
        Ok(())
    }
    
    /// Rollback to a specific checkpoint
    pub async fn rollback_to(&self, checkpoint: CheckpointId) -> Result<()> {
        info!("Rolling back to checkpoint: {}", checkpoint.id);
        
        if self.use_git {
            self.rollback_git_checkpoint(&checkpoint).await?;
        } else {
            self.rollback_file_checkpoint(&checkpoint).await?;
        }
        
        Ok(())
    }
    
    /// Rollback using Git
    async fn rollback_git_checkpoint(&self, checkpoint: &CheckpointId) -> Result<()> {
        debug!("Rolling back Git checkpoint: {}", checkpoint.id);
        
        // First, discard any current changes
        let output = Command::new("git")
            .args(&["reset", "--hard"])
            .current_dir(&self.project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| IncrementalError::RollbackError(format!("Failed to reset: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to reset: {}", stderr);
        }
        
        // Find and apply the stash
        let stash_name = format!("auto-dev-checkpoint-{}", checkpoint.id);
        
        // List stashes to find the right one
        let output = Command::new("git")
            .args(&["stash", "list"])
            .current_dir(&self.project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| IncrementalError::RollbackError(format!("Failed to list stashes: {}", e)))?;
        
        let stash_list = String::from_utf8_lossy(&output.stdout);
        let stash_index = stash_list
            .lines()
            .position(|line| line.contains(&stash_name));
        
        if let Some(index) = stash_index {
            // Apply the stash
            let stash_ref = format!("stash@{{{}}}", index);
            let output = Command::new("git")
                .args(&["stash", "apply", &stash_ref])
                .current_dir(&self.project_root)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|e| IncrementalError::RollbackError(format!("Failed to apply stash: {}", e)))?;
            
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(IncrementalError::RollbackError(format!("Failed to apply stash: {}", stderr)));
            }
        } else {
            warn!("Checkpoint stash not found: {}", stash_name);
        }
        
        Ok(())
    }
    
    /// Rollback using file-based checkpoint
    async fn rollback_file_checkpoint(&self, checkpoint: &CheckpointId) -> Result<()> {
        debug!("Rolling back file-based checkpoint: {}", checkpoint.id);
        
        let checkpoint_path = self.checkpoint_dir.join(&checkpoint.id.to_string());
        
        if !checkpoint_path.exists() {
            return Err(IncrementalError::RollbackError(format!(
                "Checkpoint not found: {}", checkpoint.id
            )));
        }
        
        // Restore source files
        let src_checkpoint = checkpoint_path.join("src");
        if src_checkpoint.exists() {
            let src_dest = self.project_root.join("src");
            if src_dest.exists() {
                fs::remove_dir_all(&src_dest).await?;
            }
            self.copy_directory_recursive(&src_checkpoint, &src_dest).await?;
        }
        
        // Restore Cargo.toml
        let cargo_checkpoint = checkpoint_path.join("Cargo.toml");
        if cargo_checkpoint.exists() {
            let cargo_dest = self.project_root.join("Cargo.toml");
            fs::copy(&cargo_checkpoint, &cargo_dest).await?;
        }
        
        Ok(())
    }
    
    /// Save checkpoint metadata
    async fn save_checkpoint_metadata(&self, checkpoint: &CheckpointId) -> Result<()> {
        fs::create_dir_all(&self.checkpoint_dir).await?;
        
        let metadata_path = self.checkpoint_dir.join(format!("{}.json", checkpoint.id));
        let metadata = CheckpointMetadata {
            id: checkpoint.id,
            created_at: checkpoint.created_at,
            description: checkpoint.description.clone(),
        };
        
        let json = serde_json::to_string_pretty(&metadata)?;
        fs::write(metadata_path, json).await?;
        
        Ok(())
    }
    
    /// List all available checkpoints
    pub async fn list_checkpoints(&self) -> Result<Vec<CheckpointMetadata>> {
        let mut checkpoints = Vec::new();
        
        if !self.checkpoint_dir.exists() {
            return Ok(checkpoints);
        }
        
        let mut entries = fs::read_dir(&self.checkpoint_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path).await?;
                if let Ok(metadata) = serde_json::from_str::<CheckpointMetadata>(&content) {
                    checkpoints.push(metadata);
                }
            }
        }
        
        // Sort by creation time (newest first)
        checkpoints.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        Ok(checkpoints)
    }
    
    /// Clean up old checkpoints, keeping the most recent N
    pub async fn cleanup_old_checkpoints(&self, keep_count: usize) -> Result<()> {
        let checkpoints = self.list_checkpoints().await?;
        
        if checkpoints.len() <= keep_count {
            return Ok(());
        }
        
        info!("Cleaning up old checkpoints, keeping {}", keep_count);
        
        for checkpoint in checkpoints.iter().skip(keep_count) {
            self.delete_checkpoint(&checkpoint.id).await?;
        }
        
        Ok(())
    }
    
    /// Delete a specific checkpoint
    async fn delete_checkpoint(&self, checkpoint_id: &Uuid) -> Result<()> {
        debug!("Deleting checkpoint: {}", checkpoint_id);
        
        // Delete metadata
        let metadata_path = self.checkpoint_dir.join(format!("{}.json", checkpoint_id));
        if metadata_path.exists() {
            fs::remove_file(metadata_path).await?;
        }
        
        // Delete file-based checkpoint if it exists
        let checkpoint_path = self.checkpoint_dir.join(checkpoint_id.to_string());
        if checkpoint_path.exists() {
            fs::remove_dir_all(checkpoint_path).await?;
        }
        
        // Remove Git stash if using Git
        if self.use_git {
            let stash_name = format!("auto-dev-checkpoint-{}", checkpoint_id);
            
            // Find and drop the stash
            let output = Command::new("git")
                .args(&["stash", "list"])
                .current_dir(&self.project_root)
                .stdout(Stdio::piped())
                .output()
                .await?;
            
            let stash_list = String::from_utf8_lossy(&output.stdout);
            if let Some(index) = stash_list.lines().position(|line| line.contains(&stash_name)) {
                let stash_ref = format!("stash@{{{}}}", index);
                Command::new("git")
                    .args(&["stash", "drop", &stash_ref])
                    .current_dir(&self.project_root)
                    .output()
                    .await?;
            }
        }
        
        Ok(())
    }
}

/// Checkpoint identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointId {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub description: Option<String>,
}

impl CheckpointId {
    /// Create a new checkpoint ID
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            description: None,
        }
    }
    
    /// Create with description
    pub fn with_description(description: String) -> Self {
        let mut checkpoint = Self::new();
        checkpoint.description = Some(description);
        checkpoint
    }
}

/// Checkpoint metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_checkpoint_creation() {
        let checkpoint = CheckpointId::new();
        assert!(!checkpoint.id.is_nil());
    }
    
    #[test]
    fn test_checkpoint_with_description() {
        let checkpoint = CheckpointId::with_description("Test checkpoint".to_string());
        assert_eq!(checkpoint.description, Some("Test checkpoint".to_string()));
    }
}