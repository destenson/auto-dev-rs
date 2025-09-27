#![allow(unused)]
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;
use tempfile::TempDir;
use tokio::fs;
use tokio::process::Command;
use tracing::{debug, info, warn};

use super::SelfTestError;

/// Isolated test environment for safe testing
pub struct TestSandbox {
    config: SandboxConfig,
    temp_dir: TempDir,
    project_snapshot: Option<ProjectSnapshot>,
}

impl TestSandbox {
    pub async fn new(config: SandboxConfig) -> Result<Self, SelfTestError> {
        let temp_dir = TempDir::new()
            .map_err(|e| SelfTestError::Sandbox(format!("Failed to create temp dir: {}", e)))?;

        info!("Created test sandbox at: {:?}", temp_dir.path());

        Ok(Self { config, temp_dir, project_snapshot: None })
    }

    /// Copy project to sandbox for testing
    pub async fn setup_project(&mut self, source_path: &Path) -> Result<(), SelfTestError> {
        info!("Setting up project in sandbox from: {:?}", source_path);

        // Take snapshot for potential restoration
        self.project_snapshot = Some(ProjectSnapshot::capture(source_path).await?);

        // Copy project files to sandbox
        self.copy_directory(source_path, self.temp_dir.path()).await?;

        // Remove target directory to ensure clean build
        let target_dir = self.temp_dir.path().join("target");
        if target_dir.exists() {
            fs::remove_dir_all(&target_dir).await.map_err(|e| {
                SelfTestError::Sandbox(format!("Failed to remove target dir: {}", e))
            })?;
        }

        Ok(())
    }

    /// Apply modifications to sandboxed project
    pub async fn apply_modifications(
        &mut self,
        modifications: Vec<FileModification>,
    ) -> Result<(), SelfTestError> {
        for modification in modifications {
            let file_path = self.temp_dir.path().join(&modification.path);

            match modification.operation {
                ModificationOp::Create | ModificationOp::Update => {
                    if let Some(parent) = file_path.parent() {
                        fs::create_dir_all(parent).await.map_err(|e| {
                            SelfTestError::Sandbox(format!("Failed to create dir: {}", e))
                        })?;
                    }

                    fs::write(&file_path, &modification.content).await.map_err(|e| {
                        SelfTestError::Sandbox(format!("Failed to write file: {}", e))
                    })?;

                    debug!("Applied modification to: {:?}", file_path);
                }
                ModificationOp::Delete => {
                    if file_path.exists() {
                        fs::remove_file(&file_path).await.map_err(|e| {
                            SelfTestError::Sandbox(format!("Failed to delete file: {}", e))
                        })?;
                        debug!("Deleted file: {:?}", file_path);
                    }
                }
            }
        }

        Ok(())
    }

    /// Run a command in the sandbox environment
    pub async fn run_command(
        &self,
        program: &str,
        args: &[&str],
    ) -> Result<CommandResult, SelfTestError> {
        let start = Instant::now();

        debug!("Running command in sandbox: {} {:?}", program, args);

        let output = Command::new(program)
            .args(args)
            .current_dir(self.temp_dir.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| SelfTestError::Execution(format!("Command failed: {}", e)))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(CommandResult {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout,
            stderr,
            output: format!(
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
            duration_ms,
        })
    }

    /// Get path to a file in the sandbox
    pub fn get_path(&self, relative_path: &Path) -> PathBuf {
        self.temp_dir.path().join(relative_path)
    }

    /// Check if a file exists in the sandbox
    pub async fn file_exists(&self, relative_path: &Path) -> bool {
        self.get_path(relative_path).exists()
    }

    /// Read a file from the sandbox
    pub async fn read_file(&self, relative_path: &Path) -> Result<String, SelfTestError> {
        let path = self.get_path(relative_path);
        fs::read_to_string(&path)
            .await
            .map_err(|e| SelfTestError::Sandbox(format!("Failed to read file: {}", e)))
    }

    /// Write a file to the sandbox
    pub async fn write_file(
        &self,
        relative_path: &Path,
        content: &str,
    ) -> Result<(), SelfTestError> {
        let path = self.get_path(relative_path);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| SelfTestError::Sandbox(format!("Failed to create dir: {}", e)))?;
        }

        fs::write(&path, content)
            .await
            .map_err(|e| SelfTestError::Sandbox(format!("Failed to write file: {}", e)))
    }

    /// Copy directory recursively
    async fn copy_directory(&self, from: &Path, to: &Path) -> Result<(), SelfTestError> {
        debug!("Copying directory from {:?} to {:?}", from, to);

        fs::create_dir_all(to)
            .await
            .map_err(|e| SelfTestError::Sandbox(format!("Failed to create dir: {}", e)))?;

        let mut entries = fs::read_dir(from)
            .await
            .map_err(|e| SelfTestError::Sandbox(format!("Failed to read dir: {}", e)))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| SelfTestError::Sandbox(format!("Failed to read entry: {}", e)))?
        {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            // Skip build artifacts and version control
            if file_name_str == "target"
                || file_name_str == ".git"
                || file_name_str == "node_modules"
            {
                continue;
            }

            let from_path = entry.path();
            let to_path = to.join(&file_name);

            let metadata = entry
                .metadata()
                .await
                .map_err(|e| SelfTestError::Sandbox(format!("Failed to get metadata: {}", e)))?;

            if metadata.is_dir() {
                Box::pin(self.copy_directory(&from_path, &to_path)).await?;
            } else {
                fs::copy(&from_path, &to_path)
                    .await
                    .map_err(|e| SelfTestError::Sandbox(format!("Failed to copy file: {}", e)))?;
            }
        }

        Ok(())
    }

    /// Clean up sandbox (happens automatically on drop)
    pub async fn cleanup(&mut self) -> Result<(), SelfTestError> {
        info!("Cleaning up sandbox at: {:?}", self.temp_dir.path());
        // TempDir automatically cleans up on drop
        Ok(())
    }

    /// Restore from snapshot
    pub async fn restore_snapshot(&mut self) -> Result<(), SelfTestError> {
        if let Some(snapshot) = &self.project_snapshot {
            snapshot.restore(self.temp_dir.path()).await?;
            info!("Restored sandbox from snapshot");
        }
        Ok(())
    }
}

/// Configuration for the test sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub isolation_level: IsolationLevel,
    pub resource_limits: ResourceLimits,
    pub network_access: bool,
    pub filesystem_access: Vec<PathBuf>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            isolation_level: IsolationLevel::Process,
            resource_limits: ResourceLimits::default(),
            network_access: false,
            filesystem_access: vec![],
        }
    }
}

/// Level of isolation for the sandbox
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// Basic process isolation
    Process,
    /// Container-based isolation
    Container,
    /// Full VM isolation
    VirtualMachine,
}

/// Resource limits for sandboxed execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory_mb: usize,
    pub cpu_cores: usize,
    pub disk_mb: usize,
    pub timeout_seconds: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self { memory_mb: 2048, cpu_cores: 2, disk_mb: 1024, timeout_seconds: 300 }
    }
}

/// Result from running a command in the sandbox
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub output: String,
    pub duration_ms: u64,
}

/// File modification to apply in sandbox
#[derive(Debug, Clone)]
pub struct FileModification {
    pub path: PathBuf,
    pub operation: ModificationOp,
    pub content: String,
}

/// Type of file modification
#[derive(Debug, Clone, Copy)]
pub enum ModificationOp {
    Create,
    Update,
    Delete,
}

/// Snapshot of project state for restoration
struct ProjectSnapshot {
    files: Vec<(PathBuf, Vec<u8>)>,
    timestamp: std::time::SystemTime,
}

impl ProjectSnapshot {
    async fn capture(path: &Path) -> Result<Self, SelfTestError> {
        let mut files = Vec::new();
        Self::capture_recursive(path, path, &mut files).await?;

        Ok(Self { files, timestamp: std::time::SystemTime::now() })
    }

    async fn capture_recursive(
        base: &Path,
        current: &Path,
        files: &mut Vec<(PathBuf, Vec<u8>)>,
    ) -> Result<(), SelfTestError> {
        let mut entries = fs::read_dir(current)
            .await
            .map_err(|e| SelfTestError::Sandbox(format!("Failed to read dir: {}", e)))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| SelfTestError::Sandbox(format!("Failed to read entry: {}", e)))?
        {
            let path = entry.path();
            let relative_path = path
                .strip_prefix(base)
                .map_err(|e| SelfTestError::Sandbox(format!("Failed to strip prefix: {}", e)))?
                .to_path_buf();

            let metadata = entry
                .metadata()
                .await
                .map_err(|e| SelfTestError::Sandbox(format!("Failed to get metadata: {}", e)))?;

            if metadata.is_dir() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name != "target" && file_name != ".git" {
                    Box::pin(Self::capture_recursive(base, &path, files)).await?;
                }
            } else {
                let content = fs::read(&path)
                    .await
                    .map_err(|e| SelfTestError::Sandbox(format!("Failed to read file: {}", e)))?;
                files.push((relative_path, content));
            }
        }

        Ok(())
    }

    async fn restore(&self, target: &Path) -> Result<(), SelfTestError> {
        for (relative_path, content) in &self.files {
            let full_path = target.join(relative_path);

            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| SelfTestError::Sandbox(format!("Failed to create dir: {}", e)))?;
            }

            fs::write(&full_path, content)
                .await
                .map_err(|e| SelfTestError::Sandbox(format!("Failed to write file: {}", e)))?;
        }

        Ok(())
    }
}
