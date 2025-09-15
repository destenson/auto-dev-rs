//! Performance monitoring and optimization for LLM routing

use crate::llm::provider::ModelTier;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Performance monitoring system
pub struct PerformanceMonitor {
    metrics: Arc<RwLock<HashMap<String, ModelMetrics>>>,
    history: Arc<RwLock<PerformanceHistory>>,
    config: PerformanceConfig,
}

impl PerformanceMonitor {
    pub fn new(config: PerformanceConfig) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(PerformanceHistory::new(config.history_size))),
            config,
        }
    }

    /// Update metrics for a model execution
    pub async fn update_metrics(
        &self,
        model_id: &str,
        tier: ModelTier,
        success: bool,
        latency: Duration,
        tokens: usize,
        quality_score: Option<f32>,
    ) {
        let mut metrics_map = self.metrics.write().await;
        let metrics = metrics_map.entry(model_id.to_string())
            .or_insert_with(|| ModelMetrics::new(model_id.to_string(), tier));
        
        // Update success/failure counts
        if success {
            metrics.success_count += 1;
            metrics.total_latency += latency;
            metrics.total_tokens += tokens;
            
            if let Some(score) = quality_score {
                metrics.quality_scores.push(score);
                if metrics.quality_scores.len() > 100 {
                    metrics.quality_scores.remove(0);
                }
            }
        } else {
            metrics.failure_count += 1;
        }
        
        // Update latency percentiles
        metrics.latencies.push_back(latency);
        if metrics.latencies.len() > self.config.percentile_window {
            metrics.latencies.pop_front();
        }
        
        // Recalculate aggregates
        metrics.recalculate();
        
        // Add to history
        let mut history = self.history.write().await;
        history.add_event(PerformanceEvent {
            timestamp: Utc::now(),
            model_id: model_id.to_string(),
            tier,
            success,
            latency,
            tokens,
            quality_score,
        });
        
        // Check for performance issues
        if metrics.failure_rate() > self.config.max_failure_rate {
            warn!("Model {} has high failure rate: {:.2}%",
                  model_id, metrics.failure_rate() * 100.0);
        }
        
        if metrics.average_latency() > self.config.max_latency {
            warn!("Model {} has high latency: {:?}",
                  model_id, metrics.average_latency());
        }
    }

    /// Get performance metrics for a model
    pub async fn get_metrics(&self, model_id: &str) -> Option<ModelMetrics> {
        self.metrics.read().await.get(model_id).cloned()
    }

    /// Get all metrics
    pub async fn get_all_metrics(&self) -> HashMap<String, ModelMetrics> {
        self.metrics.read().await.clone()
    }

    /// Recommend tier adjustments based on performance
    pub async fn recommend_tier_adjustments(&self) -> Vec<TierAdjustment> {
        let metrics = self.metrics.read().await;
        let mut adjustments = Vec::new();
        
        for (model_id, model_metrics) in metrics.iter() {
            // Check if model is underperforming
            if model_metrics.failure_rate() > 0.2 {
                adjustments.push(TierAdjustment {
                    model_id: model_id.clone(),
                    current_tier: model_metrics.tier,
                    recommended_tier: self.next_tier(model_metrics.tier),
                    reason: format!("High failure rate: {:.1}%", 
                                  model_metrics.failure_rate() * 100.0),
                });
            }
            
            // Check if model is too slow
            if model_metrics.average_latency() > Duration::from_secs(10) {
                adjustments.push(TierAdjustment {
                    model_id: model_id.clone(),
                    current_tier: model_metrics.tier,
                    recommended_tier: self.next_tier(model_metrics.tier),
                    reason: format!("High latency: {:?}", 
                                  model_metrics.average_latency()),
                });
            }
            
            // Check if quality is too low
            if model_metrics.average_quality() < self.config.min_quality_score {
                adjustments.push(TierAdjustment {
                    model_id: model_id.clone(),
                    current_tier: model_metrics.tier,
                    recommended_tier: self.next_tier(model_metrics.tier),
                    reason: format!("Low quality score: {:.2}", 
                                  model_metrics.average_quality()),
                });
            }
        }
        
        adjustments
    }

    /// Get performance report
    pub async fn get_performance_report(&self) -> PerformanceReport {
        let metrics = self.metrics.read().await;
        let history = self.history.read().await;
        
        let mut total_requests = 0usize;
        let mut total_successes = 0usize;
        let mut total_tokens = 0usize;
        let mut tier_performance = HashMap::new();
        
        for model_metrics in metrics.values() {
            total_requests += model_metrics.success_count + model_metrics.failure_count;
            total_successes += model_metrics.success_count;
            total_tokens += model_metrics.total_tokens;
            
            let tier_stats = tier_performance.entry(model_metrics.tier)
                .or_insert_with(TierPerformance::default);
            tier_stats.requests += model_metrics.success_count + model_metrics.failure_count;
            tier_stats.successes += model_metrics.success_count;
            tier_stats.total_latency += model_metrics.total_latency;
            tier_stats.tokens += model_metrics.total_tokens;
        }
        
        PerformanceReport {
            total_requests,
            total_successes,
            total_tokens,
            overall_success_rate: if total_requests > 0 {
                total_successes as f32 / total_requests as f32
            } else {
                0.0
            },
            tier_performance,
            recent_events: history.recent_events(20),
            model_rankings: self.rank_models(&metrics),
        }
    }

    fn next_tier(&self, current: ModelTier) -> ModelTier {
        match current {
            ModelTier::NoLLM => ModelTier::Tiny,
            ModelTier::Tiny => ModelTier::Small,
            ModelTier::Small => ModelTier::Medium,
            ModelTier::Medium => ModelTier::Large,
            ModelTier::Large => ModelTier::Large,
        }
    }

    fn rank_models(&self, metrics: &HashMap<String, ModelMetrics>) -> Vec<ModelRanking> {
        let mut rankings: Vec<_> = metrics.iter()
            .map(|(id, m)| {
                let score = m.success_rate() * 0.4
                    + (1.0 - m.normalized_latency()) * 0.3
                    + m.average_quality() * 0.3;
                
                ModelRanking {
                    model_id: id.clone(),
                    tier: m.tier,
                    performance_score: score,
                    success_rate: m.success_rate(),
                    average_latency: m.average_latency(),
                    quality_score: m.average_quality(),
                }
            })
            .collect();
        
        rankings.sort_by(|a, b| b.performance_score.partial_cmp(&a.performance_score).unwrap());
        rankings
    }
}

/// Performance metrics for a single model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetrics {
    pub model_id: String,
    pub tier: ModelTier,
    pub success_count: usize,
    pub failure_count: usize,
    pub total_latency: Duration,
    pub total_tokens: usize,
    pub quality_scores: Vec<f32>,
    pub latencies: VecDeque<Duration>,
    pub average_latency_ms: u64,
    pub p50_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
    pub success_rate: f32,
    pub tokens_per_second: f32,
}

impl ModelMetrics {
    fn new(model_id: String, tier: ModelTier) -> Self {
        Self {
            model_id,
            tier,
            success_count: 0,
            failure_count: 0,
            total_latency: Duration::from_secs(0),
            total_tokens: 0,
            quality_scores: Vec::new(),
            latencies: VecDeque::new(),
            average_latency_ms: 0,
            p50_latency_ms: 0,
            p95_latency_ms: 0,
            p99_latency_ms: 0,
            success_rate: 0.0,
            tokens_per_second: 0.0,
        }
    }

    fn recalculate(&mut self) {
        // Calculate success rate
        let total = self.success_count + self.failure_count;
        self.success_rate = if total > 0 {
            self.success_count as f32 / total as f32
        } else {
            0.0
        };
        
        // Calculate average latency
        if self.success_count > 0 {
            self.average_latency_ms = self.total_latency.as_millis() as u64 / self.success_count as u64;
        }
        
        // Calculate percentiles
        if !self.latencies.is_empty() {
            let mut sorted: Vec<_> = self.latencies.iter().collect();
            sorted.sort();
            
            let len = sorted.len();
            self.p50_latency_ms = sorted[len / 2].as_millis() as u64;
            self.p95_latency_ms = sorted[len * 95 / 100].as_millis() as u64;
            self.p99_latency_ms = sorted[len * 99 / 100].as_millis() as u64;
        }
        
        // Calculate tokens per second
        if self.total_latency.as_secs() > 0 {
            self.tokens_per_second = self.total_tokens as f32 / self.total_latency.as_secs_f32();
        }
    }

    pub fn failure_rate(&self) -> f32 {
        1.0 - self.success_rate
    }

    pub fn average_latency(&self) -> Duration {
        Duration::from_millis(self.average_latency_ms)
    }

    pub fn average_quality(&self) -> f32 {
        if self.quality_scores.is_empty() {
            0.5 // Default middle score
        } else {
            self.quality_scores.iter().sum::<f32>() / self.quality_scores.len() as f32
        }
    }

    pub fn normalized_latency(&self) -> f32 {
        // Normalize to 0-1 where 0 is best (0ms) and 1 is worst (10s+)
        (self.average_latency_ms as f32 / 10000.0).min(1.0)
    }

    pub fn success_rate(&self) -> f32 {
        self.success_rate
    }
}

/// Performance history tracking
#[derive(Debug, Clone)]
struct PerformanceHistory {
    events: VecDeque<PerformanceEvent>,
    max_size: usize,
}

impl PerformanceHistory {
    fn new(max_size: usize) -> Self {
        Self {
            events: VecDeque::new(),
            max_size,
        }
    }

    fn add_event(&mut self, event: PerformanceEvent) {
        self.events.push_back(event);
        if self.events.len() > self.max_size {
            self.events.pop_front();
        }
    }

    fn recent_events(&self, count: usize) -> Vec<PerformanceEvent> {
        self.events.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }
}

/// Performance event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceEvent {
    pub timestamp: DateTime<Utc>,
    pub model_id: String,
    pub tier: ModelTier,
    pub success: bool,
    pub latency: Duration,
    pub tokens: usize,
    pub quality_score: Option<f32>,
}

/// Tier adjustment recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierAdjustment {
    pub model_id: String,
    pub current_tier: ModelTier,
    pub recommended_tier: ModelTier,
    pub reason: String,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub history_size: usize,
    pub percentile_window: usize,
    pub max_failure_rate: f32,
    pub max_latency: Duration,
    pub min_quality_score: f32,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            history_size: 1000,
            percentile_window: 100,
            max_failure_rate: 0.1,
            max_latency: Duration::from_secs(5),
            min_quality_score: 0.7,
        }
    }
}

/// Performance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub total_requests: usize,
    pub total_successes: usize,
    pub total_tokens: usize,
    pub overall_success_rate: f32,
    pub tier_performance: HashMap<ModelTier, TierPerformance>,
    pub recent_events: Vec<PerformanceEvent>,
    pub model_rankings: Vec<ModelRanking>,
}

/// Tier-specific performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TierPerformance {
    pub requests: usize,
    pub successes: usize,
    pub total_latency: Duration,
    pub tokens: usize,
}

impl TierPerformance {
    pub fn success_rate(&self) -> f32 {
        if self.requests > 0 {
            self.successes as f32 / self.requests as f32
        } else {
            0.0
        }
    }

    pub fn average_latency(&self) -> Duration {
        if self.successes > 0 {
            self.total_latency / self.successes as u32
        } else {
            Duration::from_secs(0)
        }
    }
}

/// Model ranking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRanking {
    pub model_id: String,
    pub tier: ModelTier,
    pub performance_score: f32,
    pub success_rate: f32,
    pub average_latency: Duration,
    pub quality_score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_tracking() {
        let monitor = PerformanceMonitor::new(PerformanceConfig::default());
        
        // Track some successful requests
        monitor.update_metrics(
            "gpt-4",
            ModelTier::Large,
            true,
            Duration::from_millis(500),
            1000,
            Some(0.9),
        ).await;
        
        monitor.update_metrics(
            "gpt-4",
            ModelTier::Large,
            true,
            Duration::from_millis(600),
            1200,
            Some(0.85),
        ).await;
        
        // Track a failure
        monitor.update_metrics(
            "gpt-4",
            ModelTier::Large,
            false,
            Duration::from_millis(0),
            0,
            None,
        ).await;
        
        let metrics = monitor.get_metrics("gpt-4").await.unwrap();
        assert_eq!(metrics.success_count, 2);
        assert_eq!(metrics.failure_count, 1);
        assert!(metrics.success_rate > 0.6);
    }

    #[tokio::test]
    async fn test_tier_recommendations() {
        let monitor = PerformanceMonitor::new(PerformanceConfig::default());
        
        // Track high failure rate
        for _ in 0..10 {
            monitor.update_metrics(
                "weak-model",
                ModelTier::Small,
                false,
                Duration::from_millis(100),
                0,
                None,
            ).await;
        }
        
        monitor.update_metrics(
            "weak-model",
            ModelTier::Small,
            true,
            Duration::from_millis(100),
            100,
            Some(0.5),
        ).await;
        
        let adjustments = monitor.recommend_tier_adjustments().await;
        assert!(!adjustments.is_empty());
        assert_eq!(adjustments[0].recommended_tier, ModelTier::Medium);
    }
}