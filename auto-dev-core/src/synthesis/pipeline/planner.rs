//! Implementation planning stage

use super::{PipelineContext, PipelineStage};
use crate::synthesis::{
    Complexity, ImplementationApproach, Result, SynthesisError, state::ImplementationTask,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;

/// Plans what needs to be implemented
pub struct ImplementationPlanner {
    strategies: HashMap<String, PlanningStrategy>,
}

impl ImplementationPlanner {
    /// Create a new implementation planner
    pub fn new() -> Self {
        Self { strategies: Self::initialize_strategies() }
    }

    fn initialize_strategies() -> HashMap<String, PlanningStrategy> {
        let mut strategies = HashMap::new();

        strategies.insert("api".to_string(), PlanningStrategy::ApiFirst);
        strategies.insert("model".to_string(), PlanningStrategy::ModelFirst);
        strategies.insert("test".to_string(), PlanningStrategy::TestDriven);
        strategies.insert("incremental".to_string(), PlanningStrategy::Incremental);

        strategies
    }

    /// Create implementation plan from specification
    fn create_plan(&self, context: &PipelineContext) -> Plan {
        let mut tasks = Vec::new();
        let spec = &context.spec;

        // Plan tasks for requirements
        for requirement in &spec.requirements {
            let task = ImplementationTask::new(
                requirement.id.clone(),
                requirement.description.clone(),
                self.determine_target_file(&requirement.description),
            );
            tasks.push(task);
        }

        // Plan tasks for APIs
        for (idx, api) in spec.apis.iter().enumerate() {
            let task = ImplementationTask::new(
                format!("api_{}", idx),
                format!("Implement API endpoint: {} {}", api.method, api.endpoint),
                PathBuf::from("src/api.rs"),
            );
            tasks.push(task);
        }

        // Plan tasks for data models
        for model in &spec.data_models {
            let task = ImplementationTask::new(
                format!("model_{}", model.name),
                format!("Implement data model: {}", model.name),
                PathBuf::from("src/models.rs"),
            );
            tasks.push(task);
        }

        // Determine complexity and approach
        let complexity = self.assess_complexity(&tasks);
        let approach = self.determine_approach(&spec, &complexity);

        // Build dependency graph
        let dependencies = self.build_dependencies(&tasks);

        Plan { tasks, dependencies, estimated_complexity: complexity, approach }
    }

    /// Determine target file for implementation
    fn determine_target_file(&self, description: &str) -> PathBuf {
        let lower = description.to_lowercase();

        if lower.contains("api") || lower.contains("endpoint") {
            PathBuf::from("src/api.rs")
        } else if lower.contains("model") || lower.contains("schema") {
            PathBuf::from("src/models.rs")
        } else if lower.contains("test") {
            PathBuf::from("tests/integration.rs")
        } else {
            PathBuf::from("src/lib.rs")
        }
    }

    /// Assess overall complexity
    fn assess_complexity(&self, tasks: &[ImplementationTask]) -> Complexity {
        match tasks.len() {
            0..=2 => Complexity::Trivial,
            3..=5 => Complexity::Simple,
            6..=10 => Complexity::Moderate,
            11..=20 => Complexity::Complex,
            _ => Complexity::VeryComplex,
        }
    }

    /// Determine implementation approach
    fn determine_approach(
        &self,
        spec: &crate::parser::model::Specification,
        complexity: &Complexity,
    ) -> ImplementationApproach {
        // If spec has behaviors (tests), use test-driven
        if !spec.behaviors.is_empty() {
            return ImplementationApproach::TestDriven;
        }

        // For complex projects, use incremental
        if matches!(complexity, Complexity::Complex | Complexity::VeryComplex) {
            return ImplementationApproach::Incremental;
        }

        // Default to incremental for safety
        ImplementationApproach::Incremental
    }

    /// Build task dependencies
    fn build_dependencies(&self, tasks: &[ImplementationTask]) -> HashMap<String, Vec<String>> {
        let mut deps = HashMap::new();

        // Simple dependency detection based on task types
        for task in tasks {
            let mut task_deps = Vec::new();

            // APIs depend on models
            if task.id.starts_with("api_") {
                for other in tasks {
                    if other.id.starts_with("model_") {
                        task_deps.push(other.id.clone());
                    }
                }
            }

            // Tests depend on implementations
            if task.description.to_lowercase().contains("test") {
                for other in tasks {
                    if !other.description.to_lowercase().contains("test") {
                        task_deps.push(other.id.clone());
                    }
                }
            }

            if !task_deps.is_empty() {
                deps.insert(task.id.clone(), task_deps);
            }
        }

        deps
    }
}

#[async_trait]
impl PipelineStage for ImplementationPlanner {
    fn name(&self) -> &'static str {
        "ImplementationPlanner"
    }

    async fn execute(&self, mut context: PipelineContext) -> Result<PipelineContext> {
        tracing::info!("Planning implementation for: {}", context.spec.source.display());

        context.metadata.current_stage = self.name().to_string();

        // Create implementation plan
        let plan = self.create_plan(&context);

        // Update context with planned tasks
        context.pending_tasks = plan.tasks;
        context.metadata.complexity = plan.estimated_complexity;

        // Record planning decision
        let decision = crate::synthesis::ArchitectureDecision {
            id: format!("plan_{}", chrono::Utc::now().timestamp()),
            title: "Implementation Plan".to_string(),
            status: crate::synthesis::DecisionStatus::Accepted,
            context: format!(
                "Planning implementation for {} requirements, {} APIs, {} models",
                context.spec.requirements.len(),
                context.spec.apis.len(),
                context.spec.data_models.len()
            ),
            decision: format!(
                "Using {:?} approach with {:?} complexity. {} tasks planned.",
                plan.approach,
                plan.estimated_complexity,
                context.pending_tasks.len()
            ),
            alternatives: vec![],
            consequences: "Structured implementation with clear task ordering".to_string(),
            timestamp: chrono::Utc::now(),
        };

        context.record_decision(decision);

        tracing::debug!(
            "Planned {} tasks with {:?} complexity",
            context.pending_tasks.len(),
            plan.estimated_complexity
        );

        Ok(context)
    }
}

/// Planning strategy enum
enum PlanningStrategy {
    ApiFirst,
    ModelFirst,
    TestDriven,
    Incremental,
}

/// Implementation plan
struct Plan {
    tasks: Vec<ImplementationTask>,
    dependencies: HashMap<String, Vec<String>>,
    estimated_complexity: Complexity,
    approach: ImplementationApproach,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complexity_assessment() {
        let planner = ImplementationPlanner::new();

        let mut tasks = vec![];
        assert_eq!(planner.assess_complexity(&tasks), Complexity::Trivial);

        for i in 0..5 {
            tasks.push(ImplementationTask::new(
                format!("task_{}", i),
                format!("Task {}", i),
                PathBuf::from("test.rs"),
            ));
        }
        assert_eq!(planner.assess_complexity(&tasks), Complexity::Simple);

        for i in 5..15 {
            tasks.push(ImplementationTask::new(
                format!("task_{}", i),
                format!("Task {}", i),
                PathBuf::from("test.rs"),
            ));
        }
        assert_eq!(planner.assess_complexity(&tasks), Complexity::Complex);
    }
}
