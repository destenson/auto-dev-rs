#![allow(unused)]
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::collections::VecDeque;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use tracing::{debug, info, warn};

/// Maintains an audit trail of all self-modifications
pub struct AuditTrail {
    /// In-memory buffer of recent entries
    entries: VecDeque<AuditEntry>,
    /// Path to the audit log file
    log_file: PathBuf,
    /// Maximum entries to keep in memory
    max_memory_entries: usize,
    /// Configuration
    config: AuditConfig,
}

/// Configuration for audit trail
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// Directory where audit logs are stored
    pub log_directory: PathBuf,
    /// Maximum size of a single log file (in bytes)
    pub max_log_size: usize,
    /// Number of backup log files to keep
    pub backup_count: usize,
    /// Include file content diffs in audit
    pub include_diffs: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            log_directory: PathBuf::from(".auto-dev/audit"),
            max_log_size: 10_000_000, // 10MB
            backup_count: 5,
            include_diffs: true,
        }
    }
}

/// Single audit trail entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique ID for this entry
    pub id: String,
    /// Timestamp of the modification
    pub timestamp: SystemTime,
    /// File that was modified
    pub file_path: PathBuf,
    /// Type of modification
    pub action: AuditAction,
    /// Who/what initiated the modification
    pub initiator: ModificationInitiator,
    /// Result of the modification attempt
    pub result: ModificationResult,
    /// Additional metadata
    pub metadata: Option<AuditMetadata>,
}

/// Types of modification actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    Create,
    Update,
    Delete,
    Rename { from: PathBuf, to: PathBuf },
    PermissionChange,
}

/// Who initiated the modification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModificationInitiator {
    /// Auto-dev system itself
    System { component: String },
    /// User action
    User { username: Option<String> },
    /// External tool or IDE
    External { tool: String },
    /// Unknown source
    Unknown,
}

/// Result of modification attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModificationResult {
    Success,
    Blocked { reason: String },
    Failed { error: String },
    RequiresReview { reason: String },
}

/// Additional metadata for audit entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditMetadata {
    /// Size of file before modification
    pub size_before: Option<usize>,
    /// Size of file after modification
    pub size_after: Option<usize>,
    /// Hash of content before
    pub hash_before: Option<String>,
    /// Hash of content after
    pub hash_after: Option<String>,
    /// Content diff if available
    pub diff: Option<String>,
    /// Any safety validations performed
    pub validations: Vec<String>,
}

impl AuditTrail {
    pub fn new(config: AuditConfig) -> Result<Self> {
        // Ensure log directory exists
        fs::create_dir_all(&config.log_directory)?;
        
        let log_file = config.log_directory.join("audit.log");
        
        Ok(Self {
            entries: VecDeque::with_capacity(1000),
            log_file,
            max_memory_entries: 1000,
            config,
        })
    }

    /// Record a new audit entry
    pub fn record(&mut self, entry: AuditEntry) -> Result<()> {
        // Add to memory buffer
        self.entries.push_back(entry.clone());
        
        // Trim memory buffer if needed
        if self.entries.len() > self.max_memory_entries {
            self.entries.pop_front();
        }

        // Write to file
        self.write_to_file(&entry)?;
        
        // Check if rotation is needed
        self.rotate_if_needed()?;
        
        info!("Audit entry recorded: {:?} on {:?}", entry.action, entry.file_path);
        
        Ok(())
    }

    /// Create an audit entry for a file modification
    pub fn create_entry(
        &self,
        path: &Path,
        action: AuditAction,
        initiator: ModificationInitiator,
        result: ModificationResult,
    ) -> AuditEntry {
        AuditEntry {
            id: Self::generate_id(),
            timestamp: SystemTime::now(),
            file_path: path.to_path_buf(),
            action,
            initiator,
            result,
            metadata: None,
        }
    }

    /// Add metadata to an entry
    pub fn add_metadata(&self, mut entry: AuditEntry, metadata: AuditMetadata) -> AuditEntry {
        entry.metadata = Some(metadata);
        entry
    }

    /// Write entry to log file
    fn write_to_file(&self, entry: &AuditEntry) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)
            .context("Failed to open audit log file")?;

        let json = serde_json::to_string(entry)?;
        writeln!(file, "{}", json)?;
        
        Ok(())
    }

    /// Rotate log file if it's too large
    fn rotate_if_needed(&self) -> Result<()> {
        let metadata = fs::metadata(&self.log_file)?;
        
        if metadata.len() > self.config.max_log_size as u64 {
            self.rotate_logs()?;
        }
        
        Ok(())
    }

    /// Rotate log files
    fn rotate_logs(&self) -> Result<()> {
        // Move existing backups
        for i in (1..self.config.backup_count).rev() {
            let from = self.config.log_directory.join(format!("audit.log.{}", i));
            let to = self.config.log_directory.join(format!("audit.log.{}", i + 1));
            
            if from.exists() {
                fs::rename(from, to)?;
            }
        }

        // Move current log to .1
        let backup = self.config.log_directory.join("audit.log.1");
        fs::rename(&self.log_file, backup)?;
        
        info!("Audit log rotated");
        Ok(())
    }

    /// Generate unique ID for entries
    fn generate_id() -> String {
        use std::time::UNIX_EPOCH;
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        
        format!("{}-{:x}", timestamp.as_secs(), rand::random::<u32>())
    }

    /// Query audit entries
    pub fn query(&self, filter: AuditFilter) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|entry| filter.matches(entry))
            .collect()
    }

    /// Get recent entries
    pub fn get_recent(&self, count: usize) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .rev()
            .take(count)
            .collect()
    }

    /// Export audit trail to file
    pub fn export(&self, path: &Path) -> Result<()> {
        let entries: Vec<_> = self.entries.iter().cloned().collect();
        let json = serde_json::to_string_pretty(&entries)?;
        fs::write(path, json)?;
        
        info!("Audit trail exported to {:?}", path);
        Ok(())
    }

    /// Clear old entries from memory
    pub fn cleanup_old_entries(&mut self, max_age: std::time::Duration) {
        let cutoff = SystemTime::now() - max_age;
        
        self.entries.retain(|entry| entry.timestamp > cutoff);
    }
}

/// Filter for querying audit entries
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub file_path: Option<PathBuf>,
    pub action_type: Option<AuditAction>,
    pub result_type: Option<ModificationResult>,
    pub since: Option<SystemTime>,
    pub until: Option<SystemTime>,
}

impl AuditFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.file_path = Some(path);
        self
    }

    pub fn with_action(mut self, action: AuditAction) -> Self {
        self.action_type = Some(action);
        self
    }

    pub fn since(mut self, time: SystemTime) -> Self {
        self.since = Some(time);
        self
    }

    fn matches(&self, entry: &AuditEntry) -> bool {
        if let Some(ref path) = self.file_path {
            if entry.file_path != *path {
                return false;
            }
        }

        if let Some(ref since) = self.since {
            if entry.timestamp < *since {
                return false;
            }
        }

        if let Some(ref until) = self.until {
            if entry.timestamp > *until {
                return false;
            }
        }

        true
    }
}

/// Generate a summary of audit activity
pub struct AuditSummary {
    pub total_modifications: usize,
    pub successful: usize,
    pub blocked: usize,
    pub failed: usize,
    pub files_modified: Vec<PathBuf>,
    pub time_range: (SystemTime, SystemTime),
}

impl AuditSummary {
    pub fn from_entries(entries: &[AuditEntry]) -> Self {
        let mut summary = Self {
            total_modifications: entries.len(),
            successful: 0,
            blocked: 0,
            failed: 0,
            files_modified: Vec::new(),
            time_range: (SystemTime::UNIX_EPOCH, SystemTime::UNIX_EPOCH),
        };

        if entries.is_empty() {
            return summary;
        }

        let mut files = std::collections::HashSet::new();
        let mut min_time = entries[0].timestamp;
        let mut max_time = entries[0].timestamp;

        for entry in entries {
            files.insert(entry.file_path.clone());
            
            if entry.timestamp < min_time {
                min_time = entry.timestamp;
            }
            if entry.timestamp > max_time {
                max_time = entry.timestamp;
            }

            match &entry.result {
                ModificationResult::Success => summary.successful += 1,
                ModificationResult::Blocked { .. } => summary.blocked += 1,
                ModificationResult::Failed { .. } => summary.failed += 1,
                ModificationResult::RequiresReview { .. } => {}
            }
        }

        summary.files_modified = files.into_iter().collect();
        summary.time_range = (min_time, max_time);
        
        summary
    }
}

// Note: Using a placeholder for rand, should be replaced with actual random generation
mod rand {
    pub fn random<T>() -> T
    where
        T: Default,
    {
        T::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_audit_entry_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = AuditConfig {
            log_directory: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let trail = AuditTrail::new(config).unwrap();
        let entry = trail.create_entry(
            &PathBuf::from("test.rs"),
            AuditAction::Update,
            ModificationInitiator::System { 
                component: "self_monitor".to_string() 
            },
            ModificationResult::Success,
        );
        
        assert_eq!(entry.file_path, PathBuf::from("test.rs"));
        assert!(matches!(entry.action, AuditAction::Update));
        assert!(matches!(entry.result, ModificationResult::Success));
    }

    #[test]
    fn test_audit_recording() {
        let temp_dir = TempDir::new().unwrap();
        let config = AuditConfig {
            log_directory: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let mut trail = AuditTrail::new(config).unwrap();
        
        let entry = trail.create_entry(
            &PathBuf::from("test.rs"),
            AuditAction::Create,
            ModificationInitiator::User { username: None },
            ModificationResult::Success,
        );
        
        trail.record(entry.clone()).unwrap();
        
        let recent = trail.get_recent(1);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].file_path, PathBuf::from("test.rs"));
    }

    #[test]
    fn test_audit_filter() {
        let temp_dir = TempDir::new().unwrap();
        let config = AuditConfig {
            log_directory: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let mut trail = AuditTrail::new(config).unwrap();
        
        // Add multiple entries
        for i in 0..5 {
            let entry = trail.create_entry(
                &PathBuf::from(format!("test{}.rs", i)),
                AuditAction::Update,
                ModificationInitiator::System { 
                    component: "test".to_string() 
                },
                ModificationResult::Success,
            );
            trail.record(entry).unwrap();
        }
        
        // Query with filter
        let filter = AuditFilter::new()
            .with_path(PathBuf::from("test2.rs"));
        
        let results = trail.query(filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].file_path, PathBuf::from("test2.rs"));
    }
}
