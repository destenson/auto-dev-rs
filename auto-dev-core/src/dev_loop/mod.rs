#![allow(unused)]
//! Continuous monitoring and autonomous development loop module

pub mod control_server;
pub mod decision_engine;
pub mod event_processor;
pub mod health_monitor;
pub mod llm_optimizer;
pub mod orchestrator;
pub mod scheduler;

use chrono::{DateTime, Utc};
use priority_queue::PriorityQueue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

pub use decision_engine::DecisionEngine;
pub use event_processor::EventProcessor;
pub use health_monitor::HealthMonitor;
pub use llm_optimizer::LLMOptimizer;
pub use orchestrator::Orchestrator;

/// Main development loop state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentLoop {
    pub state: LoopState,
    pub metrics: LoopMetrics,
}

/// Current state of the development loop
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LoopState {
    Idle,
    Processing(String), // Task description
    WaitingForValidation,
    RecoveringFromError,
    Shutdown,
}

/// Event in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub priority: Priority,
    pub source: PathBuf,
    pub requires_llm: Option<bool>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Event {}

impl std::hash::Hash for Event {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// Type of event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EventType {
    SpecificationChanged,
    TestAdded,
    TestFailed,
    CodeModified,
    DependencyUpdated,
    ConfigurationChanged,
    HealthCheck,
    UserCommand(String),
}

/// Event priority
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    Critical = 0,
    High = 1,
    Medium = 2,
    Low = 3,
    Background = 4,
}

/// Decision made by the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Decision {
    Implement(ImplementationTask),
    UpdateTests(Vec<TestUpdate>),
    Refactor(RefactorTask),
    Skip(String), // Reason for skipping
    RequiresLLM(LLMRequest),
    UsePattern(String),  // Pattern ID
    UseTemplate(String), // Template ID
    UseCached(CachedResponse),
    AdaptSimilar(SimilarSolution),
}

/// Task to implement specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationTask {
    pub spec_path: PathBuf,
    pub target_path: PathBuf,
    pub requirements: Vec<String>,
    pub incremental: bool,
}

/// Test update task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestUpdate {
    pub test_path: PathBuf,
    pub update_type: TestUpdateType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestUpdateType {
    AddTest(String),
    UpdateAssertion(String),
    AddFixture(String),
    FixFailure(String),
}

/// Refactoring task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactorTask {
    pub file_path: PathBuf,
    pub refactor_type: RefactorType,
    pub scope: RefactorScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefactorType {
    ExtractFunction,
    InlineVariable,
    RenameSymbol,
    OptimizeImports,
    FormatCode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefactorScope {
    File,
    Function(String),
    Module,
    Project,
}

/// LLM request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub context: String,
    pub prompt: String,
    pub model_tier: ModelTier,
    pub max_tokens: Option<usize>,
}

/// Model tier for LLM routing
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModelTier {
    Tier1Pattern,  // Use existing patterns (no LLM)
    Tier2Template, // Use templates with substitution (no LLM)
    Tier3Cached,   // Use cached LLM responses
    Tier4Similar,  // Find similar past solutions (no LLM)
    Tier5LLM,      // Required LLM call
}

/// Cached LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    pub request_hash: String,
    pub response: String,
    pub timestamp: DateTime<Utc>,
    pub usage_count: usize,
}

/// Similar solution found
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarSolution {
    pub solution_id: String,
    pub similarity_score: f32,
    pub solution: String,
    pub adaptations_needed: Vec<String>,
}

/// Loop performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoopMetrics {
    pub events_processed: u64,
    pub llm_calls_made: u64,
    pub llm_calls_avoided: u64,
    pub implementations_completed: u64,
    pub tests_generated: u64,
    pub errors_encountered: u64,
    pub average_event_latency_ms: f64,
    pub llm_cost_saved: f64,
    pub uptime_seconds: u64,
}

/// Health status of the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub memory_usage: f32,
    pub cpu_usage: f32,
    pub disk_space: f32,
    pub llm_quota: f32,
    pub error_rate: f32,
    pub is_healthy: bool,
    pub warnings: Vec<String>,
}

/// Configuration for the loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopConfig {
    pub enabled: bool,
    pub max_concurrent_tasks: usize,
    pub event_debounce_ms: u64,
    pub health_check_interval: u64,
    pub llm_optimization: LLMOptimizationConfig,
    pub recovery: RecoveryConfig,
    pub self_targeting: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMOptimizationConfig {
    pub cache_ttl_hours: u64,
    pub similarity_threshold: f32,
    pub batch_size: usize,
    pub max_context_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryConfig {
    pub max_retries: usize,
    pub backoff_multiplier: f64,
    pub checkpoint_interval: u64,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_concurrent_tasks: 4,
            event_debounce_ms: 500,
            health_check_interval: 60,
            llm_optimization: LLMOptimizationConfig {
                cache_ttl_hours: 24,
                similarity_threshold: 0.85,
                batch_size: 5,
                max_context_tokens: 2000,
            },
            recovery: RecoveryConfig {
                max_retries: 3,
                backoff_multiplier: 2.0,
                checkpoint_interval: 300,
            },
            self_targeting: None,
        }
    }
}

impl Event {
    pub fn new(event_type: EventType, source: PathBuf) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type,
            priority: Priority::Medium,
            source,
            requires_llm: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::new(EventType::SpecificationChanged, PathBuf::from("test.md"))
            .with_priority(Priority::High);

        assert_eq!(event.priority, Priority::High);
        assert_eq!(event.event_type, EventType::SpecificationChanged);
    }

    #[test]
    fn test_loop_config_default() {
        let config = LoopConfig::default();
        assert_eq!(config.max_concurrent_tasks, 4);
        assert_eq!(config.event_debounce_ms, 500);
        assert_eq!(config.llm_optimization.similarity_threshold, 0.85);
    }
}
