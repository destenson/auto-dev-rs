//! Branch management for git repositories

use anyhow::{Context, Result};
use git2::{Branch, BranchType, Repository};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Manages git branches for self-development
pub struct BranchManager {
    repo: Repository,
    repo_path: PathBuf,
}

impl BranchManager {
    /// Create new branch manager
    pub fn new(repo_path: impl AsRef<Path>) -> Result<Self> {
        let repo_path = repo_path.as_ref().to_path_buf();
        let repo = Repository::open(&repo_path)
            .context("Failed to open git repository")?;
        
        Ok(Self { repo, repo_path })
    }

    /// Create a new branch and switch to it
    pub fn create_and_switch(&self, branch_name: &str) -> Result<()> {
        // Get current HEAD commit
        let head_commit = self.repo.head()
            .context("Failed to get HEAD")?
            .peel_to_commit()
            .context("Failed to get HEAD commit")?;
        
        // Create new branch
        let branch = self.repo.branch(
            branch_name,
            &head_commit,
            false, // don't force if exists
        ).with_context(|| format!("Failed to create branch: {}", branch_name))?;
        
        info!("Created branch: {}", branch_name);
        
        // Switch to new branch
        self.switch_to_branch(branch_name)?;
        
        Ok(())
    }

    /// Switch to an existing branch
    pub fn switch_to_branch(&self, branch_name: &str) -> Result<()> {
        let branch = self.repo.find_branch(branch_name, BranchType::Local)
            .with_context(|| format!("Failed to find branch: {}", branch_name))?;
        
        let reference = branch.get();
        let ref_name = reference.name()
            .ok_or_else(|| anyhow::anyhow!("Invalid branch reference"))?;
        
        // Set HEAD to point to the branch
        self.repo.set_head(ref_name)
            .with_context(|| format!("Failed to switch to branch: {}", branch_name))?;
        
        // Checkout the working tree
        self.repo.checkout_head(Some(
            git2::build::CheckoutBuilder::default()
                .force() // Force checkout to handle conflicts
                .remove_untracked(false) // Don't remove untracked files
        )).context("Failed to checkout branch")?;
        
        info!("Switched to branch: {}", branch_name);
        Ok(())
    }

    /// Switch to main/master branch
    pub fn switch_to_main(&self) -> Result<()> {
        // Try common main branch names
        let main_branches = ["main", "master"];
        
        for branch_name in &main_branches {
            if self.branch_exists(branch_name)? {
                return self.switch_to_branch(branch_name);
            }
        }
        
        anyhow::bail!("No main/master branch found")
    }

    /// Delete a branch
    pub fn delete_branch(&self, branch_name: &str) -> Result<()> {
        // Ensure we're not deleting current branch
        let current = self.current_branch()?;
        if current == branch_name {
            anyhow::bail!("Cannot delete current branch: {}", branch_name);
        }
        
        let mut branch = self.repo.find_branch(branch_name, BranchType::Local)
            .with_context(|| format!("Failed to find branch: {}", branch_name))?;
        
        branch.delete()
            .with_context(|| format!("Failed to delete branch: {}", branch_name))?;
        
        info!("Deleted branch: {}", branch_name);
        Ok(())
    }

    /// Check if a branch exists
    pub fn branch_exists(&self, branch_name: &str) -> Result<bool> {
        match self.repo.find_branch(branch_name, BranchType::Local) {
            Ok(_) => Ok(true),
            Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// List all local branches
    pub fn list_branches(&self) -> Result<Vec<BranchInfo>> {
        let branches = self.repo.branches(Some(BranchType::Local))?;
        let current = self.current_branch()?;
        
        let mut branch_list = Vec::new();
        for branch_result in branches {
            let (branch, _) = branch_result?;
            if let Some(name) = branch.name()? {
                let is_current = name == current;
                let upstream = branch.upstream()
                    .ok()
                    .and_then(|u| u.name().ok().flatten().map(|n| n.to_string()));
                
                branch_list.push(BranchInfo {
                    name: name.to_string(),
                    is_current,
                    upstream,
                });
            }
        }
        
        Ok(branch_list)
    }

    /// Get current branch name
    pub fn current_branch(&self) -> Result<String> {
        let head = self.repo.head()
            .context("Failed to get HEAD")?;
        
        if head.is_branch() {
            let name = head.shorthand()
                .ok_or_else(|| anyhow::anyhow!("Invalid branch name"))?;
            Ok(name.to_string())
        } else {
            Err(anyhow::anyhow!("HEAD is detached"))
        }
    }

    /// Create a branch from a specific commit
    pub fn create_branch_from_commit(&self, branch_name: &str, commit_id: &str) -> Result<()> {
        let oid = git2::Oid::from_str(commit_id)
            .with_context(|| format!("Invalid commit ID: {}", commit_id))?;
        
        let commit = self.repo.find_commit(oid)
            .with_context(|| format!("Commit not found: {}", commit_id))?;
        
        self.repo.branch(branch_name, &commit, false)?;
        
        info!("Created branch {} from commit {}", branch_name, commit_id);
        Ok(())
    }

    /// Rename a branch
    pub fn rename_branch(&self, old_name: &str, new_name: &str) -> Result<()> {
        let mut branch = self.repo.find_branch(old_name, BranchType::Local)
            .with_context(|| format!("Failed to find branch: {}", old_name))?;
        
        branch.rename(new_name, false)
            .with_context(|| format!("Failed to rename branch {} to {}", old_name, new_name))?;
        
        info!("Renamed branch {} to {}", old_name, new_name);
        Ok(())
    }

    /// Get branch upstream tracking information
    pub fn get_upstream(&self, branch_name: &str) -> Result<Option<String>> {
        let branch = self.repo.find_branch(branch_name, BranchType::Local)?;
        
        match branch.upstream() {
            Ok(upstream) => {
                Ok(upstream.name()?.map(|s| s.to_string()))
            }
            Err(_) => Ok(None),
        }
    }

    /// Set branch upstream
    pub fn set_upstream(&self, branch_name: &str, upstream: &str) -> Result<()> {
        let mut branch = self.repo.find_branch(branch_name, BranchType::Local)?;
        branch.set_upstream(Some(upstream))?;
        
        info!("Set upstream for {} to {}", branch_name, upstream);
        Ok(())
    }

    /// Check if branch has unpushed commits
    pub fn has_unpushed_commits(&self, branch_name: &str) -> Result<bool> {
        let branch = self.repo.find_branch(branch_name, BranchType::Local)?;
        
        if let Ok(upstream) = branch.upstream() {
            let local_oid = branch.get().target()
                .ok_or_else(|| anyhow::anyhow!("Branch has no target"))?;
            let upstream_oid = upstream.get().target()
                .ok_or_else(|| anyhow::anyhow!("Upstream has no target"))?;
            
            let ahead = self.repo.graph_ahead_behind(local_oid, upstream_oid)?;
            Ok(ahead.0 > 0)
        } else {
            // No upstream means all commits are unpushed
            Ok(true)
        }
    }
}

/// Branch information
#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub upstream: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn init_test_repo() -> Result<(TempDir, BranchManager)> {
        let dir = TempDir::new()?;
        let repo = Repository::init(&dir)?;
        
        // Create initial commit
        let sig = git2::Signature::now("test", "test@example.com")?;
        let tree_id = {
            let mut index = repo.index()?;
            index.write_tree()?
        };
        let tree = repo.find_tree(tree_id)?;
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Initial commit",
            &tree,
            &[],
        )?;
        
        let manager = BranchManager::new(dir.path())?;
        Ok((dir, manager))
    }

    #[test]
    fn test_create_and_switch_branch() -> Result<()> {
        let (_dir, manager) = init_test_repo()?;
        
        manager.create_and_switch("test-branch")?;
        assert_eq!(manager.current_branch()?, "test-branch");
        
        Ok(())
    }

    #[test]
    fn test_branch_exists() -> Result<()> {
        let (_dir, manager) = init_test_repo()?;
        
        assert!(manager.branch_exists("master")?);
        assert!(!manager.branch_exists("nonexistent")?);
        
        manager.create_and_switch("test-branch")?;
        assert!(manager.branch_exists("test-branch")?);
        
        Ok(())
    }

    #[test]
    fn test_list_branches() -> Result<()> {
        let (_dir, manager) = init_test_repo()?;
        
        manager.create_and_switch("feature-1")?;
        manager.create_and_switch("feature-2")?;
        
        let branches = manager.list_branches()?;
        assert_eq!(branches.len(), 3); // master + 2 features
        
        let current = branches.iter().find(|b| b.is_current).unwrap();
        assert_eq!(current.name, "feature-2");
        
        Ok(())
    }
}