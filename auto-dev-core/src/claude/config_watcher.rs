//! File watcher for Claude configuration changes
//!
//! Monitors .claude directories for changes and triggers reloads.

use crate::claude::{ClaudeContextProvider, CommandParser, CommandRegistrySystem, CommandSource};
use crate::monitor::{FileWatcher, WatcherConfig, FileChange, ChangeType, MonitorConfig};
use anyhow::{Context, Result};
use notify::{Event, EventKind, RecursiveMode, Watcher as NotifyWatcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

/// Events that can occur in Claude configuration
#[derive(Debug, Clone)]
pub enum ClaudeFileChange {
    /// CLAUDE.md file changed
    ClaudeMdChanged(PathBuf),
    /// Command file added or modified
    CommandChanged(PathBuf),
    /// Command file deleted
    CommandDeleted(PathBuf),
    /// Directory created or deleted
    DirectoryChanged(PathBuf),
}

/// Handler for configuration reload
pub struct ReloadHandler {
    /// Reference to context provider
    context_provider: Arc<ClaudeContextProvider>,
    /// Command registry
    command_registry: Arc<CommandRegistrySystem>,
}

impl ReloadHandler {
    /// Create a new reload handler
    pub fn new(
        context_provider: Arc<ClaudeContextProvider>,
        command_registry: Arc<CommandRegistrySystem>,
    ) -> Self {
        Self {
            context_provider,
            command_registry,
        }
    }

    /// Handle a file change event
    pub async fn handle_change(&self, change: ClaudeFileChange) -> Result<()> {
        match change {
            ClaudeFileChange::ClaudeMdChanged(path) => {
                info!("CLAUDE.md changed at {:?}, reloading context", path);
                self.context_provider.reload().await?;
            }
            ClaudeFileChange::CommandChanged(path) => {
                info!("Command file changed at {:?}, reloading commands", path);
                self.reload_command(&path).await?;
            }
            ClaudeFileChange::CommandDeleted(path) => {
                info!("Command file deleted at {:?}, refreshing registry", path);
                self.refresh_registry().await?;
            }
            ClaudeFileChange::DirectoryChanged(path) => {
                info!("Directory changed at {:?}, full reload", path);
                self.context_provider.reload().await?;
                self.refresh_registry().await?;
            }
        }
        Ok(())
    }

    /// Reload a specific command file
    async fn reload_command(&self, path: &Path) -> Result<()> {
        let mut parser = CommandParser::new();
        parser.parse_file(path)?;
        
        // Determine source based on path
        let source = if path.to_string_lossy().contains(".claude") {
            if path.to_string_lossy().contains("home") 
                || path.to_string_lossy().contains("Users") {
                CommandSource::Global
            } else {
                CommandSource::Project
            }
        } else {
            CommandSource::Project
        };

        // Register updated commands
        let registry = parser.into_registry();
        for command in registry.all_commands() {
            self.command_registry.register_command(command.clone(), source)?;
        }

        Ok(())
    }

    /// Refresh entire command registry
    async fn refresh_registry(&self) -> Result<()> {
        self.command_registry.clear();
        self.context_provider.reload().await?;
        Ok(())
    }
}

/// Debounce buffer to prevent rapid reloads
struct DebounceBuffer {
    /// Pending changes by path
    changes: HashMap<PathBuf, (ClaudeFileChange, Instant)>,
    /// Debounce window duration
    window: Duration,
}

impl DebounceBuffer {
    fn new(window_ms: u64) -> Self {
        Self {
            changes: HashMap::new(),
            window: Duration::from_millis(window_ms),
        }
    }

    /// Add a change to the buffer
    fn add(&mut self, path: PathBuf, change: ClaudeFileChange) {
        self.changes.insert(path, (change, Instant::now()));
    }

    /// Get changes that are ready (past debounce window)
    fn take_ready(&mut self) -> Vec<ClaudeFileChange> {
        let now = Instant::now();
        let mut ready = Vec::new();
        
        self.changes.retain(|_path, (change, time)| {
            if now.duration_since(*time) >= self.window {
                ready.push(change.clone());
                false // Remove from buffer
            } else {
                true // Keep in buffer
            }
        });

        ready
    }

    /// Check if buffer has pending changes
    fn has_pending(&self) -> bool {
        !self.changes.is_empty()
    }
}

/// Specialized watcher for Claude configuration
pub struct ClaudeConfigWatcher {
    /// Paths to watch
    watch_paths: Vec<PathBuf>,
    /// Change event sender
    tx: mpsc::UnboundedSender<ClaudeFileChange>,
    /// Debounce buffer
    debounce: Arc<RwLock<DebounceBuffer>>,
    /// Reload handler
    reload_handler: Arc<ReloadHandler>,
}

impl ClaudeConfigWatcher {
    /// Create a new Claude config watcher
    pub fn new(
        context_provider: Arc<ClaudeContextProvider>,
        command_registry: Arc<CommandRegistrySystem>,
    ) -> Self {
        let (tx, _rx) = mpsc::unbounded_channel();
        let reload_handler = Arc::new(ReloadHandler::new(context_provider, command_registry));
        
        Self {
            watch_paths: Vec::new(),
            tx,
            debounce: Arc::new(RwLock::new(DebounceBuffer::new(500))),
            reload_handler,
        }
    }

    /// Add paths to watch
    pub fn add_watch_paths(&mut self, paths: Vec<PathBuf>) {
        self.watch_paths.extend(paths);
    }

    /// Discover and add .claude directories
    pub async fn discover_watch_paths(&mut self) -> Result<()> {
        let mut paths = Vec::new();

        // Check project .claude
        let project_claude = PathBuf::from(".claude");
        if project_claude.exists() {
            paths.push(project_claude);
        }

        // Check home .claude
        if let Some(home) = dirs::home_dir() {
            let home_claude = home.join(".claude");
            if home_claude.exists() {
                paths.push(home_claude);
            }
        }

        self.watch_paths = paths;
        Ok(())
    }

    /// Start watching for changes
    pub async fn start(self) -> Result<mpsc::UnboundedReceiver<ClaudeFileChange>> {
        let (tx, rx) = mpsc::unbounded_channel();
        
        if self.watch_paths.is_empty() {
            warn!("No Claude configuration directories to watch");
            return Ok(rx);
        }

        let debounce = self.debounce.clone();
        let reload_handler = self.reload_handler.clone();

        // Create watcher configuration
        let config = WatcherConfig {
            paths: self.watch_paths.clone(),
            recursive: true,
            monitor_config: MonitorConfig {
                ignore_patterns: vec![
                    "*.swp".to_string(),
                    "*.tmp".to_string(),
                    "~*".to_string(),
                ],
                ..Default::default()
            },
        };

        // Create and start file watcher
        let watcher = FileWatcher::new(config);
        let mut change_rx = watcher.start().await?;

        // Spawn task to process file changes
        tokio::spawn(async move {
            while let Some(change) = change_rx.recv().await {
                if let Some(claude_change) = classify_change(&change).await {
                    // Add to debounce buffer
                    let mut buffer = debounce.write().await;
                    buffer.add(change.path.clone(), claude_change.clone());
                    drop(buffer);

                    // Send to channel
                    if tx.send(claude_change).is_err() {
                        break;
                    }
                }
            }
        });

        // Spawn task to process debounced changes
        let debounce_clone = self.debounce.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                
                let mut buffer = debounce_clone.write().await;
                let ready = buffer.take_ready();
                drop(buffer);

                for change in ready {
                    if let Err(e) = reload_handler.handle_change(change).await {
                        warn!("Failed to handle configuration change: {}", e);
                    }
                }
            }
        });

        Ok(rx)
    }

    /// Check if any paths are being watched
    pub fn is_watching(&self) -> bool {
        !self.watch_paths.is_empty()
    }
}

/// Classify a general file change as a Claude-specific change
async fn classify_change(change: &FileChange) -> Option<ClaudeFileChange> {
    let path = &change.path;
    let path_str = path.to_string_lossy();

    // Check if it's in a .claude directory
    if !path_str.contains(".claude") {
        return None;
    }

    // Ignore editor temp files
    if path_str.ends_with(".swp") || path_str.ends_with("~") || path_str.contains(".tmp") {
        return None;
    }

    // Classify based on file name and change type
    if path.file_name() == Some("CLAUDE.md".as_ref()) {
        Some(ClaudeFileChange::ClaudeMdChanged(path.clone()))
    } else if path_str.contains("/commands/") || path_str.contains("\\commands\\") {
        match change.change_type {
            ChangeType::Created | ChangeType::Modified => {
                if path.extension() == Some("md".as_ref()) {
                    Some(ClaudeFileChange::CommandChanged(path.clone()))
                } else {
                    None
                }
            }
            ChangeType::Deleted => {
                Some(ClaudeFileChange::CommandDeleted(path.clone()))
            }
            _ => None,
        }
    } else if path.is_dir() {
        Some(ClaudeFileChange::DirectoryChanged(path.clone()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::ClaudeConfigDiscovery;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_debounce_buffer() {
        let mut buffer = DebounceBuffer::new(100);
        
        let path1 = PathBuf::from("/test1");
        let path2 = PathBuf::from("/test2");
        
        buffer.add(path1.clone(), ClaudeFileChange::ClaudeMdChanged(path1.clone()));
        buffer.add(path2.clone(), ClaudeFileChange::CommandChanged(path2.clone()));
        
        assert!(buffer.has_pending());
        
        // Immediately, nothing is ready
        let ready = buffer.take_ready();
        assert!(ready.is_empty());
        
        // After waiting, changes are ready
        std::thread::sleep(Duration::from_millis(150));
        let ready = buffer.take_ready();
        assert_eq!(ready.len(), 2);
        assert!(!buffer.has_pending());
    }

    #[tokio::test]
    async fn test_classify_change() {
        let change = FileChange {
            path: PathBuf::from("/project/.claude/CLAUDE.md"),
            category: crate::monitor::FileCategory::Configuration,
            change_type: ChangeType::Modified,
            timestamp: std::time::SystemTime::now(),
        };

        let result = classify_change(&change).await;
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ClaudeFileChange::ClaudeMdChanged(_)));
    }

    #[tokio::test]
    async fn test_classify_command_change() {
        let change = FileChange {
            path: PathBuf::from("/project/.claude/commands/test.md"),
            category: crate::monitor::FileCategory::Configuration,
            change_type: ChangeType::Created,
            timestamp: std::time::SystemTime::now(),
        };

        let result = classify_change(&change).await;
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ClaudeFileChange::CommandChanged(_)));
    }

    #[tokio::test]
    async fn test_ignore_temp_files() {
        let change = FileChange {
            path: PathBuf::from("/project/.claude/CLAUDE.md.swp"),
            category: crate::monitor::FileCategory::Configuration,
            change_type: ChangeType::Modified,
            timestamp: std::time::SystemTime::now(),
        };

        let result = classify_change(&change).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_discover_watch_paths() {
        let temp_dir = TempDir::new().unwrap();
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir(&claude_dir).unwrap();

        std::env::set_current_dir(temp_dir.path()).unwrap();

        let context_provider = Arc::new(ClaudeContextProvider::new().unwrap());
        let registry = Arc::new(CommandRegistrySystem::new());
        
        let mut watcher = ClaudeConfigWatcher::new(context_provider, registry);
        watcher.discover_watch_paths().await.unwrap();
        
        assert!(watcher.is_watching());
    }
}