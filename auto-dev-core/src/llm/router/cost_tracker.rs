//! Cost tracking and optimization for LLM usage

use crate::llm::provider::ModelTier;
use anyhow::Result;
use chrono::{DateTime, Datelike, Local, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Main cost tracking system
pub struct CostTracker {
    state: Arc<RwLock<CostState>>,
    config: CostConfig,
    alerts: Arc<RwLock<Vec<BudgetAlert>>>,
}

impl CostTracker {
    pub fn new(config: CostConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(CostState::new())),
            config,
            alerts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Track usage of a model
    pub async fn track_usage(&self, model_id: &str, tier: ModelTier, tokens: usize, cost: f64) {
        let mut state = self.state.write().await;

        // Update daily spending
        state.spent_today += cost;

        // Update model-specific costs
        *state.model_costs.entry(model_id.to_string()).or_insert(0.0) += cost;

        // Update tier usage
        *state.tier_usage.entry(tier).or_insert(0) += tokens;

        // Track hourly usage for rate limiting
        let hour = Local::now().hour() as usize;
        state.hourly_costs[hour] += cost;

        // Check budget alerts
        self.check_budget_alerts(state.spent_today).await;

        info!(
            "Tracked usage: {} tokens for ${:.4} (model: {}, tier: {:?})",
            tokens, cost, model_id, tier
        );
    }

    /// Get current routing strategy based on budget
    pub async fn get_routing_strategy(&self) -> CostStrategy {
        let state = self.state.read().await;
        let budget_used_percent = (state.spent_today / self.config.daily_budget) * 100.0;

        if budget_used_percent > 90.0 {
            CostStrategy::UltraFrugal
        } else if budget_used_percent > 70.0 {
            CostStrategy::PreferLocal
        } else if budget_used_percent > 50.0 {
            CostStrategy::Balanced
        } else if budget_used_percent < 20.0 {
            CostStrategy::QualityFirst
        } else {
            CostStrategy::CostOptimized
        }
    }

    /// Check and trigger budget alerts
    async fn check_budget_alerts(&self, spent_today: f64) {
        let percent_used = (spent_today / self.config.daily_budget) * 100.0;

        let mut alerts = self.alerts.write().await;

        // 50% alert
        if percent_used >= 50.0 && !alerts.iter().any(|a| matches!(a, BudgetAlert::FiftyPercent)) {
            alerts.push(BudgetAlert::FiftyPercent);
            warn!(
                "Budget alert: 50% of daily budget used (${:.2} of ${:.2})",
                spent_today, self.config.daily_budget
            );
        }

        // 80% alert
        if percent_used >= 80.0 && !alerts.iter().any(|a| matches!(a, BudgetAlert::EightyPercent)) {
            alerts.push(BudgetAlert::EightyPercent);
            warn!(
                "Budget alert: 80% of daily budget used (${:.2} of ${:.2})",
                spent_today, self.config.daily_budget
            );
        }

        // 100% alert
        if percent_used >= 100.0 && !alerts.iter().any(|a| matches!(a, BudgetAlert::BudgetExceeded))
        {
            alerts.push(BudgetAlert::BudgetExceeded);
            warn!(
                "BUDGET EXCEEDED: Daily budget of ${:.2} exceeded (current: ${:.2})",
                self.config.daily_budget, spent_today
            );
        }
    }

    /// Get cost statistics
    pub async fn get_stats(&self) -> CostStats {
        let state = self.state.read().await;

        CostStats {
            spent_today: state.spent_today,
            spent_this_month: state.spent_this_month,
            model_costs: state.model_costs.clone(),
            tier_usage: state.tier_usage.clone(),
            hourly_costs: state.hourly_costs.clone(),
            daily_budget: self.config.daily_budget,
            monthly_budget: self.config.monthly_budget,
        }
    }

    /// Reset daily counters (should be called at midnight)
    pub async fn reset_daily(&self) {
        let mut state = self.state.write().await;

        // Add today's spending to monthly total
        state.spent_this_month += state.spent_today;

        // Reset daily counters
        state.spent_today = 0.0;
        state.model_costs.clear();
        state.hourly_costs = [0.0; 24];

        // Clear alerts
        self.alerts.write().await.clear();

        info!("Daily cost counters reset. Month total: ${:.2}", state.spent_this_month);
    }

    /// Reset monthly counters (should be called on the 1st)
    pub async fn reset_monthly(&self) {
        let mut state = self.state.write().await;
        state.spent_this_month = 0.0;
        info!("Monthly cost counters reset");
    }

    /// Calculate estimated cost for a task
    pub fn estimate_cost(&self, tier: ModelTier, estimated_tokens: usize) -> f64 {
        let cost_per_1k = match tier {
            ModelTier::NoLLM => 0.0,
            ModelTier::Tiny => 0.0,      // Local models
            ModelTier::Small => 0.0002,  // ~$0.20 per million
            ModelTier::Medium => 0.0006, // ~$0.60 per million
            ModelTier::Large => 0.015,   // ~$15 per million
        };

        (estimated_tokens as f64 / 1000.0) * cost_per_1k
    }

    /// Get remaining budget for today
    pub async fn get_remaining_budget(&self) -> f64 {
        let state = self.state.read().await;
        (self.config.daily_budget - state.spent_today).max(0.0)
    }

    /// Check if we can afford a task
    pub async fn can_afford(&self, estimated_cost: f64) -> bool {
        let remaining = self.get_remaining_budget().await;

        // Allow small overages for important tasks
        if estimated_cost < 0.01 {
            true
        } else {
            estimated_cost <= remaining * 1.1 // Allow 10% overage
        }
    }
}

/// Cost tracking state
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CostState {
    spent_today: f64,
    spent_this_month: f64,
    model_costs: HashMap<String, f64>,
    tier_usage: HashMap<ModelTier, usize>,
    hourly_costs: [f64; 24],
    last_reset: DateTime<Local>,
}

impl CostState {
    fn new() -> Self {
        Self {
            spent_today: 0.0,
            spent_this_month: 0.0,
            model_costs: HashMap::new(),
            tier_usage: HashMap::new(),
            hourly_costs: [0.0; 24],
            last_reset: Local::now(),
        }
    }
}

/// Cost tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    pub daily_budget: f64,
    pub monthly_budget: f64,
    pub alert_thresholds: Vec<f64>,
    pub prefer_local_when_over_budget: bool,
    pub block_expensive_when_over_budget: bool,
}

impl Default for CostConfig {
    fn default() -> Self {
        Self {
            daily_budget: 5.0,     // $5/day default
            monthly_budget: 100.0, // $100/month default
            alert_thresholds: vec![0.5, 0.8, 1.0],
            prefer_local_when_over_budget: true,
            block_expensive_when_over_budget: true,
        }
    }
}

/// Budget alerts
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BudgetAlert {
    FiftyPercent,
    EightyPercent,
    BudgetExceeded,
    RateLimitApproaching,
}

/// Cost optimization strategies
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CostStrategy {
    QualityFirst,  // Use best models regardless of cost
    Balanced,      // Balance cost and quality
    CostOptimized, // Prefer cheaper models when possible
    PreferLocal,   // Strongly prefer local models
    UltraFrugal,   // Only use free/local models
}

/// Cost statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostStats {
    pub spent_today: f64,
    pub spent_this_month: f64,
    pub model_costs: HashMap<String, f64>,
    pub tier_usage: HashMap<ModelTier, usize>,
    pub hourly_costs: [f64; 24],
    pub daily_budget: f64,
    pub monthly_budget: f64,
}

impl CostStats {
    pub fn budget_remaining_today(&self) -> f64 {
        (self.daily_budget - self.spent_today).max(0.0)
    }

    pub fn budget_remaining_month(&self) -> f64 {
        (self.monthly_budget - self.spent_this_month).max(0.0)
    }

    pub fn usage_by_tier(&self) -> Vec<(ModelTier, usize)> {
        let mut usage: Vec<_> =
            self.tier_usage.iter().map(|(tier, tokens)| (*tier, *tokens)).collect();
        usage.sort_by_key(|(tier, _)| *tier);
        usage
    }

    pub fn most_expensive_model(&self) -> Option<(String, f64)> {
        self.model_costs
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(name, cost)| (name.clone(), *cost))
    }

    pub fn peak_usage_hour(&self) -> Option<(usize, f64)> {
        self.hourly_costs
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(hour, cost)| (hour, *cost))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cost_tracking() {
        let config = CostConfig { daily_budget: 1.0, ..Default::default() };

        let tracker = CostTracker::new(config);

        // Track some usage
        tracker.track_usage("gpt-4", ModelTier::Large, 1000, 0.015).await;
        tracker.track_usage("llama-7b", ModelTier::Small, 5000, 0.001).await;

        let stats = tracker.get_stats().await;
        assert_eq!(stats.spent_today, 0.016);
        assert_eq!(stats.model_costs.len(), 2);
    }

    #[tokio::test]
    async fn test_routing_strategy() {
        let config = CostConfig { daily_budget: 1.0, ..Default::default() };

        let tracker = CostTracker::new(config);

        // Low spending should give QualityFirst
        let strategy = tracker.get_routing_strategy().await;
        assert_eq!(strategy, CostStrategy::QualityFirst);

        // High spending should give UltraFrugal
        tracker.track_usage("gpt-4", ModelTier::Large, 100000, 1.5).await;
        let strategy = tracker.get_routing_strategy().await;
        assert_eq!(strategy, CostStrategy::UltraFrugal);
    }

    #[test]
    fn test_cost_estimation() {
        let config = CostConfig::default();
        let tracker = CostTracker::new(config);

        // Test tier costs
        assert_eq!(tracker.estimate_cost(ModelTier::NoLLM, 1000), 0.0);
        assert_eq!(tracker.estimate_cost(ModelTier::Tiny, 1000), 0.0);
        assert!(tracker.estimate_cost(ModelTier::Large, 1000) > 0.01);
    }
}
