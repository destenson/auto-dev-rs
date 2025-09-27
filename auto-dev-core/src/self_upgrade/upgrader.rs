#![allow(unused)]
//! Main upgrade orchestrator

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{error, info, warn};

use super::platform::BinarySwapper;
use super::{BuildProfile, RollbackManager, StatePreserver, UpgradeConfig, VersionVerifier};

/// Main upgrade orchestrator
pub struct SelfUpgrader {
    config: UpgradeConfig,
    verifier: VersionVerifier,
    state_preserver: StatePreserver,
    rollback_manager: RollbackManager,
    binary_swapper: BinarySwapper,
}

impl SelfUpgrader {
    /// Create a new upgrader
    pub fn new(config: UpgradeConfig) -> Self {
        let staging_dir = config.staging_dir.clone();
        let binary_path = config.binary_path.clone();

        Self {
            verifier: VersionVerifier::new(config.verification_timeout),
            state_preserver: StatePreserver::new(staging_dir.clone()),
            rollback_manager: RollbackManager::new(binary_path.clone(), config.keep_versions),
            binary_swapper: BinarySwapper::new(binary_path),
            config,
        }
    }

    /// Execute the upgrade process
    pub async fn execute(&self) -> Result<()> {
        info!("Starting self-upgrade process");

        // Step 1: Save current state
        info!("Preserving current state");
        let state = self.state_preserver.save_state().await?;

        // Step 2: Compile new version
        info!("Compiling new version");
        let new_binary = self.compile_new_version().await?;

        // Step 3: Verify new version
        info!("Verifying new version");
        if !self.verifier.verify(&new_binary).await? {
            error!("Verification failed for new version");
            return Err(anyhow::anyhow!("New version failed verification"));
        }

        // Step 4: Create backup of current version
        info!("Creating backup of current version");
        self.rollback_manager.create_backup().await?;

        if self.config.dry_run {
            info!("DRY-RUN: Would replace binary with new version");
            return Ok(());
        }

        // Step 5: Replace binary
        info!("Replacing binary with new version");
        match self.binary_swapper.swap(&new_binary).await {
            Ok(_) => {
                info!("Binary replacement successful");

                // Step 6: Restart with preserved state
                info!("Restarting with preserved state");
                self.restart_with_state(state).await?;

                Ok(())
            }
            Err(e) => {
                error!("Binary replacement failed: {}", e);
                warn!("Attempting rollback");
                self.rollback_manager.rollback().await?;
                Err(e)
            }
        }
    }

    /// Compile the modified version
    async fn compile_new_version(&self) -> Result<PathBuf> {
        info!(
            "Building {} version",
            match &self.config.build_profile {
                BuildProfile::Debug => "debug",
                BuildProfile::Release => "release",
                BuildProfile::Custom(name) => name.as_str(),
            }
        );

        let output = Command::new("cargo")
            .args(&self.config.build_profile.cargo_args())
            .output()
            .context("Failed to run cargo build")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Build failed: {}", stderr));
        }

        // Find the built binary
        let binary_name = if cfg!(windows) { "auto-dev.exe" } else { "auto-dev" };

        let new_binary = self.config.build_profile.target_dir().join(binary_name);

        if !new_binary.exists() {
            return Err(anyhow::anyhow!("Built binary not found at {:?}", new_binary));
        }

        // Copy to staging directory
        std::fs::create_dir_all(&self.config.staging_dir)?;
        let staged_binary = self.config.staging_dir.join(binary_name);
        std::fs::copy(&new_binary, &staged_binary)?;

        Ok(staged_binary)
    }

    /// Restart the application with preserved state
    async fn restart_with_state(&self, state: serde_json::Value) -> Result<()> {
        // Save state to file for new process to read
        let state_file = self.config.staging_dir.join("upgrade_state.json");
        std::fs::write(&state_file, serde_json::to_string_pretty(&state)?)?;

        // Platform-specific restart
        self.binary_swapper
            .restart_with_args(&["--restore-state", state_file.to_str().unwrap()])?;

        Ok(())
    }
}
