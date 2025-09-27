use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use std::collections::{HashMap, VecDeque};
use anyhow::{Result, bail};
use tracing::{debug, warn, error};

/// Detects and prevents infinite modification loops
pub struct LoopDetector {
    /// Sliding window of modifications per file
    modification_windows: HashMap<PathBuf, ModificationWindow>,
    /// Configuration for loop detection
    config: LoopDetectorConfig,
    /// Files currently in cooldown
    cooldown_files: HashMap<PathBuf, SystemTime>,
}

/// Configuration for loop detection behavior
#[derive(Debug, Clone)]
pub struct LoopDetectorConfig {
    /// Maximum modifications allowed in the window period
    pub max_modifications_per_window: usize,
    /// Time window for counting modifications (in seconds)
    pub window_duration_secs: u64,
    /// Cooldown period after loop detection (in seconds)
    pub cooldown_duration_secs: u64,
    /// Number of iterations before declaring a loop
    pub loop_threshold: usize,
}

impl Default for LoopDetectorConfig {
    fn default() -> Self {
        Self {
            max_modifications_per_window: 5,
            window_duration_secs: 60,
            cooldown_duration_secs: 300, // 5 minutes
            loop_threshold: 3,
        }
    }
}

impl LoopDetector {
    pub fn new(config: LoopDetectorConfig) -> Self {
        Self {
            modification_windows: HashMap::new(),
            config,
            cooldown_files: HashMap::new(),
        }
    }

    /// Check if a modification would create a loop
    pub fn check_modification(&mut self, path: &Path) -> Result<LoopDetectionResult> {
        let now = SystemTime::now();
        
        // Check if file is in cooldown
        if let Some(cooldown_end) = self.cooldown_files.get(path) {
            if now < *cooldown_end {
                let remaining = cooldown_end.duration_since(now)
                    .unwrap_or(Duration::from_secs(0));
                return Ok(LoopDetectionResult::InCooldown(remaining));
            } else {
                // Cooldown expired, remove from map
                self.cooldown_files.remove(path);
            }
        }

        // Get or create modification window for this file
        let window = self.modification_windows
            .entry(path.to_path_buf())
            .or_insert_with(|| ModificationWindow::new(
                Duration::from_secs(self.config.window_duration_secs)
            ));

        // Record this modification attempt
        window.record_modification(now);

        // Check for rapid modifications
        let recent_count = window.count_recent_modifications();
        if recent_count >= self.config.max_modifications_per_window {
            warn!("Loop detected for file: {:?}, {} modifications in window", 
                  path, recent_count);
            
            // Put file in cooldown
            let cooldown_end = now + Duration::from_secs(self.config.cooldown_duration_secs);
            self.cooldown_files.insert(path.to_path_buf(), cooldown_end);
            
            return Ok(LoopDetectionResult::LoopDetected {
                modification_count: recent_count,
                window_duration: Duration::from_secs(self.config.window_duration_secs),
            });
        }

        // Check for modification patterns (same changes being reverted/reapplied)
        if let Some(pattern) = self.detect_modification_pattern(path) {
            warn!("Modification pattern detected for file: {:?}", path);
            return Ok(LoopDetectionResult::PatternDetected(pattern));
        }

        Ok(LoopDetectionResult::Safe)
    }

    /// Detect if there's a repeating pattern of modifications
    fn detect_modification_pattern(&self, path: &Path) -> Option<ModificationPattern> {
        let window = self.modification_windows.get(path)?;
        
        // Check for ping-pong pattern (A->B->A->B)
        if window.has_ping_pong_pattern() {
            return Some(ModificationPattern::PingPong);
        }

        // Check for rapid burst pattern
        if window.has_burst_pattern() {
            return Some(ModificationPattern::RapidBurst);
        }

        None
    }

    /// Clear the history for a specific file
    pub fn clear_file_history(&mut self, path: &Path) {
        self.modification_windows.remove(path);
        self.cooldown_files.remove(path);
    }

    /// Get current status of a file
    pub fn get_file_status(&self, path: &Path) -> FileLoopStatus {
        let now = SystemTime::now();
        
        if let Some(cooldown_end) = self.cooldown_files.get(path) {
            if now < *cooldown_end {
                return FileLoopStatus::InCooldown;
            }
        }

        if let Some(window) = self.modification_windows.get(path) {
            let count = window.count_recent_modifications();
            if count > self.config.max_modifications_per_window / 2 {
                return FileLoopStatus::Warning(count);
            }
        }

        FileLoopStatus::Normal
    }

    /// Clean up old tracking data
    pub fn cleanup_old_data(&mut self) {
        let now = SystemTime::now();
        
        // Remove expired cooldowns
        self.cooldown_files.retain(|_, cooldown_end| now < *cooldown_end);
        
        // Clean up old windows
        self.modification_windows.retain(|_, window| {
            window.has_recent_activity()
        });
    }
}

/// Tracks modifications within a time window
struct ModificationWindow {
    /// Timestamps of recent modifications
    timestamps: VecDeque<SystemTime>,
    /// Duration of the sliding window
    window_duration: Duration,
    /// Hash of content at each modification (for pattern detection)
    content_hashes: VecDeque<u64>,
}

impl ModificationWindow {
    fn new(window_duration: Duration) -> Self {
        Self {
            timestamps: VecDeque::new(),
            window_duration,
            content_hashes: VecDeque::new(),
        }
    }

    fn record_modification(&mut self, timestamp: SystemTime) {
        self.timestamps.push_back(timestamp);
        self.cleanup_old_entries(timestamp);
    }

    fn cleanup_old_entries(&mut self, now: SystemTime) {
        let cutoff = now - self.window_duration;
        
        while let Some(front) = self.timestamps.front() {
            if *front < cutoff {
                self.timestamps.pop_front();
                if !self.content_hashes.is_empty() {
                    self.content_hashes.pop_front();
                }
            } else {
                break;
            }
        }
    }

    fn count_recent_modifications(&self) -> usize {
        self.timestamps.len()
    }

    fn has_recent_activity(&self) -> bool {
        if let Some(last) = self.timestamps.back() {
            if let Ok(elapsed) = SystemTime::now().duration_since(*last) {
                return elapsed < self.window_duration * 2;
            }
        }
        false
    }

    fn has_ping_pong_pattern(&self) -> bool {
        if self.content_hashes.len() < 4 {
            return false;
        }

        // Check if we see A->B->A->B pattern in hashes
        let hashes: Vec<_> = self.content_hashes.iter().cloned().collect();
        for i in 0..hashes.len().saturating_sub(3) {
            if hashes[i] == hashes[i + 2] && 
               hashes[i + 1] == hashes[i + 3] &&
               hashes[i] != hashes[i + 1] {
                return true;
            }
        }

        false
    }

    fn has_burst_pattern(&self) -> bool {
        if self.timestamps.len() < 3 {
            return false;
        }

        // Check if modifications are happening very rapidly (< 1 second apart)
        let mut rapid_count = 0;
        for i in 1..self.timestamps.len() {
            if let (Some(prev), Some(curr)) = (
                self.timestamps.get(i - 1),
                self.timestamps.get(i)
            ) {
                if let Ok(duration) = curr.duration_since(*prev) {
                    if duration < Duration::from_secs(1) {
                        rapid_count += 1;
                    }
                }
            }
        }

        rapid_count >= 3
    }
}

/// Result of loop detection check
#[derive(Debug, Clone)]
pub enum LoopDetectionResult {
    /// Modification is safe to proceed
    Safe,
    /// Loop has been detected
    LoopDetected {
        modification_count: usize,
        window_duration: Duration,
    },
    /// File is in cooldown period
    InCooldown(Duration),
    /// A modification pattern was detected
    PatternDetected(ModificationPattern),
}

impl LoopDetectionResult {
    pub fn is_safe(&self) -> bool {
        matches!(self, LoopDetectionResult::Safe)
    }

    pub fn should_block(&self) -> bool {
        !self.is_safe()
    }
}

/// Types of modification patterns
#[derive(Debug, Clone, Copy)]
pub enum ModificationPattern {
    /// File alternates between two states
    PingPong,
    /// Many modifications in rapid succession
    RapidBurst,
}

/// Current status of a file in the loop detector
#[derive(Debug, Clone)]
pub enum FileLoopStatus {
    /// File is operating normally
    Normal,
    /// File has elevated modification count
    Warning(usize),
    /// File is in cooldown
    InCooldown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_loop_detection() {
        let config = LoopDetectorConfig {
            max_modifications_per_window: 3,
            window_duration_secs: 10,
            ..Default::default()
        };
        
        let mut detector = LoopDetector::new(config);
        let path = PathBuf::from("test.rs");

        // First few modifications should be safe
        assert!(detector.check_modification(&path).unwrap().is_safe());
        assert!(detector.check_modification(&path).unwrap().is_safe());
        assert!(detector.check_modification(&path).unwrap().is_safe());

        // Fourth modification should trigger loop detection
        match detector.check_modification(&path).unwrap() {
            LoopDetectionResult::LoopDetected { modification_count, .. } => {
                assert_eq!(modification_count, 3);
            }
            _ => panic!("Expected loop detection"),
        }

        // Subsequent attempts should be in cooldown
        assert!(matches!(
            detector.check_modification(&path).unwrap(),
            LoopDetectionResult::InCooldown(_)
        ));
    }

    #[test]
    fn test_modification_window() {
        let mut window = ModificationWindow::new(Duration::from_secs(10));
        let now = SystemTime::now();
        
        window.record_modification(now);
        window.record_modification(now + Duration::from_secs(1));
        window.record_modification(now + Duration::from_secs(2));
        
        assert_eq!(window.count_recent_modifications(), 3);
        
        // Old entries should be cleaned up
        window.cleanup_old_entries(now + Duration::from_secs(11));
        assert_eq!(window.count_recent_modifications(), 2);
    }

    #[test]
    fn test_burst_pattern_detection() {
        let mut window = ModificationWindow::new(Duration::from_secs(60));
        let now = SystemTime::now();
        
        // Add rapid modifications
        for i in 0..5 {
            window.record_modification(now + Duration::from_millis(i * 100));
        }
        
        assert!(window.has_burst_pattern());
    }
}