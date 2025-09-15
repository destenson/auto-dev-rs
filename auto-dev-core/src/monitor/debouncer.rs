//! Event debouncing logic to handle rapid file changes

use crate::monitor::FileChange;
use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, trace};

/// Configuration for the debouncer
#[derive(Debug, Clone)]
pub struct DebouncerConfig {
    /// Delay in milliseconds before emitting an event
    pub delay_ms: u64,
    /// Maximum number of events to buffer
    pub max_buffer_size: usize,
}

impl Default for DebouncerConfig {
    fn default() -> Self {
        Self {
            delay_ms: 500,
            max_buffer_size: 1000,
        }
    }
}

/// Debouncer for file system events
/// 
/// Aggregates rapid changes to the same file and emits
/// a single event after a configurable delay
pub struct Debouncer {
    config: DebouncerConfig,
    pending: Arc<DashMap<PathBuf, PendingChange>>,
    tx: mpsc::UnboundedSender<FileChange>,
    rx: Option<mpsc::UnboundedReceiver<FileChange>>,
}

struct PendingChange {
    change: FileChange,
    last_update: Instant,
    count: usize,
}

impl Debouncer {
    /// Create a new debouncer
    pub fn new(config: DebouncerConfig) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        Self {
            config,
            pending: Arc::new(DashMap::new()),
            tx,
            rx: Some(rx),
        }
    }

    /// Start the debouncer and return a receiver for debounced events
    pub fn start(mut self) -> (mpsc::UnboundedSender<FileChange>, mpsc::UnboundedReceiver<FileChange>) {
        let rx = self.rx.take().expect("Receiver already taken");
        let (input_tx, mut input_rx) = mpsc::unbounded_channel();
        
        let pending = self.pending.clone();
        let config = self.config.clone();
        let output_tx = self.tx.clone();

        // Spawn the debouncing task
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(change) = input_rx.recv() => {
                        handle_change(change, &pending, &config);
                    }
                    _ = sleep(Duration::from_millis(50)) => {
                        emit_ready_changes(&pending, &output_tx, &config);
                    }
                }
            }
        });

        (input_tx, rx)
    }

    /// Process a file change event
    pub fn on_event(&self, change: FileChange) {
        handle_change(change, &self.pending, &self.config);
    }

    /// Get the number of pending changes
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Clear all pending changes
    pub fn clear(&self) {
        self.pending.clear();
    }
}

/// Handle an incoming change event
fn handle_change(
    change: FileChange,
    pending: &Arc<DashMap<PathBuf, PendingChange>>,
    config: &DebouncerConfig,
) {
    let path = change.path.clone();
    let now = Instant::now();

    // Check buffer size limit
    if pending.len() >= config.max_buffer_size {
        debug!("Debouncer buffer full, dropping oldest entries");
        // Remove oldest entries (simple strategy - remove first 10%)
        let to_remove = config.max_buffer_size / 10;
        let mut removed = 0;
        for entry in pending.iter() {
            if removed >= to_remove {
                break;
            }
            pending.remove(entry.key());
            removed += 1;
        }
    }

    pending
        .entry(path.clone())
        .and_modify(|e| {
            e.change = change.clone();
            e.last_update = now;
            e.count += 1;
            trace!("Updated pending change for {:?} (count: {})", path, e.count);
        })
        .or_insert_with(|| {
            trace!("New pending change for {:?}", path);
            PendingChange {
                change,
                last_update: now,
                count: 1,
            }
        });
}

/// Emit changes that have been stable for the configured delay
fn emit_ready_changes(
    pending: &Arc<DashMap<PathBuf, PendingChange>>,
    tx: &mpsc::UnboundedSender<FileChange>,
    config: &DebouncerConfig,
) {
    let now = Instant::now();
    let delay = Duration::from_millis(config.delay_ms);
    
    let mut to_emit = Vec::new();

    // Find changes that are ready to emit
    for entry in pending.iter() {
        let (path, pending_change) = entry.pair();
        if now.duration_since(pending_change.last_update) >= delay {
            to_emit.push((path.clone(), pending_change.change.clone(), pending_change.count));
        }
    }

    // Remove and emit ready changes
    for (path, change, count) in to_emit {
        pending.remove(&path);
        debug!("Emitting debounced change for {:?} (aggregated {} events)", path, count);
        if let Err(e) = tx.send(change) {
            debug!("Failed to send debounced change: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::{ChangeType, FileCategory};
    use std::time::SystemTime;

    #[tokio::test]
    async fn test_debouncer_aggregates_rapid_changes() {
        let config = DebouncerConfig {
            delay_ms: 100,
            max_buffer_size: 100,
        };

        let debouncer = Debouncer::new(config);
        let (tx, mut rx) = debouncer.start();

        let change = FileChange {
            path: PathBuf::from("test.rs"),
            category: FileCategory::Implementation,
            change_type: ChangeType::Modified,
            timestamp: SystemTime::now(),
        };

        // Send multiple rapid changes
        for _ in 0..5 {
            tx.send(change.clone()).unwrap();
            sleep(Duration::from_millis(10)).await;
        }

        // Wait for debounce delay
        sleep(Duration::from_millis(150)).await;

        // Should receive only one event
        let mut count = 0;
        while let Ok(_) = rx.try_recv() {
            count += 1;
        }
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_debouncer_separate_files() {
        let config = DebouncerConfig {
            delay_ms: 100,
            max_buffer_size: 100,
        };

        let debouncer = Debouncer::new(config);
        let (tx, mut rx) = debouncer.start();

        let change1 = FileChange {
            path: PathBuf::from("file1.rs"),
            category: FileCategory::Implementation,
            change_type: ChangeType::Modified,
            timestamp: SystemTime::now(),
        };

        let change2 = FileChange {
            path: PathBuf::from("file2.rs"),
            category: FileCategory::Implementation,
            change_type: ChangeType::Modified,
            timestamp: SystemTime::now(),
        };

        // Send changes to different files
        tx.send(change1).unwrap();
        tx.send(change2).unwrap();

        // Wait for debounce delay
        sleep(Duration::from_millis(150)).await;

        // Should receive two events (one for each file)
        let mut count = 0;
        while let Ok(_) = rx.try_recv() {
            count += 1;
        }
        assert_eq!(count, 2);
    }
}