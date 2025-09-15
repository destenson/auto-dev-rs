//! Filesystem monitoring and change detection system
//! 
//! This module provides comprehensive file watching capabilities with:
//! - Event debouncing to handle rapid changes
//! - File type classification
//! - Change impact analysis  
//! - Prioritized change queue management
//! - Gitignore pattern respect

pub mod analyzer;
pub mod classifier;
pub mod debouncer;
pub mod queue;
pub mod watcher;

pub use analyzer::{ChangeAnalyzer, ChangeImpact};
pub use classifier::{FileCategory, FileClassifier};
pub use debouncer::{Debouncer, DebouncerConfig};
pub use queue::{ChangeQueue, QueuedChange};
pub use watcher::{FileWatcher, WatcherConfig};

use std::path::PathBuf;
use std::time::SystemTime;

/// Represents a detected filesystem change
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub path: PathBuf,
    pub category: FileCategory,
    pub change_type: ChangeType,
    pub timestamp: SystemTime,
}

/// Types of filesystem changes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
    Renamed,
}

/// Configuration for the monitoring system
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    pub enabled: bool,
    pub debounce_ms: u64,
    pub max_queue_size: usize,
    pub ignore_patterns: Vec<String>,
    pub watch_hidden: bool,
    pub follow_symlinks: bool,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            debounce_ms: 500,
            max_queue_size: 1000,
            ignore_patterns: vec![
                "target/**".to_string(),
                "node_modules/**".to_string(),
                ".git/**".to_string(),
                "*.tmp".to_string(),
            ],
            watch_hidden: false,
            follow_symlinks: false,
        }
    }
}