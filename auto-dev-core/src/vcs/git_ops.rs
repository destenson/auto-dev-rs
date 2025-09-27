//! Git operations wrapper using git2

use anyhow::{Context, Result};
use git2::{
    BranchType, Commit, DiffOptions, IndexAddOption, Oid, Repository, Signature, Status,
    StatusOptions,
};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use super::RepoStatus;

/// Wrapper for git operations
pub struct GitOperations {
    repo: Repository,
    repo_path: PathBuf,
}

impl GitOperations {
    /// Create new git operations wrapper
    pub fn new(repo_path: impl AsRef<Path>) -> Result<Self> {
        let repo_path = repo_path.as_ref().to_path_buf();
        let repo = Repository::open(&repo_path).context("Failed to open git repository")?;

        Ok(Self { repo, repo_path })
    }

    /// Get current branch name
    pub fn current_branch(&self) -> Result<String> {
        let head = self.repo.head().context("Failed to get HEAD")?;

        if head.is_branch() {
            let name = head.shorthand().ok_or_else(|| anyhow::anyhow!("Invalid branch name"))?;
            Ok(name.to_string())
        } else {
            Err(anyhow::anyhow!("HEAD is detached"))
        }
    }

    /// Get repository status
    pub fn status(&self) -> Result<RepoStatus> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true).include_ignored(false);

        let statuses =
            self.repo.statuses(Some(&mut opts)).context("Failed to get repository status")?;

        let mut modified_files = Vec::new();
        let mut staged_files = Vec::new();
        let mut untracked_files = Vec::new();
        let mut has_conflicts = false;

        for entry in statuses.iter() {
            let path = PathBuf::from(entry.path().unwrap_or(""));
            let status = entry.status();

            if status.contains(Status::CONFLICTED) {
                has_conflicts = true;
            }

            if status.contains(Status::WT_MODIFIED)
                || status.contains(Status::WT_DELETED)
                || status.contains(Status::WT_RENAMED)
            {
                modified_files.push(path.clone());
            }

            if status.contains(Status::INDEX_NEW)
                || status.contains(Status::INDEX_MODIFIED)
                || status.contains(Status::INDEX_DELETED)
                || status.contains(Status::INDEX_RENAMED)
            {
                staged_files.push(path.clone());
            }

            if status.contains(Status::WT_NEW) {
                untracked_files.push(path);
            }
        }

        let is_clean =
            modified_files.is_empty() && staged_files.is_empty() && untracked_files.is_empty();

        Ok(RepoStatus {
            current_branch: self.current_branch().unwrap_or_else(|_| "detached".to_string()),
            modified_files,
            staged_files,
            untracked_files,
            has_conflicts,
            is_clean,
        })
    }

    /// Stage all changes
    pub fn stage_all(&self) -> Result<()> {
        let mut index = self.repo.index().context("Failed to get repository index")?;

        // Add all files to index
        index
            .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
            .context("Failed to stage files")?;

        // Write index to disk
        index.write().context("Failed to write index")?;

        info!("Staged all changes");
        Ok(())
    }

    /// Stage specific files
    pub fn stage_files(&self, files: &[PathBuf]) -> Result<()> {
        let mut index = self.repo.index().context("Failed to get repository index")?;

        for file in files {
            index.add_path(file).with_context(|| format!("Failed to stage file: {:?}", file))?;
        }

        index.write().context("Failed to write index")?;

        info!("Staged {} files", files.len());
        Ok(())
    }

    /// Create a commit
    pub fn commit(&self, message: &str, sign: bool) -> Result<String> {
        let sig = self.get_signature()?;
        let tree_id = {
            let mut index = self.repo.index().context("Failed to get repository index")?;
            index.write_tree().context("Failed to write tree")?
        };

        let tree = self.repo.find_tree(tree_id).context("Failed to find tree")?;

        let parent_commit = self.get_head_commit()?;

        let commit_id = if sign {
            // TODO: Implement GPG signing
            warn!("GPG signing not yet implemented, creating unsigned commit");
            self.repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent_commit])?
        } else {
            self.repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent_commit])?
        };

        info!("Created commit: {}", commit_id);
        Ok(commit_id.to_string())
    }

    /// Merge a branch
    pub fn merge(&self, branch_name: &str) -> Result<()> {
        let branch = self
            .repo
            .find_branch(branch_name, BranchType::Local)
            .with_context(|| format!("Failed to find branch: {}", branch_name))?;

        let annotated = self.repo.find_annotated_commit(
            branch.get().target().ok_or_else(|| anyhow::anyhow!("Branch has no target"))?,
        )?;

        // Perform merge analysis
        let (analysis, _) = self.repo.merge_analysis(&[&annotated])?;

        if analysis.is_fast_forward() {
            // Fast-forward merge
            self.fast_forward_merge(&annotated)?;
            info!("Fast-forwarded to {}", branch_name);
        } else if analysis.is_normal() {
            // Normal merge
            self.normal_merge(&annotated, branch_name)?;
            info!("Merged branch {}", branch_name);
        } else if analysis.is_up_to_date() {
            info!("Already up to date with {}", branch_name);
        } else {
            anyhow::bail!("Cannot merge branch: {}", branch_name);
        }

        Ok(())
    }

    /// Check if there are uncommitted changes
    pub fn has_changes(&self) -> Result<bool> {
        let status = self.status()?;
        Ok(!status.is_clean)
    }

    /// Get list of commits between two refs
    pub fn get_commits_between(&self, from: &str, to: &str) -> Result<Vec<CommitInfo>> {
        let from_oid = self.resolve_ref(from)?;
        let to_oid = self.resolve_ref(to)?;

        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(to_oid)?;
        revwalk.hide(from_oid)?;

        let mut commits = Vec::new();
        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            commits.push(CommitInfo::from_commit(&commit));
        }

        Ok(commits)
    }

    /// Reset to a specific commit
    pub fn reset_to(&self, commit_ref: &str, hard: bool) -> Result<()> {
        let oid = self.resolve_ref(commit_ref)?;
        let commit = self.repo.find_commit(oid)?;
        let reset_type = if hard { git2::ResetType::Hard } else { git2::ResetType::Mixed };

        self.repo.reset(commit.as_object(), reset_type, None)?;

        info!("Reset to commit: {}", commit_ref);
        Ok(())
    }

    // Helper methods

    fn get_signature(&self) -> Result<Signature> {
        Signature::now("auto-dev-rs", "auto-dev@localhost").context("Failed to create signature")
    }

    fn get_head_commit(&self) -> Result<Commit> {
        let head = self.repo.head().context("Failed to get HEAD")?;
        let commit = head.peel_to_commit().context("Failed to get HEAD commit")?;
        Ok(commit)
    }

    fn resolve_ref(&self, reference: &str) -> Result<Oid> {
        let obj = self
            .repo
            .revparse_single(reference)
            .with_context(|| format!("Failed to resolve reference: {}", reference))?;
        Ok(obj.id())
    }

    fn fast_forward_merge(&self, annotated: &git2::AnnotatedCommit) -> Result<()> {
        let refname = format!("refs/heads/{}", self.current_branch()?);
        let mut reference = self.repo.find_reference(&refname)?;
        reference.set_target(annotated.id(), "fast-forward merge")?;
        self.repo.set_head(&refname)?;
        self.repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        Ok(())
    }

    fn normal_merge(&self, annotated: &git2::AnnotatedCommit, branch_name: &str) -> Result<()> {
        let local = self.get_head_commit()?;
        let remote = self.repo.find_commit(annotated.id())?;

        let local_tree = local.tree()?;
        let remote_tree = remote.tree()?;
        let ancestor =
            self.repo.find_commit(self.repo.merge_base(local.id(), remote.id())?)?.tree()?;

        let mut index = self.repo.merge_trees(&ancestor, &local_tree, &remote_tree, None)?;

        if index.has_conflicts() {
            anyhow::bail!("Merge conflicts detected");
        }

        let tree_id = index.write_tree_to(&self.repo)?;
        let tree = self.repo.find_tree(tree_id)?;
        let sig = self.get_signature()?;
        let message = format!("Merge branch '{}'", branch_name);

        self.repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[&local, &remote])?;

        Ok(())
    }
}

/// Commit information
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub id: String,
    pub author: String,
    pub message: String,
    pub timestamp: i64,
}

impl CommitInfo {
    fn from_commit(commit: &Commit) -> Self {
        Self {
            id: commit.id().to_string(),
            author: commit.author().name().unwrap_or("unknown").to_string(),
            message: commit.message().unwrap_or("").to_string(),
            timestamp: commit.time().seconds(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn init_test_repo() -> Result<(TempDir, GitOperations)> {
        let dir = TempDir::new()?;
        let repo = Repository::init(&dir)?;

        // Create initial commit
        let sig = Signature::now("test", "test@example.com")?;
        let tree_id = {
            let mut index = repo.index()?;
            index.write_tree()?
        };
        let tree = repo.find_tree(tree_id)?;
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;

        let ops = GitOperations::new(dir.path())?;
        Ok((dir, ops))
    }

    #[test]
    fn test_current_branch() -> Result<()> {
        let (_dir, ops) = init_test_repo()?;
        let branch = ops.current_branch()?;
        assert_eq!(branch, "master");
        Ok(())
    }

    #[test]
    fn test_repo_status() -> Result<()> {
        let (_dir, ops) = init_test_repo()?;
        let status = ops.status()?;
        assert!(status.is_clean);
        assert_eq!(status.current_branch, "master");
        Ok(())
    }
}
