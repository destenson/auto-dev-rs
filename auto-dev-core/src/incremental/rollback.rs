//! Rollback and checkpoint management

use super::{Result, IncrementalError};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use tracing::{info, debug, warn};

/// Manages checkpoints and rollback operations using Git
pub struct RollbackManager {
    project_root: PathBuf,
}

impl RollbackManager {
    /// Create a new rollback manager
    pub fn new(project_root: PathBuf) -> Result<Self> {
        // Ensure Git is available
        if !Self::is_git_available(&project_root) {
            return Err(IncrementalError::RollbackError(
                "Git is required for rollback functionality. Please initialize a git repository.".to_string()
            ));
        }
        
        Ok(Self {
            project_root,
        })
    }
    
    /// Check if Git is available in the project
    fn is_git_available(project_root: &Path) -> bool {
        project_root.join(".git").exists()
    }
    
    /// Create a new checkpoint
    pub async fn create_checkpoint(&self) -> Result<CheckpointId> {
        let checkpoint_id = CheckpointId::new();
        info!("Creating Git checkpoint: {}", checkpoint_id.id);
        
        self.create_git_checkpoint(&checkpoint_id).await?;
        
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
    
    
    /// Rollback to a specific checkpoint
    pub async fn rollback_to(&self, checkpoint: CheckpointId) -> Result<()> {
        info!("Rolling back to Git checkpoint: {}", checkpoint.id);
        
        self.rollback_git_checkpoint(&checkpoint).await?;
        
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
    
    
    /// List all available checkpoints from Git stash
    pub async fn list_checkpoints(&self) -> Result<Vec<CheckpointMetadata>> {
        let output = Command::new("git")
            .args(&["stash", "list"])
            .current_dir(&self.project_root)
            .stdout(Stdio::piped())
            .output()
            .await?;
        
        let stash_list = String::from_utf8_lossy(&output.stdout);
        let mut checkpoints = Vec::new();
        
        for line in stash_list.lines() {
            if line.contains("auto-dev-checkpoint-") {
                // Parse the checkpoint ID from the stash message
                if let Some(id_str) = line.split("auto-dev-checkpoint-").nth(1) {
                    if let Some(id_str) = id_str.split_whitespace().next() {
                        if let Ok(id) = id_str.parse::<Uuid>() {
                            checkpoints.push(CheckpointMetadata {
                                id,
                                created_at: Utc::now(), // Git doesn't easily give us the stash timestamp
                                description: Some(format!("Git stash checkpoint")),
                            });
                        }
                    }
                }
            }
        }
        
        Ok(checkpoints)
    }
    
    /// Clean up old checkpoints, keeping the most recent N
    pub async fn cleanup_old_checkpoints(&self, keep_count: usize) -> Result<()> {
        // Git stash management is more complex, and we might want to keep stashes
        // for other purposes. For now, we'll just log that we would clean up.
        debug!("Checkpoint cleanup requested, keeping {}", keep_count);
        // In a production system, we might want to implement smarter stash management
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