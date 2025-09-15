//! Synthesis state management

use super::{Result, SynthesisError, ArchitectureDecision};
use crate::parser::model::Specification;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Current state of the synthesis process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisState {
    pub specifications: HashMap<PathBuf, SpecificationStatus>,
    pub implementations: HashMap<PathBuf, ImplementationStatus>,
    pub coverage: super::coverage::CoverageReport,
    pub pending_tasks: Vec<ImplementationTask>,
    pub completed_tasks: Vec<ImplementationTask>,
    pub decisions: Vec<ArchitectureDecision>,
    pub checkpoints: Vec<Checkpoint>,
    pub last_updated: DateTime<Utc>,
}

impl SynthesisState {
    /// Create a new synthesis state
    pub fn new() -> Self {
        Self {
            specifications: HashMap::new(),
            implementations: HashMap::new(),
            coverage: super::coverage::CoverageReport::new(),
            pending_tasks: Vec::new(),
            completed_tasks: Vec::new(),
            decisions: Vec::new(),
            checkpoints: Vec::new(),
            last_updated: Utc::now(),
        }
    }
    
    /// Load state from disk or create new
    pub fn load_or_create(state_dir: &Path) -> Result<Self> {
        let state_file = state_dir.join("synthesis_state.json");
        
        if state_file.exists() {
            let content = std::fs::read_to_string(&state_file)?;
            let state: Self = serde_json::from_str(&content)?;
            Ok(state)
        } else {
            Ok(Self::new())
        }
    }
    
    /// Save state to disk
    pub fn save(&self, state_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(state_dir)?;
        let state_file = state_dir.join("synthesis_state.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(state_file, content)?;
        Ok(())
    }
    
    /// Update specification status
    pub fn update_specification_status(&mut self, path: &Path, status: SpecificationStatus) {
        self.specifications.insert(path.to_path_buf(), status);
        self.last_updated = Utc::now();
    }
    
    /// Add a pending task
    pub fn add_task(&mut self, task: ImplementationTask) {
        self.pending_tasks.push(task);
        self.last_updated = Utc::now();
    }
    
    /// Mark task as completed
    pub fn mark_task_completed(&mut self, task: ImplementationTask) {
        self.pending_tasks.retain(|t| t.id != task.id);
        self.completed_tasks.push(task);
        self.last_updated = Utc::now();
    }
    
    /// Record an architecture decision
    pub fn record_decision(&mut self, decision: ArchitectureDecision) {
        self.decisions.push(decision);
        self.last_updated = Utc::now();
    }
    
    /// Update coverage report
    pub fn update_coverage(&mut self, coverage: &super::coverage::CoverageReport) {
        self.coverage = coverage.clone();
        self.last_updated = Utc::now();
    }
    
    /// Create a checkpoint for rollback
    pub fn checkpoint(&mut self, description: String) -> String {
        let checkpoint = Checkpoint {
            id: Uuid::new_v4().to_string(),
            description,
            state: Box::new(self.clone()),
            created_at: Utc::now(),
        };
        
        let id = checkpoint.id.clone();
        self.checkpoints.push(checkpoint);
        id
    }
    
    /// Rollback to a checkpoint
    pub fn rollback(&mut self, checkpoint_id: &str) -> Result<()> {
        let checkpoint = self.checkpoints.iter()
            .find(|c| c.id == checkpoint_id)
            .ok_or_else(|| SynthesisError::StateError(
                format!("Checkpoint {} not found", checkpoint_id)
            ))?;
        
        *self = *checkpoint.state.clone();
        Ok(())
    }
    
    /// Get implementation progress percentage
    pub fn get_progress(&self) -> f32 {
        let total = self.pending_tasks.len() + self.completed_tasks.len();
        if total == 0 {
            return 0.0;
        }
        
        (self.completed_tasks.len() as f32 / total as f32) * 100.0
    }
}

/// Status of a specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecificationStatus {
    pub path: PathBuf,
    pub status: ProcessingStatus,
    pub requirements_total: usize,
    pub requirements_implemented: usize,
    pub last_processed: DateTime<Utc>,
}

impl SpecificationStatus {
    pub fn is_complete(&self) -> bool {
        matches!(self.status, ProcessingStatus::Completed)
    }
}

/// Status of an implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationStatus {
    pub file: PathBuf,
    pub status: ProcessingStatus,
    pub functions_added: Vec<String>,
    pub functions_modified: Vec<String>,
    pub lines_added: usize,
    pub lines_modified: usize,
    pub last_modified: DateTime<Utc>,
}

/// Processing status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingStatus {
    Pending,
    InProgress,
    Completed,
    Failed { reason: String },
    Skipped { reason: String },
}

/// Implementation task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationTask {
    pub id: String,
    pub spec_id: String,
    pub description: String,
    pub target_file: PathBuf,
    pub status: TaskStatus,
    pub attempts: Vec<GenerationAttempt>,
    pub dependencies: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl ImplementationTask {
    /// Create a new implementation task
    pub fn new(spec_id: String, description: String, target_file: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            spec_id,
            description,
            target_file,
            status: TaskStatus::Pending,
            attempts: Vec::new(),
            dependencies: Vec::new(),
            created_at: Utc::now(),
            completed_at: None,
        }
    }
    
    /// Mark task as completed
    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
        self.completed_at = Some(Utc::now());
    }
    
    /// Add a generation attempt
    pub fn add_attempt(&mut self, attempt: GenerationAttempt) {
        self.attempts.push(attempt);
    }
}

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Blocked,
}

/// Generation attempt record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationAttempt {
    pub timestamp: DateTime<Utc>,
    pub model: String,
    pub success: bool,
    pub error: Option<String>,
    pub tokens_used: usize,
}

/// Checkpoint for rollback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub description: String,
    pub state: Box<SynthesisState>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_state_creation() {
        let state = SynthesisState::new();
        assert!(state.specifications.is_empty());
        assert!(state.pending_tasks.is_empty());
    }
    
    #[test]
    fn test_state_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let state = SynthesisState::new();
        
        assert!(state.save(temp_dir.path()).is_ok());
        
        let loaded = SynthesisState::load_or_create(temp_dir.path()).unwrap();
        assert!(loaded.specifications.is_empty());
    }
    
    #[test]
    fn test_checkpoint_rollback() {
        let mut state = SynthesisState::new();
        
        let task = ImplementationTask::new(
            "spec1".to_string(),
            "Test task".to_string(),
            PathBuf::from("test.rs"),
        );
        
        state.add_task(task.clone());
        let checkpoint_id = state.checkpoint("Before completion".to_string());
        
        state.mark_task_completed(task);
        assert_eq!(state.completed_tasks.len(), 1);
        assert_eq!(state.pending_tasks.len(), 0);
        
        state.rollback(&checkpoint_id).unwrap();
        assert_eq!(state.completed_tasks.len(), 0);
        assert_eq!(state.pending_tasks.len(), 1);
    }
}