//! Synthesis pipeline stages

pub mod analyzer;
pub mod planner;
pub mod generator;
pub mod merger;
pub mod validator;

use super::{Result, ArchitectureDecision, Complexity};
use crate::parser::model::Specification;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use async_trait::async_trait;

/// Pipeline stage trait that all stages must implement
#[async_trait]
pub trait PipelineStage: Send + Sync {
    /// Name of the stage
    fn name(&self) -> &'static str;
    
    /// Execute the stage
    async fn execute(&self, context: PipelineContext) -> Result<PipelineContext>;
}

/// Context passed through the pipeline
#[derive(Debug, Clone)]
pub struct PipelineContext {
    pub spec: Specification,
    pub config: super::engine::SynthesisConfig,
    pub generated_files: Vec<PathBuf>,
    pub modified_files: Vec<PathBuf>,
    pub completed_tasks: Vec<super::state::ImplementationTask>,
    pub pending_tasks: Vec<super::state::ImplementationTask>,
    pub decisions: Vec<ArchitectureDecision>,
    pub coverage: super::coverage::CoverageReport,
    pub warnings: Vec<String>,
    pub metadata: PipelineMetadata,
}

impl PipelineContext {
    /// Create a new pipeline context
    pub fn new(spec: Specification, config: super::engine::SynthesisConfig) -> Self {
        Self {
            spec,
            config,
            generated_files: Vec::new(),
            modified_files: Vec::new(),
            completed_tasks: Vec::new(),
            pending_tasks: Vec::new(),
            decisions: Vec::new(),
            coverage: super::coverage::CoverageReport::new(),
            warnings: Vec::new(),
            metadata: PipelineMetadata::new(),
        }
    }
    
    /// Add a generated file
    pub fn add_generated_file(&mut self, path: PathBuf) {
        self.generated_files.push(path);
    }
    
    /// Add a modified file
    pub fn add_modified_file(&mut self, path: PathBuf) {
        self.modified_files.push(path);
    }
    
    /// Add a warning
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
    
    /// Record a decision
    pub fn record_decision(&mut self, decision: ArchitectureDecision) {
        self.decisions.push(decision);
    }
    
    /// Get current status
    pub fn get_status(&self) -> super::state::SpecificationStatus {
        super::state::SpecificationStatus {
            path: self.spec.source.clone(),
            status: if self.completed_tasks.is_empty() {
                super::state::ProcessingStatus::Pending
            } else if self.pending_tasks.is_empty() {
                super::state::ProcessingStatus::Completed
            } else {
                super::state::ProcessingStatus::InProgress
            },
            requirements_total: self.spec.requirements.len(),
            requirements_implemented: self.completed_tasks.len(),
            last_processed: chrono::Utc::now(),
        }
    }
}

/// Metadata for pipeline execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineMetadata {
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub current_stage: String,
    pub complexity: Complexity,
    pub estimated_tokens: usize,
    pub actual_tokens: usize,
}

impl PipelineMetadata {
    pub fn new() -> Self {
        Self {
            start_time: chrono::Utc::now(),
            current_stage: String::new(),
            complexity: Complexity::Simple,
            estimated_tokens: 0,
            actual_tokens: 0,
        }
    }
}