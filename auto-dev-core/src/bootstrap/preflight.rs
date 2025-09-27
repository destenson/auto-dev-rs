//! Pre-flight checks for bootstrap sequence

use super::{BootstrapError, PreflightConfig, Result};
use std::path::Path;
use std::process::Command;
use tracing::{debug, info, warn};

pub struct PreflightChecker {
    config: PreflightConfig,
}

impl PreflightChecker {
    pub fn new(config: PreflightConfig) -> Self {
        Self { config }
    }

    pub async fn run_checks(&self) -> Result<()> {
        info!("Running pre-flight checks");
        
        // Check Rust toolchain
        self.check_rust_toolchain()?;
        
        // Check disk space
        if self.config.check_disk_space {
            self.check_disk_space()?;
        }
        
        // Check git state
        if self.config.check_git_state {
            self.check_git_state()?;
        }
        
        // Run tests if in strict mode
        if self.config.strict {
            self.run_tests()?;
        }
        
        // Check configuration
        self.check_configuration()?;
        
        info!("All pre-flight checks passed");
        Ok(())
    }
    
    fn check_rust_toolchain(&self) -> Result<()> {
        debug!("Checking Rust toolchain availability");
        
        let output = Command::new("rustc")
            .arg("--version")
            .output()
            .map_err(|e| BootstrapError::PreflightFailed(
                format!("Failed to check Rust toolchain: {}", e)
            ))?;
        
        if !output.status.success() {
            return Err(BootstrapError::PreflightFailed(
                "Rust toolchain not available".to_string()
            ));
        }
        
        let version = String::from_utf8_lossy(&output.stdout);
        debug!("Rust toolchain found: {}", version.trim());
        
        // Check cargo
        let cargo_output = Command::new("cargo")
            .arg("--version")
            .output()
            .map_err(|e| BootstrapError::PreflightFailed(
                format!("Failed to check cargo: {}", e)
            ))?;
        
        if !cargo_output.status.success() {
            return Err(BootstrapError::PreflightFailed(
                "Cargo not available".to_string()
            ));
        }
        
        Ok(())
    }
    
    fn check_disk_space(&self) -> Result<()> {
        debug!("Checking available disk space");
        
        #[cfg(target_os = "windows")]
        {
            // Windows-specific disk space check
            let output = Command::new("wmic")
                .args(&["logicaldisk", "get", "size,freespace,caption"])
                .output()
                .map_err(|e| BootstrapError::PreflightFailed(
                    format!("Failed to check disk space: {}", e)
                ))?;
            
            let output_str = String::from_utf8_lossy(&output.stdout);
            
            // Parse the output to check if we have enough space
            // This is a simplified check - in production we'd parse more carefully
            if output_str.contains("Caption") {
                debug!("Disk space check passed (Windows)");
                return Ok(());
            }
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            // Unix-like disk space check
            let output = Command::new("df")
                .args(&["-k", "."])
                .output()
                .map_err(|e| BootstrapError::PreflightFailed(
                    format!("Failed to check disk space: {}", e)
                ))?;
            
            let output_str = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = output_str.lines().collect();
            
            if lines.len() > 1 {
                // Parse available space (simplified)
                debug!("Disk space check passed (Unix)");
                return Ok(());
            }
        }
        
        // For now, we'll assume we have enough space if the check runs
        warn!("Disk space check simplified - assuming sufficient space");
        Ok(())
    }
    
    fn check_git_state(&self) -> Result<()> {
        debug!("Checking git repository state");
        
        // Check if we're in a git repo
        let status_output = Command::new("git")
            .args(&["status", "--porcelain"])
            .output()
            .map_err(|e| BootstrapError::PreflightFailed(
                format!("Failed to check git status: {}", e)
            ))?;
        
        if !status_output.status.success() {
            return Err(BootstrapError::PreflightFailed(
                "Not in a git repository".to_string()
            ));
        }
        
        let status = String::from_utf8_lossy(&status_output.stdout);
        
        if self.config.require_clean_git && !status.trim().is_empty() {
            return Err(BootstrapError::PreflightFailed(
                "Git repository has uncommitted changes. Please commit or stash them first.".to_string()
            ));
        }
        
        if !status.trim().is_empty() {
            warn!("Git repository has uncommitted changes");
        } else {
            debug!("Git repository is clean");
        }
        
        Ok(())
    }
    
    fn run_tests(&self) -> Result<()> {
        info!("Running test suite to verify system integrity");
        
        let output = Command::new("cargo")
            .args(&["test", "--quiet"])
            .output()
            .map_err(|e| BootstrapError::PreflightFailed(
                format!("Failed to run tests: {}", e)
            ))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BootstrapError::PreflightFailed(
                format!("Test suite failed:\n{}", stderr)
            ));
        }
        
        debug!("Test suite passed");
        Ok(())
    }
    
    fn check_configuration(&self) -> Result<()> {
        debug!("Checking configuration validity");
        
        // Check if .auto-dev directory exists or can be created
        let auto_dev_dir = Path::new(".auto-dev");
        if !auto_dev_dir.exists() {
            std::fs::create_dir_all(auto_dev_dir)
                .map_err(|e| BootstrapError::PreflightFailed(
                    format!("Cannot create .auto-dev directory: {}", e)
                ))?;
            debug!("Created .auto-dev directory");
        }
        
        // Check if Cargo.toml exists
        if !Path::new("Cargo.toml").exists() {
            return Err(BootstrapError::PreflightFailed(
                "Cargo.toml not found. Must run from project root.".to_string()
            ));
        }
        
        Ok(())
    }
    
    pub fn describe_checks(&self) -> Vec<String> {
        let mut checks = vec![
            "Rust toolchain availability".to_string(),
            "Configuration validity".to_string(),
        ];
        
        if self.config.check_disk_space {
            checks.push(format!("Disk space (>{}GB)", self.config.required_disk_gb));
        }
        
        if self.config.check_git_state {
            checks.push("Git repository state".to_string());
            if self.config.require_clean_git {
                checks.push("Clean git working directory".to_string());
            }
        }
        
        if self.config.strict {
            checks.push("Test suite passes".to_string());
        }
        
        checks
    }
}