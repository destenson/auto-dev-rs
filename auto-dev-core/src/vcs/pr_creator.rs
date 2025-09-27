//! Pull request creation for GitHub/GitLab

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info, warn};

use super::PullRequestInfo;

/// Creates pull requests via CLI tools
pub struct PullRequestCreator {
    repo_path: PathBuf,
    provider: GitProvider,
}

/// Git hosting provider
#[derive(Debug, Clone)]
enum GitProvider {
    GitHub,
    GitLab,
    Unknown,
}

impl PullRequestCreator {
    /// Create new PR creator
    pub fn new(repo_path: impl AsRef<Path>) -> Result<Self> {
        let repo_path = repo_path.as_ref().to_path_buf();
        let provider = Self::detect_provider(&repo_path)?;

        Ok(Self { repo_path, provider })
    }

    /// Detect git provider from remote URL
    fn detect_provider(repo_path: &Path) -> Result<GitProvider> {
        let output = Command::new("git")
            .args(&["remote", "get-url", "origin"])
            .current_dir(repo_path)
            .output()
            .context("Failed to get remote URL")?;

        let url = String::from_utf8_lossy(&output.stdout);

        if url.contains("github.com") {
            Ok(GitProvider::GitHub)
        } else if url.contains("gitlab.com") || url.contains("gitlab") {
            Ok(GitProvider::GitLab)
        } else {
            warn!("Unknown git provider: {}", url);
            Ok(GitProvider::Unknown)
        }
    }

    /// Create a pull request
    pub async fn create(&self, title: &str, description: &str) -> Result<PullRequestInfo> {
        match self.provider {
            GitProvider::GitHub => self.create_github_pr(title, description).await,
            GitProvider::GitLab => self.create_gitlab_mr(title, description).await,
            GitProvider::Unknown => {
                anyhow::bail!("Cannot create PR: unknown git provider")
            }
        }
    }

    /// Create GitHub pull request using gh CLI
    async fn create_github_pr(&self, title: &str, description: &str) -> Result<PullRequestInfo> {
        // Check if gh CLI is available
        if !self.is_gh_available()? {
            return self.fallback_pr_instructions(title, description);
        }

        // Get current branch
        let current_branch = self.get_current_branch()?;

        // Push current branch to origin
        self.push_branch(&current_branch)?;

        // Create PR using gh CLI
        let output = Command::new("gh")
            .args(&[
                "pr",
                "create",
                "--title",
                title,
                "--body",
                description,
                "--head",
                &current_branch,
            ])
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to create GitHub PR")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to create PR: {}", error);
        }

        // Parse PR URL from output
        let output_str = String::from_utf8_lossy(&output.stdout);
        let pr_url = output_str.trim().to_string();

        // Extract PR number from URL
        let pr_number = self.extract_pr_number(&pr_url)?;

        info!("Created GitHub PR #{}: {}", pr_number, pr_url);

        Ok(PullRequestInfo {
            number: pr_number,
            url: pr_url,
            branch: current_branch,
            base: "main".to_string(), // TODO: detect default branch
        })
    }

    /// Create GitLab merge request using glab CLI
    async fn create_gitlab_mr(&self, title: &str, description: &str) -> Result<PullRequestInfo> {
        // Check if glab CLI is available
        if !self.is_glab_available()? {
            return self.fallback_pr_instructions(title, description);
        }

        // Get current branch
        let current_branch = self.get_current_branch()?;

        // Push current branch to origin
        self.push_branch(&current_branch)?;

        // Create MR using glab CLI
        let output = Command::new("glab")
            .args(&[
                "mr",
                "create",
                "--title",
                title,
                "--description",
                description,
                "--source-branch",
                &current_branch,
            ])
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to create GitLab MR")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to create MR: {}", error);
        }

        // Parse MR URL from output
        let output_str = String::from_utf8_lossy(&output.stdout);
        let mr_url = output_str.trim().to_string();

        // Extract MR number from URL
        let mr_number = self.extract_pr_number(&mr_url)?;

        info!("Created GitLab MR !{}: {}", mr_number, mr_url);

        Ok(PullRequestInfo {
            number: mr_number,
            url: mr_url,
            branch: current_branch,
            base: "main".to_string(), // TODO: detect default branch
        })
    }

    /// Check if gh CLI is available
    fn is_gh_available(&self) -> Result<bool> {
        let output = Command::new("gh").arg("--version").output();

        Ok(output.is_ok() && output.unwrap().status.success())
    }

    /// Check if glab CLI is available
    fn is_glab_available(&self) -> Result<bool> {
        let output = Command::new("glab").arg("--version").output();

        Ok(output.is_ok() && output.unwrap().status.success())
    }

    /// Get current branch name
    fn get_current_branch(&self) -> Result<String> {
        let output = Command::new("git")
            .args(&["branch", "--show-current"])
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to get current branch")?;

        if !output.status.success() {
            anyhow::bail!("Failed to get current branch");
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Push branch to origin
    fn push_branch(&self, branch: &str) -> Result<()> {
        info!("Pushing branch {} to origin", branch);

        let output = Command::new("git")
            .args(&["push", "-u", "origin", branch])
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to push branch")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to push branch: {}", error);
        }

        Ok(())
    }

    /// Extract PR/MR number from URL
    fn extract_pr_number(&self, url: &str) -> Result<u32> {
        // GitHub: https://github.com/owner/repo/pull/123
        // GitLab: https://gitlab.com/owner/repo/-/merge_requests/123

        let parts: Vec<&str> = url.split('/').collect();

        if let Some(last) = parts.last() {
            if let Ok(number) = last.parse::<u32>() {
                return Ok(number);
            }
        }

        // Fallback: look for number in URL
        for part in parts {
            if let Ok(number) = part.parse::<u32>() {
                return Ok(number);
            }
        }

        anyhow::bail!("Could not extract PR number from URL: {}", url)
    }

    /// Provide fallback instructions when CLI tools aren't available
    fn fallback_pr_instructions(&self, title: &str, description: &str) -> Result<PullRequestInfo> {
        let current_branch = self.get_current_branch()?;

        println!("\n=== Manual Pull Request Creation ===");
        println!("CLI tools (gh/glab) not available.");
        println!("\nTo create a pull request manually:");
        println!("1. Push your branch: git push -u origin {}", current_branch);
        println!("2. Visit your repository's web interface");
        println!("3. Create a new pull request with:");
        println!("   Title: {}", title);
        println!("   Description: {}", description);
        println!("   Source: {}", current_branch);
        println!("   Target: main");
        println!("=====================================\n");

        Ok(PullRequestInfo {
            number: 0,
            url: "manual".to_string(),
            branch: current_branch,
            base: "main".to_string(),
        })
    }

    /// List open PRs
    pub async fn list_open(&self) -> Result<Vec<PullRequestInfo>> {
        match self.provider {
            GitProvider::GitHub => self.list_github_prs().await,
            GitProvider::GitLab => self.list_gitlab_mrs().await,
            GitProvider::Unknown => Ok(Vec::new()),
        }
    }

    /// List GitHub PRs
    async fn list_github_prs(&self) -> Result<Vec<PullRequestInfo>> {
        if !self.is_gh_available()? {
            return Ok(Vec::new());
        }

        let output = Command::new("gh")
            .args(&["pr", "list", "--json", "number,url,headRefName,baseRefName"])
            .current_dir(&self.repo_path)
            .output()?;

        if output.status.success() {
            let json = String::from_utf8_lossy(&output.stdout);
            // Parse JSON manually or use serde_json
            // For now, return empty
            Ok(Vec::new())
        } else {
            Ok(Vec::new())
        }
    }

    /// List GitLab MRs
    async fn list_gitlab_mrs(&self) -> Result<Vec<PullRequestInfo>> {
        if !self.is_glab_available()? {
            return Ok(Vec::new());
        }

        let output =
            Command::new("glab").args(&["mr", "list"]).current_dir(&self.repo_path).output()?;

        if output.status.success() {
            // Parse output
            // For now, return empty
            Ok(Vec::new())
        } else {
            Ok(Vec::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_extract_pr_number() {
        let dir = TempDir::new().unwrap();
        let creator = PullRequestCreator {
            repo_path: dir.path().to_path_buf(),
            provider: GitProvider::GitHub,
        };

        // GitHub URL
        let url = "https://github.com/owner/repo/pull/123";
        assert_eq!(creator.extract_pr_number(url).unwrap(), 123);

        // GitLab URL
        let url = "https://gitlab.com/owner/repo/-/merge_requests/456";
        assert_eq!(creator.extract_pr_number(url).unwrap(), 456);
    }
}
