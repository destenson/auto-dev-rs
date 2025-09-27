//! Version Control System integration for self-development
//! 
//! Provides git operations to track and manage self-modifications

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod git_ops;
pub mod branch_manager;
pub mod commit_builder;
pub mod conflict_resolver;
pub mod pr_creator;
pub mod bisect;
pub mod history;

pub use git_ops::GitOperations;
pub use branch_manager::BranchManager;
pub use commit_builder::{CommitBuilder, CommitType};
pub use conflict_resolver::ConflictResolver;
pub use pr_creator::PullRequestCreator;
pub use bisect::{BisectManager, BisectHelper};
pub use history::{HistorySearcher, SearchBuilder};

/// Main VCS integration interface
pub struct VcsIntegration {
    repo_path: PathBuf,
    config: VcsConfig,
    git_ops: GitOperations,
    branch_manager: BranchManager,
    commit_builder: CommitBuilder,
}

/// VCS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcsConfig {
    /// Whether to automatically create branches for changes
    pub auto_branch: bool,
    /// Prefix for auto-created branches
    pub branch_prefix: String,
    /// Commit message style (conventional, simple)
    pub commit_style: CommitStyle,
    /// Whether to auto-merge after successful tests
    pub auto_merge: bool,
    /// Require tests to pass before committing
    pub require_tests: bool,
    /// Sign commits with GPG
    pub sign_commits: bool,
    /// Maximum conflict resolution attempts
    pub max_conflict_attempts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommitStyle {
    Conventional,
    Simple,
}

impl Default for VcsConfig {
    fn default() -> Self {
        Self {
            auto_branch: true,
            branch_prefix: "auto-dev".to_string(),
            commit_style: CommitStyle::Conventional,
            auto_merge: false,
            require_tests: true,
            sign_commits: false,
            max_conflict_attempts: 3,
        }
    }
}

impl VcsIntegration {
    /// Create new VCS integration
    pub fn new(repo_path: impl AsRef<Path>, config: VcsConfig) -> Result<Self> {
        let repo_path = repo_path.as_ref().to_path_buf();
        let git_ops = GitOperations::new(&repo_path)?;
        let branch_manager = BranchManager::new(&repo_path)?;
        let commit_builder = CommitBuilder::new(config.commit_style.clone());

        Ok(Self {
            repo_path,
            config,
            git_ops,
            branch_manager,
            commit_builder,
        })
    }

    /// Start a new feature branch for self-development
    pub async fn start_feature(&self, feature_name: &str) -> Result<String> {
        let branch_name = format!("{}/{}-{}", 
            self.config.branch_prefix,
            feature_name,
            chrono::Utc::now().timestamp()
        );
        
        self.branch_manager.create_and_switch(&branch_name)?;
        Ok(branch_name)
    }

    /// Commit current changes with semantic message
    pub async fn commit_changes(
        &self,
        commit_type: CommitType,
        scope: Option<&str>,
        description: &str,
    ) -> Result<String> {
        // Stage all changes
        self.git_ops.stage_all()?;
        
        // Generate commit message
        let message = self.commit_builder.build_message(
            commit_type,
            scope,
            description,
        );
        
        // Create commit
        let commit_id = self.git_ops.commit(&message, self.config.sign_commits)?;
        
        Ok(commit_id)
    }

    /// Complete feature and merge back
    pub async fn complete_feature(&self, branch_name: &str) -> Result<()> {
        if self.config.require_tests {
            // Run tests before merge
            self.run_tests().await?;
        }
        
        // Switch to main branch
        self.branch_manager.switch_to_main()?;
        
        // Merge feature branch
        self.git_ops.merge(branch_name)?;
        
        // Clean up feature branch
        self.branch_manager.delete_branch(branch_name)?;
        
        Ok(())
    }

    /// Handle merge conflicts
    pub async fn resolve_conflicts(&self) -> Result<ConflictResolution> {
        let resolver = ConflictResolver::new(&self.repo_path)?;
        resolver.attempt_resolution(self.config.max_conflict_attempts)
    }

    /// Create pull request for review
    pub async fn create_pr(
        &self,
        title: &str,
        description: &str,
    ) -> Result<PullRequestInfo> {
        let creator = PullRequestCreator::new(&self.repo_path)?;
        creator.create(title, description).await
    }

    /// Run tests before committing
    async fn run_tests(&self) -> Result<()> {
        // Use cargo test for Rust projects
        let output = std::process::Command::new("cargo")
            .arg("test")
            .current_dir(&self.repo_path)
            .output()?;
        
        if !output.status.success() {
            anyhow::bail!("Tests failed");
        }
        
        Ok(())
    }

    /// Get current branch name
    pub fn current_branch(&self) -> Result<String> {
        self.git_ops.current_branch()
    }

    /// Get repository status
    pub fn status(&self) -> Result<RepoStatus> {
        self.git_ops.status()
    }
}

/// Repository status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoStatus {
    pub current_branch: String,
    pub modified_files: Vec<PathBuf>,
    pub staged_files: Vec<PathBuf>,
    pub untracked_files: Vec<PathBuf>,
    pub has_conflicts: bool,
    pub is_clean: bool,
}

/// Conflict resolution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    Resolved,
    RequiresManual(Vec<PathBuf>),
    Failed(String),
}

/// Pull request information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestInfo {
    pub number: u32,
    pub url: String,
    pub branch: String,
    pub base: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vcs_config_default() {
        let config = VcsConfig::default();
        assert!(config.auto_branch);
        assert_eq!(config.branch_prefix, "auto-dev");
        assert!(!config.auto_merge);
    }
}