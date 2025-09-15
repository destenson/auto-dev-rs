//! Core file system watcher implementation using notify crate

use crate::monitor::{ChangeType, FileChange, FileClassifier, MonitorConfig};
use anyhow::Result;
use ignore::gitignore::GitignoreBuilder;
use notify::{Config, Event, EventKind, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Configuration for the file watcher
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    pub paths: Vec<PathBuf>,
    pub recursive: bool,
    pub monitor_config: MonitorConfig,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            paths: vec![PathBuf::from(".")],
            recursive: true,
            monitor_config: MonitorConfig::default(),
        }
    }
}

/// Main file system watcher
pub struct FileWatcher {
    config: WatcherConfig,
    classifier: Arc<FileClassifier>,
    tx: mpsc::UnboundedSender<FileChange>,
    rx: Option<mpsc::UnboundedReceiver<FileChange>>,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(config: WatcherConfig) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let classifier = Arc::new(FileClassifier::new());
        
        Self {
            config,
            classifier,
            tx,
            rx: Some(rx),
        }
    }

    /// Start watching for file changes
    pub async fn start(mut self) -> Result<mpsc::UnboundedReceiver<FileChange>> {
        let rx = self.rx.take().expect("Receiver already taken");
        
        // Build gitignore matcher
        let mut gitignore_builder = GitignoreBuilder::new(&self.config.paths[0]);
        for pattern in &self.config.monitor_config.ignore_patterns {
            gitignore_builder.add_line(None, pattern)?;
        }
        let gitignore = gitignore_builder.build()?;

        let tx = self.tx.clone();
        let classifier = self.classifier.clone();
        let monitor_config = self.config.monitor_config.clone();

        // Create the notify watcher
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    if let Some(change) = process_event(event, &classifier, &gitignore, &monitor_config) {
                        if let Err(e) = tx.send(change) {
                            error!("Failed to send change event: {}", e);
                        }
                    }
                }
                Err(e) => error!("Watch error: {:?}", e),
            }
        })?;

        // Add paths to watch
        for path in &self.config.paths {
            let mode = if self.config.recursive {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            
            watcher.watch(path, mode)?;
            info!("Watching path: {:?} (recursive: {})", path, self.config.recursive);
        }

        // Keep watcher alive by spawning a task
        tokio::spawn(async move {
            // Keep the watcher alive
            let _watcher = watcher;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        });

        Ok(rx)
    }

    /// Add a path to watch
    pub fn add_path(&mut self, path: PathBuf) {
        if !self.config.paths.contains(&path) {
            self.config.paths.push(path);
        }
    }

    /// Remove a path from watching
    pub fn remove_path(&mut self, path: &Path) {
        self.config.paths.retain(|p| p != path);
    }
}

/// Process a notify event into a FileChange
fn process_event(
    event: Event,
    classifier: &FileClassifier,
    gitignore: &ignore::gitignore::Gitignore,
    config: &MonitorConfig,
) -> Option<FileChange> {
    // Extract the path from the event
    let path = event.paths.first()?.clone();
    
    // Check if path should be ignored
    if gitignore.matched(&path, path.is_dir()).is_ignore() {
        debug!("Ignoring path due to gitignore: {:?}", path);
        return None;
    }

    // Check hidden files
    if !config.watch_hidden {
        if let Some(name) = path.file_name() {
            if name.to_string_lossy().starts_with('.') {
                debug!("Ignoring hidden file: {:?}", path);
                return None;
            }
        }
    }

    // Determine change type from event
    let change_type = match event.kind {
        EventKind::Create(_) => ChangeType::Created,
        EventKind::Modify(_) => ChangeType::Modified,
        EventKind::Remove(_) => ChangeType::Deleted,
        EventKind::Other => return None,
        _ => return None,
    };

    // Classify the file
    let category = classifier.classify(&path);
    
    // Skip if not a monitored category
    if category == crate::monitor::FileCategory::Other {
        debug!("Skipping non-monitored file: {:?}", path);
        return None;
    }

    Some(FileChange {
        path,
        category,
        change_type,
        timestamp: SystemTime::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_file_watcher_detects_changes() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.md");
        
        let config = WatcherConfig {
            paths: vec![temp_dir.path().to_path_buf()],
            recursive: true,
            monitor_config: MonitorConfig::default(),
        };

        let watcher = FileWatcher::new(config);
        let mut rx = watcher.start().await.unwrap();

        // Create a file
        std::fs::write(&test_file, "test content").unwrap();
        
        // Wait for event
        sleep(Duration::from_millis(100)).await;
        
        if let Ok(change) = rx.try_recv() {
            assert_eq!(change.change_type, ChangeType::Created);
            assert_eq!(change.path, test_file);
        }
    }
}