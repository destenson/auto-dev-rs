//! Cost and performance optimization for model routing

use crate::llm::provider::ModelTier;
use crate::llm::router::{
    cost_tracker::{CostStats, CostStrategy},
    performance::{ModelMetrics, PerformanceReport},
    registry::ModelConfig,
};
use anyhow::Result;
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

/// Cost optimizer for intelligent routing decisions
pub struct CostOptimizer {
    config: OptimizerConfig,
}

impl CostOptimizer {
    pub fn new(config: OptimizerConfig) -> Self {
        Self { config }
    }

    /// Select optimal model based on cost and performance
    pub fn select_optimal_model(
        &self,
        models: &[ModelConfig],
        strategy: CostStrategy,
        performance_data: Option<&HashMap<String, ModelMetrics>>,
        task_requirements: &TaskRequirements,
    ) -> Option<ModelConfig> {
        let mut candidates: Vec<_> = models
            .iter()
            .filter(|m| m.available)
            .filter(|m| self.meets_requirements(m, task_requirements))
            .collect();

        if candidates.is_empty() {
            return None;
        }

        // Sort based on strategy
        match strategy {
            CostStrategy::QualityFirst => {
                // Prefer higher tier models
                candidates.sort_by(|a, b| b.tier.cmp(&a.tier));
            }
            CostStrategy::Balanced => {
                // Score based on cost and performance
                candidates.sort_by_key(|m| {
                    let cost_score = (m.cost_per_1k_tokens * 1000.0) as i64;
                    let perf_score = self.get_performance_score(&m.id, performance_data);
                    let balanced_score = cost_score + (100.0 * (1.0 - perf_score)) as i64;
                    balanced_score
                });
            }
            CostStrategy::CostOptimized => {
                // Prefer cheaper models
                candidates.sort_by(|a, b| {
                    a.cost_per_1k_tokens.partial_cmp(&b.cost_per_1k_tokens).unwrap()
                });
            }
            CostStrategy::PreferLocal => {
                // Sort local models first, then by cost
                candidates.sort_by_key(|m| {
                    let local_priority = if m.is_local() { 0 } else { 1000 };
                    let cost_score = (m.cost_per_1k_tokens * 1000.0) as i64;
                    local_priority + cost_score
                });
            }
            CostStrategy::UltraFrugal => {
                // Only consider free models
                candidates.retain(|m| m.cost_per_1k_tokens == 0.0);
                if candidates.is_empty() {
                    return None;
                }
                // Among free models, prefer fastest
                candidates.sort_by_key(|m| m.average_latency_ms);
            }
        }

        candidates.first().map(|m| (*m).clone())
    }

    /// Calculate routing strategy based on multiple factors
    pub fn calculate_routing_strategy(
        &self,
        cost_stats: &CostStats,
        performance_report: &PerformanceReport,
    ) -> RoutingStrategy {
        let budget_usage = cost_stats.spent_today / cost_stats.daily_budget;
        let success_rate = performance_report.overall_success_rate;

        let mut strategy = RoutingStrategy {
            primary_tier: ModelTier::Small,
            fallback_tier: ModelTier::Medium,
            cost_strategy: CostStrategy::Balanced,
            quality_threshold: 0.7,
            max_latency_ms: 5000,
            prefer_cached: true,
            enable_batching: false,
            parallel_attempts: 1,
        };

        // Adjust based on budget usage
        if budget_usage > 0.9 {
            strategy.primary_tier = ModelTier::Tiny;
            strategy.fallback_tier = ModelTier::Small;
            strategy.cost_strategy = CostStrategy::UltraFrugal;
        } else if budget_usage > 0.7 {
            strategy.cost_strategy = CostStrategy::PreferLocal;
        } else if budget_usage < 0.3 {
            strategy.primary_tier = ModelTier::Medium;
            strategy.fallback_tier = ModelTier::Large;
            strategy.cost_strategy = CostStrategy::QualityFirst;
        }

        // Adjust based on success rate
        if success_rate < 0.8 {
            // Low success rate, try higher tier models
            strategy.primary_tier = self.next_tier(strategy.primary_tier);
            strategy.fallback_tier = self.next_tier(strategy.fallback_tier);
            strategy.parallel_attempts = 2; // Try multiple models
        }

        // Check peak hours
        if let Some((hour, _cost)) = cost_stats.peak_usage_hour() {
            let current_hour = chrono::Local::now().hour() as usize;
            if (current_hour as i32 - hour as i32).abs() <= 1 {
                // During peak hours, be more conservative
                strategy.enable_batching = true;
                strategy.max_latency_ms = 10000; // Allow higher latency for batching
            }
        }

        info!(
            "Routing strategy: primary={:?}, fallback={:?}, cost={:?}",
            strategy.primary_tier, strategy.fallback_tier, strategy.cost_strategy
        );

        strategy
    }

    /// Optimize model selection for batch processing
    pub fn optimize_batch(
        &self,
        tasks: &[TaskRequirements],
        available_models: &[ModelConfig],
        budget_remaining: f64,
    ) -> BatchOptimization {
        let mut optimization = BatchOptimization {
            task_assignments: HashMap::new(),
            total_cost: 0.0,
            estimated_time_ms: 0,
            models_used: Vec::new(),
        };

        // Group tasks by complexity
        let mut tier_tasks: HashMap<ModelTier, Vec<usize>> = HashMap::new();
        for (idx, task) in tasks.iter().enumerate() {
            let tier = self.estimate_tier_for_task(task);
            tier_tasks.entry(tier).or_insert_with(Vec::new).push(idx);
        }

        // Assign models to task groups
        for (tier, task_indices) in tier_tasks {
            if let Some(model) =
                self.find_best_model_for_tier(tier, available_models, budget_remaining)
            {
                for idx in task_indices {
                    let task = &tasks[idx];
                    let cost = model.estimate_cost(task.estimated_tokens);

                    if optimization.total_cost + cost <= budget_remaining {
                        optimization.task_assignments.insert(idx, model.id.clone());
                        optimization.total_cost += cost;
                        optimization.estimated_time_ms =
                            optimization.estimated_time_ms.max(model.average_latency_ms);

                        if !optimization.models_used.contains(&model.id) {
                            optimization.models_used.push(model.id.clone());
                        }
                    }
                }
            }
        }

        debug!(
            "Batch optimization: {} tasks assigned, cost=${:.4}, time={}ms",
            optimization.task_assignments.len(),
            optimization.total_cost,
            optimization.estimated_time_ms
        );

        optimization
    }

    fn meets_requirements(&self, model: &ModelConfig, requirements: &TaskRequirements) -> bool {
        // Check context window
        if model.context_window < requirements.min_context_window {
            return false;
        }

        // Check latency
        if let Some(max_latency) = requirements.max_latency_ms {
            if model.average_latency_ms > max_latency {
                return false;
            }
        }

        // Check cost
        if let Some(max_cost) = requirements.max_cost_per_1k {
            if model.cost_per_1k_tokens > max_cost {
                return false;
            }
        }

        true
    }

    fn get_performance_score(
        &self,
        model_id: &str,
        performance_data: Option<&HashMap<String, ModelMetrics>>,
    ) -> f32 {
        if let Some(data) = performance_data {
            if let Some(metrics) = data.get(model_id) {
                return metrics.success_rate() * 0.5
                    + (1.0 - metrics.normalized_latency()) * 0.3
                    + metrics.average_quality() * 0.2;
            }
        }
        0.5 // Default middle score
    }

    fn estimate_tier_for_task(&self, task: &TaskRequirements) -> ModelTier {
        if task.estimated_tokens < 100 {
            ModelTier::Tiny
        } else if task.estimated_tokens < 500 {
            ModelTier::Small
        } else if task.estimated_tokens < 2000 {
            ModelTier::Medium
        } else {
            ModelTier::Large
        }
    }

    fn find_best_model_for_tier(
        &self,
        tier: ModelTier,
        models: &[ModelConfig],
        budget: f64,
    ) -> Option<ModelConfig> {
        models
            .iter()
            .filter(|m| m.tier == tier && m.available)
            .filter(|m| m.estimate_cost(1000) <= budget)
            .min_by_key(|m| (m.cost_per_1k_tokens * 1000.0) as i64)
            .cloned()
    }

    fn next_tier(&self, tier: ModelTier) -> ModelTier {
        match tier {
            ModelTier::NoLLM => ModelTier::Tiny,
            ModelTier::Tiny => ModelTier::Small,
            ModelTier::Small => ModelTier::Medium,
            ModelTier::Medium => ModelTier::Large,
            ModelTier::Large => ModelTier::Large,
        }
    }
}

/// Optimizer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizerConfig {
    pub cost_weight: f32,
    pub performance_weight: f32,
    pub quality_weight: f32,
    pub enable_caching: bool,
    pub enable_batching: bool,
    pub batch_size: usize,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            cost_weight: 0.4,
            performance_weight: 0.3,
            quality_weight: 0.3,
            enable_caching: true,
            enable_batching: true,
            batch_size: 10,
        }
    }
}

/// Routing strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingStrategy {
    pub primary_tier: ModelTier,
    pub fallback_tier: ModelTier,
    pub cost_strategy: CostStrategy,
    pub quality_threshold: f32,
    pub max_latency_ms: u32,
    pub prefer_cached: bool,
    pub enable_batching: bool,
    pub parallel_attempts: usize,
}

/// Task requirements for routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequirements {
    pub estimated_tokens: usize,
    pub min_context_window: usize,
    pub max_latency_ms: Option<u32>,
    pub max_cost_per_1k: Option<f64>,
    pub required_capabilities: Vec<String>,
    pub priority: TaskPriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Batch optimization result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOptimization {
    pub task_assignments: HashMap<usize, String>, // task_index -> model_id
    pub total_cost: f64,
    pub estimated_time_ms: u32,
    pub models_used: Vec<String>,
}

/// A/B testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestConfig {
    pub enabled: bool,
    pub model_a: String,
    pub model_b: String,
    pub split_ratio: f32, // 0.0 to 1.0, percentage for model A
    pub metrics_to_track: Vec<String>,
}

/// Dynamic tier adjustment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicTierAdjustment {
    pub task_pattern: String,
    pub original_tier: ModelTier,
    pub adjusted_tier: ModelTier,
    pub reason: String,
    pub success_rate_improvement: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimal_model_selection() {
        let optimizer = CostOptimizer::new(OptimizerConfig::default());

        let models = vec![
            ModelConfig {
                id: "cheap".to_string(),
                name: "Cheap Model".to_string(),
                tier: ModelTier::Small,
                provider: crate::llm::router::registry::Provider::Local,
                cost_per_1k_tokens: 0.0,
                average_latency_ms: 100,
                context_window: 4096,
                capabilities: vec![],
                available: true,
                local_path: None,
                api_endpoint: None,
                requires_auth: false,
            },
            ModelConfig {
                id: "expensive".to_string(),
                name: "Expensive Model".to_string(),
                tier: ModelTier::Large,
                provider: crate::llm::router::registry::Provider::OpenAI,
                cost_per_1k_tokens: 0.01,
                average_latency_ms: 500,
                context_window: 128000,
                capabilities: vec![],
                available: true,
                local_path: None,
                api_endpoint: None,
                requires_auth: true,
            },
        ];

        let requirements = TaskRequirements {
            estimated_tokens: 1000,
            min_context_window: 2048,
            max_latency_ms: Some(1000),
            max_cost_per_1k: Some(0.005),
            required_capabilities: vec![],
            priority: TaskPriority::Normal,
        };

        // With cost optimization, should select cheap model
        let selected = optimizer.select_optimal_model(
            &models,
            CostStrategy::CostOptimized,
            None,
            &requirements,
        );

        assert_eq!(selected.unwrap().id, "cheap");
    }
}
