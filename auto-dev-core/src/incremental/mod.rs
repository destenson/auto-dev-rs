//! Incremental implementation and progressive enhancement module
//! 
//! This module implements a system for incremental code generation that builds
//! functionality progressively, ensuring each step compiles and passes tests
//! before proceeding to the next.

pub mod planner;
pub mod executor;
pub mod validator;
pub mod rollback;
pub mod progress;

use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use thiserror::Error;

pub use planner::{IncrementPlanner, IncrementPlan};
pub use executor::{IncrementExecutor, ExecutionResult};
pub use validator::{IncrementValidator, ValidationResult};
pub use rollback::{RollbackManager, CheckpointId};
pub use progress::{ProgressTracker, ProgressReport, ProgressEvent};

#[derive(Debug, Error)]
pub enum IncrementalError {
    #[error("Planning failed: {0}")]
    PlanningError(String),
    
    #[error("Execution failed: {0}")]
    ExecutionError(String),
    
    #[error("Validation failed: {0}")]
    ValidationError(String),
    
    #[error("Rollback failed: {0}")]
    RollbackError(String),
    
    #[error("Dependency not satisfied: {0}")]
    DependencyError(String),
    
    #[error("Compilation failed: {0}")]
    CompilationError(String),
    
    #[error("Test failed: {0}")]
    TestFailure(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, IncrementalError>;

/// Represents a single increment of functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Increment {
    pub id: Uuid,
    pub specification: SpecFragment,
    pub dependencies: Vec<Uuid>,
    pub implementation: Implementation,
    pub tests: Vec<TestCase>,
    pub validation: ValidationCriteria,
    pub status: IncrementStatus,
    pub attempts: Vec<Attempt>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A fragment of specification to implement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecFragment {
    pub id: String,
    pub description: String,
    pub requirements: Vec<String>,
    pub context: String,
    pub examples: Vec<String>,
}

/// Implementation details for an increment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    pub files: Vec<FileChange>,
    pub estimated_complexity: Complexity,
    pub approach: String,
    pub language: String,
}

/// File change to apply
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: PathBuf,
    pub change_type: ChangeType,
    pub content: String,
    pub line_range: Option<(usize, usize)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Create,
    Modify,
    Delete,
    Append,
    Replace,
}

/// Test case for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: String,
    pub name: String,
    pub test_type: TestType,
    pub command: String,
    pub expected_outcome: ExpectedOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestType {
    Unit,
    Integration,
    EndToEnd,
    Compilation,
    Lint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExpectedOutcome {
    Success,
    Failure(String),
    Output(String),
    Contains(String),
}

/// Validation criteria for an increment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCriteria {
    pub must_compile: bool,
    pub tests_must_pass: Vec<String>,
    pub performance_criteria: Option<PerformanceCriteria>,
    pub security_checks: Vec<SecurityCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceCriteria {
    pub max_execution_time: std::time::Duration,
    pub max_memory_usage: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityCheck {
    pub name: String,
    pub command: String,
}

/// Status of an increment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncrementStatus {
    Pending,
    InProgress,
    Testing,
    Completed,
    Failed,
    RolledBack,
}

/// Record of an attempt to implement an increment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attempt {
    pub id: Uuid,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub result: Option<AttemptResult>,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttemptResult {
    Success,
    CompilationFailure(String),
    TestFailure(Vec<String>),
    ValidationFailure(String),
    Timeout,
    Rollback(String),
}

/// Complexity assessment
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Complexity {
    Trivial,
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

impl Increment {
    /// Create a new increment
    pub fn new(spec: SpecFragment, dependencies: Vec<Uuid>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            specification: spec,
            dependencies,
            implementation: Implementation {
                files: Vec::new(),
                estimated_complexity: Complexity::Simple,
                approach: String::new(),
                language: "rust".to_string(),
            },
            tests: Vec::new(),
            validation: ValidationCriteria {
                must_compile: true,
                tests_must_pass: Vec::new(),
                performance_criteria: None,
                security_checks: Vec::new(),
            },
            status: IncrementStatus::Pending,
            attempts: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Check if increment is ready to execute
    pub fn is_ready(&self, completed: &[Uuid]) -> bool {
        self.status == IncrementStatus::Pending &&
            self.dependencies.iter().all(|dep| completed.contains(dep))
    }
    
    /// Mark increment as in progress
    pub fn start(&mut self) {
        self.status = IncrementStatus::InProgress;
        self.updated_at = Utc::now();
    }
    
    /// Mark increment as completed
    pub fn complete(&mut self) {
        self.status = IncrementStatus::Completed;
        self.updated_at = Utc::now();
    }
    
    /// Mark increment as failed
    pub fn fail(&mut self, reason: String) {
        self.status = IncrementStatus::Failed;
        self.updated_at = Utc::now();
        if let Some(attempt) = self.attempts.last_mut() {
            attempt.ended_at = Some(Utc::now());
            attempt.result = Some(AttemptResult::ValidationFailure(reason));
        }
    }
    
    /// Add an attempt record
    pub fn add_attempt(&mut self) -> &mut Attempt {
        let attempt = Attempt {
            id: Uuid::new_v4(),
            started_at: Utc::now(),
            ended_at: None,
            result: None,
            logs: Vec::new(),
        };
        self.attempts.push(attempt);
        self.attempts.last_mut().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_increment_creation() {
        let spec = SpecFragment {
            id: "test".to_string(),
            description: "Test increment".to_string(),
            requirements: vec!["Requirement 1".to_string()],
            context: "Test context".to_string(),
            examples: vec![],
        };
        
        let increment = Increment::new(spec, vec![]);
        assert_eq!(increment.status, IncrementStatus::Pending);
        assert!(increment.is_ready(&[]));
    }
    
    #[test]
    fn test_dependency_checking() {
        let spec = SpecFragment {
            id: "test".to_string(),
            description: "Test increment".to_string(),
            requirements: vec![],
            context: String::new(),
            examples: vec![],
        };
        
        let dep_id = Uuid::new_v4();
        let increment = Increment::new(spec, vec![dep_id]);
        
        assert!(!increment.is_ready(&[]));
        assert!(increment.is_ready(&[dep_id]));
    }
}