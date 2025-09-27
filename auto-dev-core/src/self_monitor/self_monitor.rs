#![allow(unused)]
use anyhow::Result;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tracing::{debug, info, warn};

use crate::monitor::classifier::FileClassifier;
use crate::monitor::watcher::FileWatcher;
use crate::monitor::{ChangeType, FileCategory, FileChange};

/// Configuration for self-monitoring behavior
#[derive(Debug, Clone)]
pub struct SelfMonitorConfig {
    /// Root directory of the auto-dev-rs project
    pub project_root: PathBuf,
    /// Paths that can be safely modified
    pub modifiable_paths: Vec<PathBuf>,
    /// Cooldown period between modifications to prevent loops
    pub cooldown_ms: u64,
    /// Maximum number of modifications per minute
    pub max_modifications_per_minute: usize,
}

impl Default for SelfMonitorConfig {
    fn default() -> Self {
        Self {
            project_root: PathBuf::from("."),
            modifiable_paths: vec![
                PathBuf::from("src"),
                PathBuf::from("tests"),
                PathBuf::from("examples"),
            ],
            cooldown_ms: 1000,
            max_modifications_per_minute: 10,
        }
    }
}

/// Specialized monitor for watching the auto-dev-rs codebase itself
pub struct SelfMonitor {
    config: SelfMonitorConfig,
    watcher: Option<RecommendedWatcher>,
    classifier: FileClassifier,
    modification_tracker: Arc<Mutex<ModificationTracker>>,
}

impl SelfMonitor {
    pub fn new(config: SelfMonitorConfig) -> Result<Self> {
        Ok(Self {
            config,
            watcher: None,
            classifier: FileClassifier::new(),
            modification_tracker: Arc::new(Mutex::new(ModificationTracker::new())),
        })
    }

    /// Start monitoring the project directory
    pub fn start(&mut self) -> Result<()> {
        let project_root = self.config.project_root.clone();
        let tracker = self.modification_tracker.clone();

        let mut watcher =
            notify::recommended_watcher(move |res: Result<Event, notify::Error>| match res {
                Ok(event) => {
                    if let Err(e) = Self::handle_event(event, &tracker) {
                        warn!("Error handling file event: {}", e);
                    }
                }
                Err(e) => warn!("Watch error: {}", e),
            })?;

        // Watch the project source directories
        for path in &self.config.modifiable_paths {
            let full_path = self.config.project_root.join(path);
            if full_path.exists() {
                watcher.watch(&full_path, RecursiveMode::Recursive)?;
                info!("Watching directory: {:?}", full_path);
            }
        }

        self.watcher = Some(watcher);
        info!("Self-monitoring started for project: {:?}", self.config.project_root);
        Ok(())
    }

    /// Stop monitoring
    pub fn stop(&mut self) {
        self.watcher = None;
        info!("Self-monitoring stopped");
    }

    /// Check if a path is safe to modify
    pub fn is_safe_to_modify(&self, path: &Path) -> bool {
        // Check if path is within allowed directories
        for allowed_dir in &self.config.modifiable_paths {
            let full_allowed = self.config.project_root.join(allowed_dir);
            if path.starts_with(&full_allowed) {
                // Additional checks for critical files
                if path.ends_with("main.rs") || path.ends_with("lib.rs") {
                    debug!("Critical file modification attempted: {:?}", path);
                    return false;
                }
                return true;
            }
        }
        false
    }

    /// Handle file system events
    fn handle_event(event: Event, tracker: &Arc<Mutex<ModificationTracker>>) -> Result<()> {
        let change_type = match event.kind {
            EventKind::Create(_) => ChangeType::Created,
            EventKind::Modify(_) => ChangeType::Modified,
            EventKind::Remove(_) => ChangeType::Deleted,
            _ => return Ok(()),
        };

        for path in event.paths {
            if Self::should_ignore(&path) {
                continue;
            }

            let mut tracker_guard = tracker.lock().unwrap();
            tracker_guard.record_modification(&path, change_type)?;

            info!("Self-modification detected: {:?} {:?}", change_type, path);
        }

        Ok(())
    }

    /// Check if a path should be ignored
    fn should_ignore(path: &Path) -> bool {
        // Ignore build artifacts and temporary files
        if path.components().any(|c| {
            c.as_os_str() == "target" || c.as_os_str() == ".git" || c.as_os_str() == "node_modules"
        }) {
            return true;
        }

        // Ignore temporary and backup files
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            if name_str.ends_with(".tmp") || name_str.ends_with("~") || name_str.starts_with(".#") {
                return true;
            }
        }

        false
    }

    /// Get recent modifications for analysis
    pub fn get_recent_modifications(&self, duration: Duration) -> Vec<ModificationRecord> {
        let tracker = self.modification_tracker.lock().unwrap();
        tracker.get_recent_modifications(duration)
    }
}

/// Tracks modification history to prevent loops
struct ModificationTracker {
    /// History of modifications with timestamps
    history: Vec<ModificationRecord>,
    /// Last modification time per file
    last_modified: HashMap<PathBuf, SystemTime>,
}

impl ModificationTracker {
    fn new() -> Self {
        Self { history: Vec::new(), last_modified: HashMap::new() }
    }

    fn record_modification(&mut self, path: &Path, change_type: ChangeType) -> Result<()> {
        let now = SystemTime::now();

        // Check cooldown period
        if let Some(last_time) = self.last_modified.get(path) {
            if let Ok(duration) = now.duration_since(*last_time) {
                if duration < Duration::from_millis(1000) {
                    debug!("Modification too soon, ignoring: {:?}", path);
                    return Ok(());
                }
            }
        }

        let record = ModificationRecord {
            path: path.to_path_buf(),
            change_type,
            timestamp: now,
            source: ModificationSource::Internal,
        };

        self.history.push(record);
        self.last_modified.insert(path.to_path_buf(), now);

        // Trim old history (keep last 1000 entries)
        if self.history.len() > 1000 {
            self.history.drain(0..100);
        }

        Ok(())
    }

    fn get_recent_modifications(&self, duration: Duration) -> Vec<ModificationRecord> {
        let cutoff = SystemTime::now() - duration;
        self.history.iter().filter(|r| r.timestamp > cutoff).cloned().collect()
    }
}

/// Record of a single modification
#[derive(Debug, Clone)]
pub struct ModificationRecord {
    pub path: PathBuf,
    pub change_type: ChangeType,
    pub timestamp: SystemTime,
    pub source: ModificationSource,
}

/// Source of a modification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModificationSource {
    /// Modified by auto-dev itself
    Internal,
    /// Modified by external process
    External,
    /// Modified by user
    User,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_safe_to_modify() {
        let config = SelfMonitorConfig {
            project_root: PathBuf::from("/project"),
            modifiable_paths: vec![PathBuf::from("src"), PathBuf::from("tests")],
            ..Default::default()
        };

        let monitor = SelfMonitor::new(config).unwrap();

        // Test allowed paths
        assert!(monitor.is_safe_to_modify(&PathBuf::from("/project/src/module.rs")));
        assert!(monitor.is_safe_to_modify(&PathBuf::from("/project/tests/test.rs")));

        // Test disallowed paths
        assert!(!monitor.is_safe_to_modify(&PathBuf::from("/project/Cargo.toml")));
        assert!(!monitor.is_safe_to_modify(&PathBuf::from("/project/src/main.rs")));
        assert!(!monitor.is_safe_to_modify(&PathBuf::from("/project/src/lib.rs")));
    }

    #[test]
    fn test_should_ignore() {
        assert!(SelfMonitor::should_ignore(&PathBuf::from("target/debug/file")));
        assert!(SelfMonitor::should_ignore(&PathBuf::from(".git/config")));
        assert!(SelfMonitor::should_ignore(&PathBuf::from("file.tmp")));
        assert!(SelfMonitor::should_ignore(&PathBuf::from("file~")));
        assert!(!SelfMonitor::should_ignore(&PathBuf::from("src/main.rs")));
    }
}
