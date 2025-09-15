//! Main synthesis orchestrator

use super::{
    ArchitectureDecision, Result, SynthesisError, SynthesisResult,
    pipeline::{PipelineContext, PipelineStage},
    state::SynthesisState,
};
use crate::parser::model::Specification;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for the synthesis engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisConfig {
    /// Enable incremental synthesis
    pub incremental: bool,

    /// Maximum size of incremental changes (lines)
    pub max_increment_size: usize,

    /// Test-first development approach
    pub test_first: bool,

    /// Cache generated code
    pub cache_generations: bool,

    /// State persistence directory
    pub state_dir: PathBuf,

    /// Enable rollback capability
    pub enable_rollback: bool,

    /// Parallelization level
    pub parallel_tasks: usize,
}

impl Default for SynthesisConfig {
    fn default() -> Self {
        Self {
            incremental: true,
            max_increment_size: 50,
            test_first: true,
            cache_generations: true,
            state_dir: PathBuf::from(".auto-dev/synthesis"),
            enable_rollback: true,
            parallel_tasks: 4,
        }
    }
}

/// Main synthesis engine that orchestrates code generation
pub struct SynthesisEngine {
    config: SynthesisConfig,
    state: Arc<RwLock<SynthesisState>>,
    pipeline: Vec<Box<dyn PipelineStage>>,
}

impl SynthesisEngine {
    /// Create a new synthesis engine
    pub fn new(config: SynthesisConfig) -> Result<Self> {
        let state = SynthesisState::load_or_create(&config.state_dir)?;

        Ok(Self { config, state: Arc::new(RwLock::new(state)), pipeline: Self::build_pipeline() })
    }

    /// Build the default pipeline
    fn build_pipeline() -> Vec<Box<dyn PipelineStage>> {
        vec![
            Box::new(super::pipeline::analyzer::CodeAnalyzer::new()),
            Box::new(super::pipeline::planner::ImplementationPlanner::new()),
            Box::new(super::pipeline::generator::CodeGenerator::new()),
            Box::new(super::pipeline::merger::CodeMerger::new()),
            Box::new(super::pipeline::validator::ImplementationValidator::new()),
        ]
    }

    /// Synthesize code from a specification
    pub async fn synthesize(&mut self, spec: &Specification) -> Result<SynthesisResult> {
        tracing::info!("Starting synthesis for specification: {}", spec.source.display());

        // Create pipeline context
        let mut context = PipelineContext::new(spec.clone(), self.config.clone());

        // Execute pipeline stages
        for stage in &self.pipeline {
            tracing::debug!("Executing stage: {}", stage.name());
            context = stage.execute(context).await?;

            // Update state after each stage
            self.update_state(&context).await?;
        }

        // Build result from context
        let result = self.build_result(context).await?;

        // Persist state
        self.persist_state().await?;

        Ok(result)
    }

    /// Synthesize multiple specifications
    pub async fn synthesize_all(
        &mut self,
        specs: Vec<Specification>,
    ) -> Result<Vec<SynthesisResult>> {
        let mut results = Vec::new();

        for spec in specs {
            match self.synthesize(&spec).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::error!("Failed to synthesize {}: {}", spec.source.display(), e);
                    if !self.config.incremental {
                        return Err(e);
                    }
                }
            }
        }

        Ok(results)
    }

    /// Get the current synthesis status
    pub async fn status(&self) -> Result<SynthesisStatus> {
        let state = self.state.read().await;

        Ok(SynthesisStatus {
            total_specifications: state.specifications.len(),
            completed_specifications: state
                .specifications
                .values()
                .filter(|s| s.is_complete())
                .count(),
            pending_tasks: state.pending_tasks.len(),
            completed_tasks: state.completed_tasks.len(),
            coverage: state.coverage.clone(),
        })
    }

    /// Rollback to a previous state
    pub async fn rollback(&mut self, checkpoint_id: &str) -> Result<()> {
        if !self.config.enable_rollback {
            return Err(SynthesisError::StateError("Rollback is not enabled".to_string()));
        }

        let mut state = self.state.write().await;
        state.rollback(checkpoint_id)?;

        Ok(())
    }

    /// Update internal state from pipeline context
    async fn update_state(&self, context: &PipelineContext) -> Result<()> {
        let mut state = self.state.write().await;

        // Update specification status
        state.update_specification_status(&context.spec.source, context.get_status());

        // Add completed tasks
        for task in &context.completed_tasks {
            state.mark_task_completed(task.clone());
        }

        // Record decisions
        for decision in &context.decisions {
            state.record_decision(decision.clone());
        }

        // Update coverage
        state.update_coverage(&context.coverage);

        Ok(())
    }

    /// Build synthesis result from context
    async fn build_result(&self, context: PipelineContext) -> Result<SynthesisResult> {
        Ok(SynthesisResult {
            files_generated: context.generated_files,
            files_modified: context.modified_files,
            tasks_completed: context
                .completed_tasks
                .iter()
                .map(|t| t.description.clone())
                .collect(),
            coverage: context.coverage,
            decisions_made: context.decisions,
            warnings: context.warnings,
        })
    }

    /// Persist current state to disk
    async fn persist_state(&self) -> Result<()> {
        let state = self.state.read().await;
        state.save(&self.config.state_dir)?;
        Ok(())
    }
}

/// Synthesis status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisStatus {
    pub total_specifications: usize,
    pub completed_specifications: usize,
    pub pending_tasks: usize,
    pub completed_tasks: usize,
    pub coverage: super::coverage::CoverageReport,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_engine_creation() {
        let config = SynthesisConfig::default();
        let engine = SynthesisEngine::new(config);
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_status_reporting() {
        let config = SynthesisConfig::default();
        let engine = SynthesisEngine::new(config).unwrap();
        let status = engine.status().await;
        assert!(status.is_ok());
    }
}
