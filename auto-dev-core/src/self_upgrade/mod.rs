//! Self-upgrade and restart mechanism for auto-dev-rs

pub mod platform;
pub mod rollback;
pub mod state_preserver;
pub mod upgrader;
pub mod verifier;

use anyhow::Result;
use std::path::PathBuf;

pub use rollback::RollbackManager;
pub use state_preserver::StatePreserver;
pub use upgrader::SelfUpgrader;
pub use verifier::VersionVerifier;

/// Configuration for self-upgrade
#[derive(Debug, Clone)]
pub struct UpgradeConfig {
    /// Path to the current binary
    pub binary_path: PathBuf,

    /// Path to staging directory for new version
    pub staging_dir: PathBuf,

    /// Enable dry-run mode
    pub dry_run: bool,

    /// Timeout for verification tests (seconds)
    pub verification_timeout: u64,

    /// Keep N previous versions for rollback
    pub keep_versions: usize,
}

impl Default for UpgradeConfig {
    fn default() -> Self {
        Self {
            binary_path: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("auto-dev")),
            staging_dir: PathBuf::from(".auto-dev/staging"),
            dry_run: false,
            verification_timeout: 60,
            keep_versions: 3,
        }
    }
}

/// Perform a self-upgrade
pub async fn upgrade(config: UpgradeConfig) -> Result<()> {
    let upgrader = SelfUpgrader::new(config);
    upgrader.execute().await
}
