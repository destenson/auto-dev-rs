use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::learning::failure_analyzer::FailureCause;
use crate::learning::learner::LearningEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionImprover {
    pub decision_weights: HashMap<DecisionType, f32>,
    pub confidence_scores: HashMap<DecisionType, f32>,
    pub decision_history: DecisionHistory,
    pub performance_tracking: HashMap<DecisionType, DecisionPerformance>,
    pub strategy_selector: StrategySelector,
}

impl DecisionImprover {
    pub fn new() -> Self {
        let mut decision_weights = HashMap::new();
        let mut confidence_scores = HashMap::new();

        for decision_type in DecisionType::all() {
            decision_weights.insert(decision_type.clone(), 0.5);
            confidence_scores.insert(decision_type.clone(), 0.5);
        }

        Self {
            decision_weights,
            confidence_scores,
            decision_history: DecisionHistory::new(),
            performance_tracking: HashMap::new(),
            strategy_selector: StrategySelector::new(),
        }
    }

    pub fn improve_from_success(&mut self, event: &LearningEvent) {
        let decision = self.extract_decision_from_event(event);

        self.update_weights(&decision, &Outcome::Success);
        self.update_confidence(&decision, &Outcome::Success);
        self.track_performance(&decision, true);

        self.decision_history.add_event(event.clone());

        tracing::debug!("Improved decision weights for {:?} after success", decision.decision_type);
    }

    pub fn improve_from_failure(&mut self, event: &LearningEvent, cause: &FailureCause) {
        let decision = self.extract_decision_from_event(event);

        self.update_weights(&decision, &Outcome::Failure);
        self.update_confidence(&decision, &Outcome::Failure);
        self.track_performance(&decision, false);

        self.learn_from_failure_cause(&decision.decision_type, cause);

        self.decision_history.add_event(event.clone());

        tracing::debug!("Adjusted decision weights for {:?} after failure", decision.decision_type);
    }

    pub fn select_decision(&self, options: Vec<Decision>) -> Decision {
        if options.is_empty() {
            return Decision::default();
        }

        if options.len() == 1 {
            return options.into_iter().next().unwrap();
        }

        options
            .into_iter()
            .max_by_key(|d| {
                let weight = self.decision_weights.get(&d.decision_type).unwrap_or(&0.5);
                let confidence = self.confidence_scores.get(&d.decision_type).unwrap_or(&0.5);
                let score = weight * confidence;

                (score * 10000.0) as u32
            })
            .unwrap_or_default()
    }

    pub fn suggest_strategy(&self, context: &DecisionContext) -> DecisionStrategy {
        self.strategy_selector.select_strategy(context, &self.performance_tracking)
    }

    pub fn get_accuracy(&self) -> f32 {
        if self.decision_history.events.is_empty() {
            return 0.5;
        }

        let successful =
            self.decision_history.events.iter().filter(|e| e.outcome.success).count() as f32;

        let total = self.decision_history.events.len() as f32;

        successful / total
    }

    pub fn get_decision_confidence(&self, decision_type: &DecisionType) -> f32 {
        *self.confidence_scores.get(decision_type).unwrap_or(&0.5)
    }

    pub fn get_decision_performance(
        &self,
        decision_type: &DecisionType,
    ) -> Option<&DecisionPerformance> {
        self.performance_tracking.get(decision_type)
    }

    fn extract_decision_from_event(&self, event: &LearningEvent) -> Decision {
        let decision_type = match event.event_type {
            crate::learning::learner::LearningEventType::ImplementationSuccess
            | crate::learning::learner::LearningEventType::ImplementationFailure => {
                DecisionType::Implementation
            }
            crate::learning::learner::LearningEventType::TestPassed
            | crate::learning::learner::LearningEventType::TestFailed => DecisionType::Testing,
            crate::learning::learner::LearningEventType::ValidationCompleted => {
                DecisionType::Validation
            }
            _ => DecisionType::General,
        };

        Decision {
            id: Uuid::new_v4(),
            decision_type,
            context: DecisionContext::from_event(event),
            confidence: self.get_decision_confidence(&decision_type),
            reasoning: String::new(),
            alternatives: Vec::new(),
            timestamp: event.timestamp,
        }
    }

    fn update_weights(&mut self, decision: &Decision, outcome: &Outcome) {
        let current_weight =
            self.decision_weights.get(&decision.decision_type).copied().unwrap_or(0.5);

        let new_weight = match outcome {
            Outcome::Success => (current_weight * 1.1).min(1.0),
            Outcome::Failure => (current_weight * 0.9).max(0.1),
        };

        self.decision_weights.insert(decision.decision_type.clone(), new_weight);

        if new_weight < 0.2 {
            self.deprecate_decision_type(&decision.decision_type);
        }
    }

    fn update_confidence(&mut self, decision: &Decision, outcome: &Outcome) {
        let current_confidence =
            self.confidence_scores.get(&decision.decision_type).copied().unwrap_or(0.5);

        let adjustment = match outcome {
            Outcome::Success => 0.05,
            Outcome::Failure => -0.05,
        };

        let new_confidence = (current_confidence + adjustment).clamp(0.1, 1.0);

        self.confidence_scores.insert(decision.decision_type.clone(), new_confidence);
    }

    fn track_performance(&mut self, decision: &Decision, success: bool) {
        let performance = self
            .performance_tracking
            .entry(decision.decision_type.clone())
            .or_insert_with(DecisionPerformance::new);

        performance.total_decisions += 1;
        if success {
            performance.successful_decisions += 1;
        } else {
            performance.failed_decisions += 1;
        }

        performance.update_success_rate();
        performance.last_decision = Utc::now();
    }

    fn learn_from_failure_cause(&mut self, decision_type: &DecisionType, cause: &FailureCause) {
        let performance = self
            .performance_tracking
            .entry(decision_type.clone())
            .or_insert_with(DecisionPerformance::new);

        let cause_category = cause.category();
        *performance.failure_causes.entry(cause_category).or_insert(0) += 1;

        if performance.failure_causes.len() > 3 {
            self.strategy_selector.add_avoidance_rule(decision_type.clone(), cause.clone());
        }
    }

    fn deprecate_decision_type(&mut self, decision_type: &DecisionType) {
        tracing::warn!("Deprecating decision type {:?} due to poor performance", decision_type);

        if let Some(performance) = self.performance_tracking.get_mut(decision_type) {
            performance.deprecated = true;
            performance.deprecated_at = Some(Utc::now());
        }
    }

    pub fn export_decision_model(&self) -> DecisionModel {
        DecisionModel {
            weights: self.decision_weights.clone(),
            confidence: self.confidence_scores.clone(),
            performance: self.performance_tracking.clone(),
            exported_at: Utc::now(),
        }
    }

    pub fn import_decision_model(&mut self, model: DecisionModel) {
        self.decision_weights = model.weights;
        self.confidence_scores = model.confidence;
        self.performance_tracking = model.performance;
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DecisionType {
    Implementation,
    Testing,
    Validation,
    Optimization,
    Refactoring,
    Documentation,
    General,
}

impl DecisionType {
    fn all() -> Vec<Self> {
        vec![
            Self::Implementation,
            Self::Testing,
            Self::Validation,
            Self::Optimization,
            Self::Refactoring,
            Self::Documentation,
            Self::General,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub id: Uuid,
    pub decision_type: DecisionType,
    pub context: DecisionContext,
    pub confidence: f32,
    pub reasoning: String,
    pub alternatives: Vec<Alternative>,
    pub timestamp: DateTime<Utc>,
}

impl Default for Decision {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            decision_type: DecisionType::General,
            context: DecisionContext::default(),
            confidence: 0.5,
            reasoning: String::new(),
            alternatives: Vec::new(),
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DecisionContext {
    pub project_type: String,
    pub language: String,
    pub complexity: usize,
    pub constraints: HashMap<String, String>,
    pub previous_attempts: u32,
}

impl DecisionContext {
    fn from_event(event: &LearningEvent) -> Self {
        Self {
            project_type: event.context.project_type.clone(),
            language: event.context.language.clone(),
            complexity: 1,
            constraints: HashMap::new(),
            previous_attempts: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    pub option: String,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionHistory {
    pub events: Vec<LearningEvent>,
    pub decisions: Vec<Decision>,
    pub outcomes: Vec<DecisionOutcome>,
}

impl DecisionHistory {
    pub fn new() -> Self {
        Self { events: Vec::new(), decisions: Vec::new(), outcomes: Vec::new() }
    }

    pub fn add_event(&mut self, event: LearningEvent) {
        self.events.push(event);

        if self.events.len() > 1000 {
            self.events.remove(0);
        }
    }

    pub fn add_decision(&mut self, decision: Decision, outcome: DecisionOutcome) {
        self.decisions.push(decision);
        self.outcomes.push(outcome);

        if self.decisions.len() > 1000 {
            self.decisions.remove(0);
            self.outcomes.remove(0);
        }
    }

    pub fn get_recent(&self, count: usize) -> Vec<LearningEvent> {
        let start = self.events.len().saturating_sub(count);
        self.events[start..].to_vec()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOutcome {
    pub decision_id: Uuid,
    pub success: bool,
    pub impact: f32,
    pub feedback: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DecisionPerformance {
    pub total_decisions: u32,
    pub successful_decisions: u32,
    pub failed_decisions: u32,
    pub success_rate: f32,
    pub failure_causes: HashMap<String, u32>,
    pub last_decision: DateTime<Utc>,
    pub deprecated: bool,
    pub deprecated_at: Option<DateTime<Utc>>,
}

impl DecisionPerformance {
    fn new() -> Self {
        Self {
            total_decisions: 0,
            successful_decisions: 0,
            failed_decisions: 0,
            success_rate: 0.5,
            failure_causes: HashMap::new(),
            last_decision: Utc::now(),
            deprecated: false,
            deprecated_at: None,
        }
    }

    fn update_success_rate(&mut self) {
        if self.total_decisions > 0 {
            self.success_rate = self.successful_decisions as f32 / self.total_decisions as f32;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StrategySelector {
    pub strategies: Vec<DecisionStrategy>,
    pub avoidance_rules: HashMap<DecisionType, Vec<FailureCause>>,
}

impl StrategySelector {
    fn new() -> Self {
        Self {
            strategies: vec![
                DecisionStrategy::Conservative,
                DecisionStrategy::Balanced,
                DecisionStrategy::Aggressive,
                DecisionStrategy::Experimental,
            ],
            avoidance_rules: HashMap::new(),
        }
    }

    fn select_strategy(
        &self,
        context: &DecisionContext,
        performance: &HashMap<DecisionType, DecisionPerformance>,
    ) -> DecisionStrategy {
        let avg_success_rate = if !performance.is_empty() {
            performance.values().map(|p| p.success_rate).sum::<f32>() / performance.len() as f32
        } else {
            0.5
        };

        if context.previous_attempts > 3 {
            DecisionStrategy::Conservative
        } else if avg_success_rate > 0.8 {
            DecisionStrategy::Aggressive
        } else if avg_success_rate < 0.3 {
            DecisionStrategy::Conservative
        } else if context.complexity > 5 {
            DecisionStrategy::Balanced
        } else {
            DecisionStrategy::Experimental
        }
    }

    fn add_avoidance_rule(&mut self, decision_type: DecisionType, cause: FailureCause) {
        self.avoidance_rules.entry(decision_type).or_insert_with(Vec::new).push(cause);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionStrategy {
    Conservative,
    Balanced,
    Aggressive,
    Experimental,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Outcome {
    Success,
    Failure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionModel {
    pub weights: HashMap<DecisionType, f32>,
    pub confidence: HashMap<DecisionType, f32>,
    pub performance: HashMap<DecisionType, DecisionPerformance>,
    pub exported_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_improver_creation() {
        let improver = DecisionImprover::new();

        assert_eq!(improver.get_accuracy(), 0.5);
        assert_eq!(improver.get_decision_confidence(&DecisionType::Implementation), 0.5);
    }

    #[test]
    fn test_decision_selection() {
        let improver = DecisionImprover::new();

        let decisions = vec![
            Decision {
                id: Uuid::new_v4(),
                decision_type: DecisionType::Implementation,
                context: DecisionContext::default(),
                confidence: 0.8,
                reasoning: String::new(),
                alternatives: Vec::new(),
                timestamp: Utc::now(),
            },
            Decision {
                id: Uuid::new_v4(),
                decision_type: DecisionType::Testing,
                context: DecisionContext::default(),
                confidence: 0.6,
                reasoning: String::new(),
                alternatives: Vec::new(),
                timestamp: Utc::now(),
            },
        ];

        let selected = improver.select_decision(decisions);
        assert_eq!(selected.decision_type, DecisionType::Implementation);
    }

    #[test]
    fn test_improve_from_success() {
        let mut improver = DecisionImprover::new();

        let event = LearningEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: crate::learning::learner::LearningEventType::ImplementationSuccess,
            specification: Default::default(),
            implementation: Some(Default::default()),
            outcome: crate::learning::learner::Outcome {
                success: true,
                score: 0.9,
                message: "Success".to_string(),
                details: serde_json::json!({}),
            },
            metrics: Default::default(),
            context: crate::learning::learner::EventContext {
                project_type: "rust".to_string(),
                language: "rust".to_string(),
                framework: None,
                dependencies: vec![],
                environment: serde_json::json!({}),
            },
        };

        let initial_weight = *improver.decision_weights.get(&DecisionType::Implementation).unwrap();
        improver.improve_from_success(&event);
        let new_weight = *improver.decision_weights.get(&DecisionType::Implementation).unwrap();

        assert!(new_weight > initial_weight);
    }
}
