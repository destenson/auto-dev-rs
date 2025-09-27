#![allow(unused)]
//! Self-upgrade and restart mechanism for auto-dev-rs

pub mod platform;
pub mod rollback;
pub mod state_preserver;
pub mod upgrader;
pub mod verifier;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use rollback::RollbackManager;
pub use state_preserver::StatePreserver;
pub use upgrader::SelfUpgrader;
pub use verifier::VersionVerifier;

/// Build profile for compilation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildProfile {
    /// Debug build - fast compilation, debug info, no optimizations
    Debug,
    /// Release build - optimized for production
    Release,
    /// Custom profile (e.g., "dev", "test", "bench", or user-defined)
    Custom(String),
}

impl Default for BuildProfile {
    fn default() -> Self {
        // Default to debug for development safety
        BuildProfile::Debug
    }
}

impl BuildProfile {
    /// Get cargo arguments for this profile
    pub fn cargo_args(&self) -> Vec<String> {
        match self {
            BuildProfile::Debug => vec!["build".to_string()],
            BuildProfile::Release => vec!["build".to_string(), "--release".to_string()],
            BuildProfile::Custom(profile) => {
                vec!["build".to_string(), "--profile".to_string(), profile.clone()]
            }
        }
    }
    
    /// Get the target directory for this profile
    pub fn target_dir(&self) -> PathBuf {
        // First try compile-time CARGO_TARGET_DIR
        let base_target = if let Some(compile_time_target) = option_env!("CARGO_TARGET_DIR") {
            compile_time_target.to_string()
        } else {
            // Fall back to runtime environment variable
            std::env::var("CARGO_TARGET_DIR")
                .unwrap_or_else(|_| "target".to_string())
        };
        
        let profile_dir = match self {
            BuildProfile::Debug => "debug",
            BuildProfile::Release => "release",
            BuildProfile::Custom(name) => name.as_str(),
        };
        
        PathBuf::from(base_target).join(profile_dir)
    }
    
    /// Get compile-time target directory information
    pub fn compile_time_target_dir() -> PathBuf {
        // CARGO_TARGET_DIR is set at build time if specified
        // Otherwise we need to use OUT_DIR to derive it
        if let Some(out_dir) = option_env!("OUT_DIR") {
            // OUT_DIR is like: target/{profile}/build/{pkg}-{hash}/out
            let path = PathBuf::from(out_dir);
            // Navigate up to the profile directory
            if let Some(target_profile_dir) = path.parent() // out -> {pkg}-{hash}
                .and_then(|p| p.parent()) // {pkg}-{hash} -> build
                .and_then(|p| p.parent()) // build -> {profile}
            {
                return target_profile_dir.to_path_buf();
            }
        }
        
        // Fallback to standard location with detected profile
        PathBuf::from("target").join(Self::compile_time_profile_dir())
    }
    
    /// Get the compile-time profile directory name
    fn compile_time_profile_dir() -> &'static str {
        // PROFILE is set by Cargo for build scripts (build.rs)
        // For regular compilation, we use cfg!(debug_assertions)
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    }
    
    /// Get the Cargo profile name from environment
    pub fn cargo_profile_from_env() -> Option<String> {
        // CARGO_BUILD_PROFILE is available in build scripts
        // PROFILE is sometimes available
        std::env::var("CARGO_BUILD_PROFILE")
            .or_else(|_| std::env::var("PROFILE"))
            .ok()
    }
    
    /// Detect build profile from environment
    pub fn from_env() -> Self {
        // Check for common environment variables
        if std::env::var("PROFILE").as_deref() == Ok("release") {
            return BuildProfile::Release;
        }
        
        if std::env::var("AUTO_DEV_BUILD_PROFILE").as_deref() == Ok("release") {
            return BuildProfile::Release;
        }
        
        // Check if we're in CI/CD environment
        if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
            return BuildProfile::Release;
        }
        
        // Default to debug for development
        BuildProfile::Debug
    }
    
    /// Detect the build profile of the current running executable
    pub fn from_current_exe() -> Self {
        // Check if we were compiled with debug assertions
        // This is the most reliable way to detect debug vs release
        if cfg!(debug_assertions) {
            BuildProfile::Debug
        } else {
            BuildProfile::Release
        }
    }
    
    /// Get the actual Cargo target directory at runtime
    pub fn get_target_dir() -> PathBuf {
        // First check CARGO_TARGET_DIR environment variable
        if let Ok(target_dir) = std::env::var("CARGO_TARGET_DIR") {
            return PathBuf::from(target_dir);
        }
        
        // Try to find it from current executable path
        if let Ok(exe_path) = std::env::current_exe() {
            // Walk up the path to find "target" directory
            let mut path = exe_path.as_path();
            while let Some(parent) = path.parent() {
                if parent.file_name() == Some(std::ffi::OsStr::new("target")) {
                    return parent.to_path_buf();
                }
                path = parent;
            }
        }
        
        // Default to "target" in current directory
        PathBuf::from("target")
    }
    
    /// Get the Cargo manifest directory (where Cargo.toml is)
    pub fn get_manifest_dir() -> Option<PathBuf> {
        // CARGO_MANIFEST_DIR is set during compilation
        if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
            return Some(PathBuf::from(manifest_dir));
        }
        
        // Try to find it at runtime by looking for Cargo.toml
        let mut current_dir = std::env::current_dir().ok()?;
        loop {
            let cargo_toml = current_dir.join("Cargo.toml");
            if cargo_toml.exists() {
                return Some(current_dir);
            }
            
            // Move up one directory
            if !current_dir.pop() {
                break;
            }
        }
        
        None
    }
    
    /// Detect profile from executable path
    fn from_current_exe_path() -> Self {
        if let Ok(exe_path) = std::env::current_exe() {
            let exe_str = exe_path.to_string_lossy();
            
            // Check for explicit profile directories
            if exe_str.contains("/release/") || exe_str.contains("\\release\\") {
                return BuildProfile::Release;
            }
            if exe_str.contains("/debug/") || exe_str.contains("\\debug\\") {
                return BuildProfile::Debug;
            }
        }
        
        // Fall back to compile-time debug_assertions
        if cfg!(debug_assertions) {
            BuildProfile::Debug
        } else {
            BuildProfile::Release
        }
    }
}

/// Configuration for self-upgrade
#[derive(Debug, Clone)]
pub struct UpgradeConfig {
    /// Path to the current binary
    pub binary_path: PathBuf,

    /// Path to staging directory for new version
    pub staging_dir: PathBuf,

    /// Build profile to use (debug or release)
    pub build_profile: BuildProfile,

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
            build_profile: BuildProfile::default(),
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
