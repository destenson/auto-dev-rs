//! Prioritized change queue management

use crate::monitor::{ChangeImpact, FileChange};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tracing::{debug, warn};

/// A change queued for processing with priority
#[derive(Debug, Clone)]
pub struct QueuedChange {
    pub change: FileChange,
    pub impact: ChangeImpact,
    pub priority: u32,
    pub queued_at: SystemTime,
    pub attempt_count: u32,
}

impl QueuedChange {
    /// Create a new queued change
    pub fn new(change: FileChange, impact: ChangeImpact) -> Self {
        let priority = impact_to_priority(impact);
        Self { change, impact, priority, queued_at: SystemTime::now(), attempt_count: 0 }
    }

    /// Increment the attempt count
    pub fn increment_attempts(&mut self) {
        self.attempt_count += 1;
    }

    /// Check if max attempts exceeded
    pub fn max_attempts_exceeded(&self) -> bool {
        self.attempt_count > 3
    }
}

// Implement ordering for priority queue (higher priority first)
impl Ord for QueuedChange {
    fn cmp(&self, other: &Self) -> Ordering {
        // First compare by priority (higher is better)
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => {
                // Then by timestamp (older is better)
                other.queued_at.cmp(&self.queued_at)
            }
            other => other,
        }
    }
}

impl PartialOrd for QueuedChange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for QueuedChange {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.queued_at == other.queued_at
    }
}

impl Eq for QueuedChange {}

/// Convert impact level to numeric priority
fn impact_to_priority(impact: ChangeImpact) -> u32 {
    match impact {
        ChangeImpact::None => 0,
        ChangeImpact::Minor => 1,
        ChangeImpact::Moderate => 5,
        ChangeImpact::Major => 10,
        ChangeImpact::Critical => 20,
    }
}

/// Manages a prioritized queue of changes to process
pub struct ChangeQueue {
    queue: Arc<Mutex<BinaryHeap<QueuedChange>>>,
    max_size: usize,
    stats: Arc<Mutex<QueueStats>>,
}

#[derive(Debug, Default)]
struct QueueStats {
    total_enqueued: u64,
    total_processed: u64,
    total_dropped: u64,
    total_failed: u64,
}

impl ChangeQueue {
    /// Create a new change queue
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: Arc::new(Mutex::new(BinaryHeap::new())),
            max_size,
            stats: Arc::new(Mutex::new(QueueStats::default())),
        }
    }

    /// Add a change to the queue
    pub fn enqueue(&self, change: FileChange, impact: ChangeImpact) -> Result<(), &'static str> {
        let mut queue = self.queue.lock().unwrap();

        // Check queue size limit
        if queue.len() >= self.max_size {
            // Drop lowest priority item if new item has higher priority
            if let Some(lowest) = queue.peek() {
                let new_priority = impact_to_priority(impact);
                if new_priority <= lowest.priority {
                    warn!("Queue full, dropping new change: {:?}", change.path);
                    self.stats.lock().unwrap().total_dropped += 1;
                    return Err("Queue full");
                }

                // Remove lowest priority items to make room
                let mut temp = Vec::new();
                while queue.len() >= self.max_size && !queue.is_empty() {
                    if let Some(item) = queue.pop() {
                        temp.push(item);
                    }
                }

                // Find and remove the actual lowest priority item
                if let Some(min_idx) = temp
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, item)| item.priority)
                    .map(|(idx, _)| idx)
                {
                    warn!("Queue full, dropping lowest priority: {:?}", temp[min_idx].change.path);
                    temp.remove(min_idx);
                    self.stats.lock().unwrap().total_dropped += 1;
                }

                // Re-add items to queue
                for item in temp {
                    queue.push(item);
                }
            }
        }

        let queued = QueuedChange::new(change, impact);
        debug!("Enqueuing change with priority {}: {:?}", queued.priority, queued.change.path);
        queue.push(queued);
        self.stats.lock().unwrap().total_enqueued += 1;

        Ok(())
    }

    /// Get the next highest priority change
    pub fn dequeue(&self) -> Option<QueuedChange> {
        let mut queue = self.queue.lock().unwrap();
        let item = queue.pop();

        if item.is_some() {
            self.stats.lock().unwrap().total_processed += 1;
            debug!("Dequeued change: {:?}", item.as_ref().unwrap().change.path);
        }

        item
    }

    /// Peek at the next change without removing it
    pub fn peek(&self) -> Option<QueuedChange> {
        let queue = self.queue.lock().unwrap();
        queue.peek().cloned()
    }

    /// Re-queue a change (e.g., after failure)
    pub fn requeue(&self, mut change: QueuedChange) -> Result<(), &'static str> {
        change.increment_attempts();

        if change.max_attempts_exceeded() {
            warn!("Max attempts exceeded for: {:?}", change.change.path);
            self.stats.lock().unwrap().total_failed += 1;
            return Err("Max attempts exceeded");
        }

        // Reduce priority on retry
        change.priority = change.priority.saturating_sub(1);

        let mut queue = self.queue.lock().unwrap();
        debug!("Re-queueing change (attempt {}): {:?}", change.attempt_count, change.change.path);
        queue.push(change);

        Ok(())
    }

    /// Get the current queue size
    pub fn size(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue.lock().unwrap().is_empty()
    }

    /// Clear all items from the queue
    pub fn clear(&self) {
        let mut queue = self.queue.lock().unwrap();
        let count = queue.len();
        queue.clear();
        debug!("Cleared {} items from queue", count);
    }

    /// Get queue statistics
    pub fn stats(&self) -> (u64, u64, u64, u64) {
        let stats = self.stats.lock().unwrap();
        (stats.total_enqueued, stats.total_processed, stats.total_dropped, stats.total_failed)
    }

    /// Get all queued items (for inspection)
    pub fn list_all(&self) -> Vec<QueuedChange> {
        let queue = self.queue.lock().unwrap();
        let mut items: Vec<_> = queue.iter().cloned().collect();
        items.sort_by(|a, b| b.cmp(a)); // Sort by priority
        items
    }

    /// Remove a specific change from the queue
    pub fn remove(&self, path: &std::path::Path) -> bool {
        let mut queue = self.queue.lock().unwrap();
        let original_size = queue.len();

        // Collect all items except the one to remove
        let items: Vec<_> = queue.drain().filter(|item| item.change.path != path).collect();

        // Re-add filtered items
        for item in items {
            queue.push(item);
        }

        let removed = original_size > queue.len();
        if removed {
            debug!("Removed change from queue: {:?}", path);
        }

        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::{ChangeType, FileCategory};
    use std::path::PathBuf;

    #[test]
    fn test_queue_priority_ordering() {
        let queue = ChangeQueue::new(10);

        // Add changes with different impacts
        let change1 = FileChange {
            path: PathBuf::from("low.rs"),
            category: FileCategory::Documentation,
            change_type: ChangeType::Modified,
            timestamp: SystemTime::now(),
        };

        let change2 = FileChange {
            path: PathBuf::from("high.rs"),
            category: FileCategory::Specification,
            change_type: ChangeType::Modified,
            timestamp: SystemTime::now(),
        };

        queue.enqueue(change1.clone(), ChangeImpact::Minor).unwrap();
        queue.enqueue(change2.clone(), ChangeImpact::Critical).unwrap();

        // High priority should come first
        let first = queue.dequeue().unwrap();
        assert_eq!(first.change.path, PathBuf::from("high.rs"));

        let second = queue.dequeue().unwrap();
        assert_eq!(second.change.path, PathBuf::from("low.rs"));
    }

    #[test]
    fn test_queue_size_limit() {
        let queue = ChangeQueue::new(2);

        let change1 = FileChange {
            path: PathBuf::from("1.rs"),
            category: FileCategory::Implementation,
            change_type: ChangeType::Modified,
            timestamp: SystemTime::now(),
        };

        let change2 = FileChange {
            path: PathBuf::from("2.rs"),
            category: FileCategory::Implementation,
            change_type: ChangeType::Modified,
            timestamp: SystemTime::now(),
        };

        let change3 = FileChange {
            path: PathBuf::from("3.rs"),
            category: FileCategory::Specification,
            change_type: ChangeType::Modified,
            timestamp: SystemTime::now(),
        };

        queue.enqueue(change1, ChangeImpact::Minor).unwrap();
        queue.enqueue(change2, ChangeImpact::Minor).unwrap();

        // This should succeed because it has higher priority
        queue.enqueue(change3, ChangeImpact::Critical).unwrap();

        // Queue should still have 2 items (one was dropped)
        assert_eq!(queue.size(), 2);
    }

    #[test]
    fn test_requeue_with_attempts() {
        let queue = ChangeQueue::new(10);

        let change = FileChange {
            path: PathBuf::from("test.rs"),
            category: FileCategory::Implementation,
            change_type: ChangeType::Modified,
            timestamp: SystemTime::now(),
        };

        queue.enqueue(change, ChangeImpact::Major).unwrap();

        let queued = queue.dequeue().unwrap();
        assert_eq!(queued.attempt_count, 0);

        // Requeue should increment attempts
        queue.requeue(queued.clone()).unwrap();

        let requeued = queue.dequeue().unwrap();
        assert_eq!(requeued.attempt_count, 1);
        assert!(requeued.priority < queued.priority); // Priority reduced
    }
}
