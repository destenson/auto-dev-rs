//! Version verification

use anyhow::Result;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tracing::{info, warn};

/// Verifies new versions before deployment
pub struct VersionVerifier {
    timeout: Duration,
}

impl VersionVerifier {
    pub fn new(timeout_secs: u64) -> Self {
        Self { timeout: Duration::from_secs(timeout_secs) }
    }

    /// Verify a new binary version
    pub async fn verify(&self, binary_path: &Path) -> Result<bool> {
        info!("Verifying binary: {:?}", binary_path);

        // Test 1: Check if binary runs
        if !self.test_binary_runs(binary_path).await? {
            warn!("Binary failed to run");
            return Ok(false);
        }

        // Test 2: Check version output
        if !self.test_version_output(binary_path).await? {
            warn!("Binary version check failed");
            return Ok(false);
        }

        // Test 3: Run self-tests
        if !self.run_self_tests(binary_path).await? {
            warn!("Binary self-tests failed");
            return Ok(false);
        }

        info!("All verification tests passed");
        Ok(true)
    }

    async fn test_binary_runs(&self, binary_path: &Path) -> Result<bool> {
        let output = Command::new(binary_path).arg("--help").output()?;

        Ok(output.status.success())
    }

    async fn test_version_output(&self, binary_path: &Path) -> Result<bool> {
        let output = Command::new(binary_path).arg("--version").output()?;

        if !output.status.success() {
            return Ok(false);
        }

        let version_str = String::from_utf8_lossy(&output.stdout);
        Ok(version_str.contains("auto-dev"))
    }

    async fn run_self_tests(&self, binary_path: &Path) -> Result<bool> {
        // Run basic parse command as a smoke test
        let output = Command::new(binary_path).args(&["parse", "--help"]).output()?;

        Ok(output.status.success())
    }
}
