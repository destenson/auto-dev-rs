//! Progress tracking and reporting

use super::{Complexity, IncrementStatus, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tracing::{debug, info};
use uuid::Uuid;

/// Tracks progress of incremental implementation
#[derive(Clone)]
pub struct ProgressTracker {
    state: Arc<Mutex<ProgressState>>,
    event_sender: broadcast::Sender<ProgressEvent>,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new(total_increments: usize) -> Self {
        let (event_sender, _) = broadcast::channel(100);

        let state = ProgressState {
            total_increments,
            completed_increments: 0,
            failed_increments: 0,
            current_increment: None,
            increment_timings: HashMap::new(),
            start_time: Instant::now(),
            events: Vec::new(),
        };

        Self { state: Arc::new(Mutex::new(state)), event_sender }
    }

    /// Subscribe to progress events
    pub fn subscribe(&self) -> broadcast::Receiver<ProgressEvent> {
        self.event_sender.subscribe()
    }

    /// Update progress with an event
    pub fn update(&self, event: ProgressEvent) {
        // Update state
        {
            let mut state = self.state.lock().unwrap();

            match &event {
                ProgressEvent::IncrementStarted { id, .. } => {
                    state.current_increment = Some(*id);
                    state.increment_timings.insert(
                        *id,
                        IncrementTiming { start: Instant::now(), end: None, duration: None },
                    );
                }
                ProgressEvent::IncrementCompleted { id, .. } => {
                    state.completed_increments += 1;
                    if state.current_increment == Some(*id) {
                        state.current_increment = None;
                    }
                    if let Some(timing) = state.increment_timings.get_mut(id) {
                        timing.end = Some(Instant::now());
                        timing.duration = Some(timing.end.unwrap() - timing.start);
                    }
                }
                ProgressEvent::IncrementFailed { id, .. } => {
                    state.failed_increments += 1;
                    if state.current_increment == Some(*id) {
                        state.current_increment = None;
                    }
                    if let Some(timing) = state.increment_timings.get_mut(id) {
                        timing.end = Some(Instant::now());
                        timing.duration = Some(timing.end.unwrap() - timing.start);
                    }
                }
                ProgressEvent::TestPassed { .. } | ProgressEvent::TestFailed { .. } => {
                    // Just log these events
                }
                ProgressEvent::Checkpoint { .. } => {
                    // Record checkpoint
                }
                ProgressEvent::Rollback { .. } => {
                    // Record rollback
                }
            }

            state.events.push(event.clone());
        }

        // Broadcast event
        let _ = self.event_sender.send(event.clone());

        // Log the event
        match &event {
            ProgressEvent::IncrementStarted { description, .. } => {
                info!("Started: {}", description);
            }
            ProgressEvent::IncrementCompleted { description, .. } => {
                info!("Completed: {}", description);
            }
            ProgressEvent::IncrementFailed { description, reason, .. } => {
                info!("Failed: {} - {}", description, reason);
            }
            _ => {
                debug!("Progress event: {:?}", event);
            }
        }
    }

    /// Get current progress report
    pub fn get_report(&self) -> ProgressReport {
        let state = self.state.lock().unwrap();

        let elapsed = state.start_time.elapsed();
        let progress_percentage = if state.total_increments > 0 {
            ((state.completed_increments as f32) / (state.total_increments as f32)) * 100.0
        } else {
            0.0
        };

        let success_rate = if state.completed_increments + state.failed_increments > 0 {
            (state.completed_increments as f32)
                / ((state.completed_increments + state.failed_increments) as f32)
                * 100.0
        } else {
            100.0
        };

        let average_duration = if !state.increment_timings.is_empty() {
            let total: Duration = state.increment_timings.values().filter_map(|t| t.duration).sum();
            total / state.increment_timings.len() as u32
        } else {
            Duration::from_secs(0)
        };

        let estimated_remaining = if state.completed_increments > 0 {
            let remaining =
                state.total_increments - state.completed_increments - state.failed_increments;
            average_duration * remaining as u32
        } else {
            Duration::from_secs(0)
        };

        ProgressReport {
            total_increments: state.total_increments,
            completed_increments: state.completed_increments,
            failed_increments: state.failed_increments,
            current_increment: state.current_increment,
            progress_percentage,
            success_rate,
            elapsed_time: elapsed,
            estimated_remaining,
            average_increment_duration: average_duration,
            recent_events: state.events.iter().rev().take(10).cloned().collect(),
        }
    }

    /// Mark an increment as started
    pub fn start_increment(&self, id: Uuid, description: String, complexity: Complexity) {
        self.update(ProgressEvent::IncrementStarted {
            id,
            description,
            complexity,
            timestamp: Utc::now(),
        });
    }

    /// Mark an increment as completed
    pub fn complete_increment(&self, id: Uuid, description: String) {
        self.update(ProgressEvent::IncrementCompleted { id, description, timestamp: Utc::now() });
    }

    /// Mark an increment as failed
    pub fn fail_increment(&self, id: Uuid, description: String, reason: String) {
        self.update(ProgressEvent::IncrementFailed {
            id,
            description,
            reason,
            timestamp: Utc::now(),
        });
    }

    /// Record a test result
    pub fn record_test(&self, increment_id: Uuid, test_name: String, passed: bool) {
        if passed {
            self.update(ProgressEvent::TestPassed {
                increment_id,
                test_name,
                timestamp: Utc::now(),
            });
        } else {
            self.update(ProgressEvent::TestFailed {
                increment_id,
                test_name,
                timestamp: Utc::now(),
            });
        }
    }

    /// Record a checkpoint creation
    pub fn record_checkpoint(&self, checkpoint_id: Uuid) {
        self.update(ProgressEvent::Checkpoint { checkpoint_id, timestamp: Utc::now() });
    }

    /// Record a rollback
    pub fn record_rollback(&self, checkpoint_id: Uuid, reason: String) {
        self.update(ProgressEvent::Rollback { checkpoint_id, reason, timestamp: Utc::now() });
    }

    /// Get a formatted progress bar string
    pub fn get_progress_bar(&self) -> String {
        let report = self.get_report();
        let bar_width = 30;
        let filled = ((report.progress_percentage / 100.0) * bar_width as f32) as usize;
        let empty = bar_width - filled;

        format!(
            "[{}{}] {:.1}% ({}/{}) | Success: {:.1}% | ETA: {:?}",
            "█".repeat(filled),
            "░".repeat(empty),
            report.progress_percentage,
            report.completed_increments,
            report.total_increments,
            report.success_rate,
            report.estimated_remaining
        )
    }
}

/// Internal state of the progress tracker
struct ProgressState {
    total_increments: usize,
    completed_increments: usize,
    failed_increments: usize,
    current_increment: Option<Uuid>,
    increment_timings: HashMap<Uuid, IncrementTiming>,
    start_time: Instant,
    events: Vec<ProgressEvent>,
}

/// Timing information for an increment
struct IncrementTiming {
    start: Instant,
    end: Option<Instant>,
    duration: Option<Duration>,
}

/// Progress event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressEvent {
    IncrementStarted {
        id: Uuid,
        description: String,
        complexity: Complexity,
        timestamp: DateTime<Utc>,
    },
    IncrementCompleted {
        id: Uuid,
        description: String,
        timestamp: DateTime<Utc>,
    },
    IncrementFailed {
        id: Uuid,
        description: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    TestPassed {
        increment_id: Uuid,
        test_name: String,
        timestamp: DateTime<Utc>,
    },
    TestFailed {
        increment_id: Uuid,
        test_name: String,
        timestamp: DateTime<Utc>,
    },
    Checkpoint {
        checkpoint_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    Rollback {
        checkpoint_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
}

/// Progress report
#[derive(Debug, Serialize, Deserialize)]
pub struct ProgressReport {
    pub total_increments: usize,
    pub completed_increments: usize,
    pub failed_increments: usize,
    pub current_increment: Option<Uuid>,
    pub progress_percentage: f32,
    pub success_rate: f32,
    pub elapsed_time: Duration,
    pub estimated_remaining: Duration,
    pub average_increment_duration: Duration,
    pub recent_events: Vec<ProgressEvent>,
}

impl ProgressReport {
    /// Get a human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "Progress: {}/{} increments ({:.1}%) | Success rate: {:.1}% | Elapsed: {:?} | ETA: {:?}",
            self.completed_increments,
            self.total_increments,
            self.progress_percentage,
            self.success_rate,
            self.elapsed_time,
            self.estimated_remaining
        )
    }

    /// Check if all increments are completed
    pub fn is_complete(&self) -> bool {
        self.completed_increments + self.failed_increments >= self.total_increments
    }

    /// Check if implementation was successful
    pub fn is_successful(&self) -> bool {
        self.is_complete() && self.failed_increments == 0
    }
}

/// Progress reporter for console output
pub struct ConsoleProgressReporter {
    tracker: ProgressTracker,
}

impl ConsoleProgressReporter {
    /// Create a new console reporter
    pub fn new(tracker: ProgressTracker) -> Self {
        Self { tracker }
    }

    /// Print current progress to console
    pub fn print_progress(&self) {
        println!("{}", self.tracker.get_progress_bar());
    }

    /// Print detailed report
    pub fn print_report(&self) {
        let report = self.tracker.get_report();

        println!("\n=== Incremental Implementation Progress ===");
        println!("Total increments: {}", report.total_increments);
        println!("Completed: {} ({:.1}%)", report.completed_increments, report.progress_percentage);
        println!("Failed: {}", report.failed_increments);
        println!("Success rate: {:.1}%", report.success_rate);
        println!("Elapsed time: {:?}", report.elapsed_time);
        println!("Estimated remaining: {:?}", report.estimated_remaining);
        println!("Average duration: {:?}", report.average_increment_duration);

        if let Some(current) = report.current_increment {
            println!("Currently executing: {}", current);
        }

        if !report.recent_events.is_empty() {
            println!("\nRecent events:");
            for event in report.recent_events.iter().take(5) {
                match event {
                    ProgressEvent::IncrementCompleted { description, .. } => {
                        println!("  ✓ Completed: {}", description);
                    }
                    ProgressEvent::IncrementFailed { description, reason, .. } => {
                        println!("  ✗ Failed: {} - {}", description, reason);
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_tracking() {
        let tracker = ProgressTracker::new(10);

        // Start an increment
        let id = Uuid::new_v4();
        tracker.start_increment(id, "Test increment".to_string(), Complexity::Simple);

        // Complete it
        tracker.complete_increment(id, "Test increment".to_string());

        // Check report
        let report = tracker.get_report();
        assert_eq!(report.completed_increments, 1);
        assert_eq!(report.total_increments, 10);
        assert!(report.progress_percentage > 0.0);
    }

    #[test]
    fn test_progress_bar() {
        let tracker = ProgressTracker::new(4);

        // Complete 2 out of 4
        for i in 0..2 {
            let id = Uuid::new_v4();
            tracker.complete_increment(id, format!("Increment {}", i));
        }

        let bar = tracker.get_progress_bar();
        assert!(bar.contains("50.0%"));
        assert!(bar.contains("2/4"));
    }
}
