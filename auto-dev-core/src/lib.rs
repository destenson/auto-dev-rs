//! Core functionality for auto-dev
//!
//! This crate contains the core business logic for the auto-dev tool,
//! including project management, code generation, and plugin systems.

pub mod bootstrap;
pub mod claude;
pub mod context;
pub mod dev_loop;
pub mod docs;
pub mod incremental;
pub mod instructions;
pub mod learning;
pub mod llm;
pub mod mcp;
pub mod metrics;
pub mod modules;
pub mod monitor;
pub mod parser;
pub mod safety;
pub mod self_dev;
pub mod self_monitor;
pub mod self_target;
pub mod self_test;
pub mod self_upgrade;
pub mod synthesis;
pub mod test_gen;
pub mod validation;
pub mod vcs;

use serde::{Deserialize, Serialize};

// Re-export logging macros at crate level
#[cfg(feature = "tracing")]
pub use tracing::{debug, error, info, trace, warn};

#[cfg(all(not(feature = "tracing"), feature = "log"))]
pub use log::{debug, error, info, trace, warn};

// Fallback macros when neither tracing nor log is enabled
#[cfg(not(any(feature = "tracing", feature = "log")))]
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        println!("[INFO] {}", format!($($arg)*));
    };
}

#[cfg(not(any(feature = "tracing", feature = "log")))]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        println!("[DEBUG] {}", format!($($arg)*));
    };
}

#[cfg(not(any(feature = "tracing", feature = "log")))]
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        eprintln!("[WARN] {}", format!($($arg)*));
    };
}

#[cfg(not(any(feature = "tracing", feature = "log")))]
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        eprintln!("[ERROR] {}", format!($($arg)*));
    };
}

#[cfg(not(any(feature = "tracing", feature = "log")))]
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        println!("[TRACE] {}", format!($($arg)*));
    };
}

// Re-export the macros at crate level for consistent access
#[cfg(not(any(feature = "tracing", feature = "log")))]
pub use {debug, error, info, trace, warn};

/// Core struct that will hold the main application state
#[derive(Debug, Default)]
pub struct Core {
    // Future fields will be added as features are implemented
}

impl Core {
    /// Create a new Core instance
    pub fn new() -> Self {
        Self::default()
    }
}

/// Configuration structure for auto-dev
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    /// Project name
    pub project_name: Option<String>,

    /// Verbosity level
    pub verbosity: String,

    /// Plugin configuration
    pub plugins: PluginConfig,
}

/// Plugin configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PluginConfig {
    /// Enabled plugins
    pub enabled: Vec<String>,

    /// Plugin directory path
    pub path: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            project_name: None,
            verbosity: "info".to_string(),
            plugins: PluginConfig { enabled: Vec::new(), path: None },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_creation() {
        let core = Core::new();
        // Basic test to ensure Core can be created
        let _ = format!("{:?}", core);
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.verbosity, "info");
        assert!(config.plugins.enabled.is_empty());
    }
}
