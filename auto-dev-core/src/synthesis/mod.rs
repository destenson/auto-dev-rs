//! Code synthesis and implementation engine
//! 
//! Orchestrates the transformation of specifications into working code,
//! managing the entire synthesis pipeline from requirement analysis to
//! code generation and integration.

pub mod engine;
pub mod pipeline;
pub mod state;
pub mod coverage;

pub use engine::{SynthesisEngine, SynthesisConfig};
pub use state::{SynthesisState, ImplementationTask, TaskStatus};
pub use coverage::{CoverageReport, SpecificationStatus};
pub use pipeline::{
    analyzer::CodeAnalyzer,
    planner::ImplementationPlanner,
    generator::CodeGenerator,
    merger::CodeMerger,
    validator::ImplementationValidator,
};

use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SynthesisError {
    #[error("Failed to analyze existing code: {0}")]
    AnalysisError(String),
    
    #[error("Failed to plan implementation: {0}")]
    PlanningError(String),
    
    #[error("Failed to generate code: {0}")]
    GenerationError(String),
    
    #[error("Failed to merge code: {0}")]
    MergeError(String),
    
    #[error("Validation failed: {0}")]
    ValidationError(String),
    
    #[error("State management error: {0}")]
    StateError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, SynthesisError>;

/// Result of a synthesis operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisResult {
    pub files_generated: Vec<PathBuf>,
    pub files_modified: Vec<PathBuf>,
    pub tasks_completed: Vec<String>,
    pub coverage: CoverageReport,
    pub decisions_made: Vec<ArchitectureDecision>,
    pub warnings: Vec<String>,
}

/// Architecture decision record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureDecision {
    pub id: String,
    pub title: String,
    pub status: DecisionStatus,
    pub context: String,
    pub decision: String,
    pub alternatives: Vec<Alternative>,
    pub consequences: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionStatus {
    Proposed,
    Accepted,
    Deprecated,
    Superseded { by: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    pub name: String,
    pub reason_not_chosen: String,
}

/// Complexity assessment for planning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Complexity {
    Trivial,
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

/// Implementation approach strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImplementationApproach {
    TestDriven,
    Incremental,
    BigBang,
    Prototype,
    Refactor,
}