//! Git bisect support for finding problematic commits

use anyhow::{Context, Result};
use git2::{Oid, Repository};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{info, warn};

/// Git bisect for automated debugging
pub struct BisectManager {
    repo: Repository,
    repo_path: PathBuf,
}

impl BisectManager {
    /// Create new bisect manager
    pub fn new(repo_path: impl AsRef<Path>) -> Result<Self> {
        let repo_path = repo_path.as_ref().to_path_buf();
        let repo = Repository::open(&repo_path).context("Failed to open git repository")?;

        Ok(Self { repo, repo_path })
    }

    /// Start a bisect session
    pub fn start(&self, good_commit: &str, bad_commit: &str) -> Result<BisectSession> {
        // Start bisect
        Command::new("git")
            .args(&["bisect", "start"])
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to start bisect")?;

        // Mark bad commit
        Command::new("git")
            .args(&["bisect", "bad", bad_commit])
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to mark bad commit")?;

        // Mark good commit
        Command::new("git")
            .args(&["bisect", "good", good_commit])
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to mark good commit")?;

        info!("Started bisect between {} (good) and {} (bad)", good_commit, bad_commit);

        Ok(BisectSession {
            repo_path: self.repo_path.clone(),
            good_commit: good_commit.to_string(),
            bad_commit: bad_commit.to_string(),
            steps_remaining: self.estimate_steps(good_commit, bad_commit)?,
        })
    }

    /// Run automated bisect with a test command
    pub async fn run_automated(
        &self,
        good_commit: &str,
        bad_commit: &str,
        test_command: &str,
    ) -> Result<String> {
        info!("Running automated bisect with test: {}", test_command);

        // Start bisect
        self.start(good_commit, bad_commit)?;

        // Run bisect with automated test
        let output = Command::new("git")
            .args(&["bisect", "run", "sh", "-c", test_command])
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to run automated bisect")?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        // Parse the result to find the first bad commit
        let bad_commit = self.parse_bisect_result(&output_str)?;

        // Clean up
        self.reset()?;

        Ok(bad_commit)
    }

    /// Estimate number of steps in bisect
    fn estimate_steps(&self, good: &str, bad: &str) -> Result<usize> {
        let good_oid = self.resolve_ref(good)?;
        let bad_oid = self.resolve_ref(bad)?;

        let mut count = 0;
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(bad_oid)?;
        revwalk.hide(good_oid)?;

        for _ in revwalk {
            count += 1;
        }

        // Bisect steps is approximately log2 of commit count
        Ok((count as f64).log2().ceil() as usize)
    }

    /// Reset bisect session
    pub fn reset(&self) -> Result<()> {
        Command::new("git")
            .args(&["bisect", "reset"])
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to reset bisect")?;

        info!("Bisect session reset");
        Ok(())
    }

    /// Parse bisect result to find bad commit
    fn parse_bisect_result(&self, output: &str) -> Result<String> {
        // Look for the line that identifies the first bad commit
        for line in output.lines() {
            if line.contains("is the first bad commit") {
                // Extract commit hash from line
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(hash) = parts.first() {
                    return Ok(hash.to_string());
                }
            }
        }

        anyhow::bail!("Could not find bad commit in bisect output")
    }

    fn resolve_ref(&self, reference: &str) -> Result<Oid> {
        let obj = self.repo.revparse_single(reference)?;
        Ok(obj.id())
    }
}

/// Active bisect session
pub struct BisectSession {
    repo_path: PathBuf,
    good_commit: String,
    bad_commit: String,
    steps_remaining: usize,
}

impl BisectSession {
    /// Mark current commit as good
    pub fn mark_good(&mut self) -> Result<()> {
        Command::new("git")
            .args(&["bisect", "good"])
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to mark as good")?;

        if self.steps_remaining > 0 {
            self.steps_remaining -= 1;
        }

        info!("Marked current commit as good, {} steps remaining", self.steps_remaining);
        Ok(())
    }

    /// Mark current commit as bad
    pub fn mark_bad(&mut self) -> Result<()> {
        Command::new("git")
            .args(&["bisect", "bad"])
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to mark as bad")?;

        if self.steps_remaining > 0 {
            self.steps_remaining -= 1;
        }

        info!("Marked current commit as bad, {} steps remaining", self.steps_remaining);
        Ok(())
    }

    /// Skip current commit
    pub fn skip(&mut self) -> Result<()> {
        Command::new("git")
            .args(&["bisect", "skip"])
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to skip commit")?;

        info!("Skipped current commit");
        Ok(())
    }

    /// Get current commit being tested
    pub fn current_commit(&self) -> Result<String> {
        let output = Command::new("git")
            .args(&["rev-parse", "HEAD"])
            .current_dir(&self.repo_path)
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Check if bisect is complete
    pub fn is_complete(&self) -> bool {
        self.steps_remaining == 0
    }

    /// Get bisect log
    pub fn get_log(&self) -> Result<String> {
        let output =
            Command::new("git").args(&["bisect", "log"]).current_dir(&self.repo_path).output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

/// Bisect helper for common debugging scenarios
pub struct BisectHelper;

impl BisectHelper {
    /// Find when a test started failing
    pub async fn find_test_failure(
        repo_path: &Path,
        test_command: &str,
        last_known_good: Option<&str>,
    ) -> Result<String> {
        let manager = BisectManager::new(repo_path)?;

        // Use HEAD as bad and find a good commit
        let good_commit = if let Some(good) = last_known_good {
            good.to_string()
        } else {
            // Try to find a good commit by going back in history
            Self::find_good_commit(repo_path, test_command).await?
        };

        manager.run_automated(&good_commit, "HEAD", test_command).await
    }

    /// Find when a file was introduced or modified
    pub async fn find_file_introduction(repo_path: &Path, file_path: &str) -> Result<String> {
        let test_command = format!("test -f {}", file_path);
        let manager = BisectManager::new(repo_path)?;

        // Find first commit in history as potential good
        let first_commit = Self::find_first_commit(repo_path)?;

        manager.run_automated(&first_commit, "HEAD", &test_command).await
    }

    /// Find when performance regressed
    pub async fn find_performance_regression(
        repo_path: &Path,
        benchmark_command: &str,
        threshold: f64,
    ) -> Result<String> {
        let test_command = format!(
            "{} | awk '{{if ($1 > {}) exit 1; else exit 0}}'",
            benchmark_command, threshold
        );

        let manager = BisectManager::new(repo_path)?;
        let good_commit = Self::find_good_commit(repo_path, &test_command).await?;

        manager.run_automated(&good_commit, "HEAD", &test_command).await
    }

    /// Find a good commit by testing backwards
    async fn find_good_commit(repo_path: &Path, test_command: &str) -> Result<String> {
        let commits = vec!["HEAD~10", "HEAD~50", "HEAD~100", "HEAD~500"];

        for commit_ref in commits {
            // Try to checkout and test
            let checkout =
                Command::new("git").args(&["checkout", commit_ref]).current_dir(repo_path).output();

            if checkout.is_err() {
                continue;
            }

            let test =
                Command::new("sh").args(&["-c", test_command]).current_dir(repo_path).output()?;

            if test.status.success() {
                let output = Command::new("git")
                    .args(&["rev-parse", "HEAD"])
                    .current_dir(repo_path)
                    .output()?;

                // Return to original branch
                Command::new("git").args(&["checkout", "-"]).current_dir(repo_path).output()?;

                return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
        }

        anyhow::bail!("Could not find a good commit in recent history")
    }

    /// Find first commit in repository
    fn find_first_commit(repo_path: &Path) -> Result<String> {
        let output = Command::new("git")
            .args(&["rev-list", "--max-parents=0", "HEAD"])
            .current_dir(repo_path)
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).lines().next().unwrap_or("HEAD").to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_repo() -> Result<(TempDir, BisectManager)> {
        let dir = TempDir::new()?;
        let repo = Repository::init(&dir)?;

        // Create some commits
        let sig = git2::Signature::now("test", "test@example.com")?;
        let tree_id = {
            let mut index = repo.index()?;
            index.write_tree()?
        };
        let tree = repo.find_tree(tree_id)?;

        // Initial commit
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;

        let manager = BisectManager::new(dir.path())?;
        Ok((dir, manager))
    }

    #[test]
    fn test_bisect_creation() -> Result<()> {
        let (_dir, _manager) = create_test_repo()?;
        Ok(())
    }
}
