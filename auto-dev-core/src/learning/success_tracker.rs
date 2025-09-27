#![allow(unused)]
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

use crate::incremental::Implementation;
use crate::learning::knowledge_base::KnowledgeBase;
use crate::learning::learner::LearningEvent;
use crate::learning::pattern_extractor::Pattern;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessTracker {
    pub successes: Vec<SuccessRecord>,
    pub success_patterns: HashMap<String, Vec<SuccessPattern>>,
    pub metrics: SuccessMetricsAggregator,
    pub reinforcement_history: Vec<ReinforcementEvent>,
}

impl SuccessTracker {
    pub fn new() -> Self {
        Self {
            successes: Vec::new(),
            success_patterns: HashMap::new(),
            metrics: SuccessMetricsAggregator::new(),
            reinforcement_history: Vec::new(),
        }
    }

    pub fn track_success(&mut self, event: LearningEvent, metrics: SuccessMetrics) {
        let record = SuccessRecord {
            id: Uuid::new_v4(),
            event_id: event.id,
            timestamp: event.timestamp,
            specification_hash: hash_specification(&event.specification),
            implementation_hash: event.implementation.as_ref().map(hash_implementation),
            metrics: metrics.clone(),
            context: event.context.clone(),
        };

        self.successes.push(record.clone());
        self.metrics.add_metrics(&metrics);

        let pattern_key = format!("{}_{}", event.context.project_type, event.context.language);
        self.success_patterns.entry(pattern_key).or_insert_with(Vec::new).push(SuccessPattern {
            record_id: record.id,
            pattern_type: identify_pattern_type(&event),
            confidence: metrics.calculate_confidence(),
            features: extract_success_features(&event),
        });

        if let Some(implementation) = &event.implementation {
            self.optimize_for_similar(implementation, &metrics);
        }
    }

    pub fn reinforce(&mut self, event: &LearningEvent) {
        let reinforcement = ReinforcementEvent {
            id: Uuid::new_v4(),
            event_id: event.id,
            timestamp: Utc::now(),
            reinforcement_type: ReinforcementType::PositiveOutcome,
            strength: calculate_reinforcement_strength(event),
        };

        self.reinforcement_history.push(reinforcement);

        if let Some(similar) = self.find_similar_success(&event) {
            self.strengthen_pattern(similar.id);
        }
    }

    pub fn get_success_rate(&self) -> f32 {
        self.metrics.get_overall_success_rate()
    }

    pub fn find_successful_approach(&self, context: &str) -> Option<SuccessfulApproach> {
        let patterns = self.success_patterns.get(context)?;

        patterns
            .iter()
            .filter(|p| p.confidence > 0.7)
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .map(|pattern| {
                let record = self.successes.iter().find(|r| r.id == pattern.record_id).unwrap();

                SuccessfulApproach {
                    approach_id: pattern.record_id,
                    confidence: pattern.confidence,
                    metrics: record.metrics.clone(),
                    features: pattern.features.clone(),
                }
            })
    }

    pub fn get_optimization_hints(&self, context: &str) -> Vec<OptimizationHint> {
        let mut hints = Vec::new();

        if let Some(patterns) = self.success_patterns.get(context) {
            for pattern in patterns.iter().filter(|p| p.confidence > 0.8) {
                if let Some(record) = self.successes.iter().find(|r| r.id == pattern.record_id) {
                    hints.extend(generate_hints_from_success(&record.metrics, &pattern.features));
                }
            }
        }

        hints.sort_by(|a, b| b.impact.partial_cmp(&a.impact).unwrap());
        hints.truncate(5);
        hints
    }

    fn optimize_for_similar(&mut self, implementation: &Implementation, metrics: &SuccessMetrics) {
        let optimization = SuccessOptimization {
            implementation_signature: generate_signature(implementation),
            optimization_strategy: determine_optimization_strategy(metrics),
            expected_improvement: calculate_expected_improvement(metrics),
        };

        self.metrics.record_optimization(optimization);
    }

    fn find_similar_success(&self, event: &LearningEvent) -> Option<&SuccessRecord> {
        let spec_hash = hash_specification(&event.specification);

        self.successes.iter().find(|record| {
            record.specification_hash == spec_hash
                || self.is_contextually_similar(&record.context, &event.context)
        })
    }

    fn is_contextually_similar(
        &self,
        context1: &crate::learning::learner::EventContext,
        context2: &crate::learning::learner::EventContext,
    ) -> bool {
        context1.language == context2.language
            && context1.project_type == context2.project_type
            && context1.framework == context2.framework
    }

    fn strengthen_pattern(&mut self, record_id: Uuid) {
        for patterns in self.success_patterns.values_mut() {
            if let Some(pattern) = patterns.iter_mut().find(|p| p.record_id == record_id) {
                pattern.confidence = (pattern.confidence * 1.1).min(1.0);
            }
        }
    }

    pub fn get_statistics(&self) -> SuccessStatistics {
        SuccessStatistics {
            total_successes: self.successes.len(),
            success_rate: self.metrics.get_overall_success_rate(),
            average_implementation_time: self.metrics.get_average_implementation_time(),
            most_successful_patterns: self.get_top_patterns(5),
            improvement_over_time: self.calculate_improvement_trend(),
        }
    }

    fn get_top_patterns(&self, count: usize) -> Vec<(String, f32)> {
        let mut pattern_scores: HashMap<String, (f32, u32)> = HashMap::new();

        for (_context, patterns) in &self.success_patterns {
            for pattern in patterns {
                let entry = pattern_scores.entry(pattern.pattern_type.clone()).or_insert((0.0, 0));
                entry.0 += pattern.confidence;
                entry.1 += 1;
            }
        }

        let mut scores: Vec<_> = pattern_scores
            .into_iter()
            .map(|(pattern, (total_confidence, count))| (pattern, total_confidence / count as f32))
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scores.truncate(count);
        scores
    }

    fn calculate_improvement_trend(&self) -> f32 {
        if self.successes.len() < 10 {
            return 0.0;
        }

        let recent = &self.successes[self.successes.len() - 5..];
        let older = &self.successes[self.successes.len() - 10..self.successes.len() - 5];

        let recent_avg: f32 =
            recent.iter().map(|r| r.metrics.calculate_overall_score()).sum::<f32>()
                / recent.len() as f32;

        let older_avg: f32 = older.iter().map(|r| r.metrics.calculate_overall_score()).sum::<f32>()
            / older.len() as f32;

        recent_avg - older_avg
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessRecord {
    pub id: Uuid,
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub specification_hash: u64,
    pub implementation_hash: Option<u64>,
    pub metrics: SuccessMetrics,
    pub context: crate::learning::learner::EventContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessMetrics {
    pub compilation_success: bool,
    pub tests_passed: bool,
    pub performance_met: bool,
    pub security_passed: bool,
    pub specification_coverage: f32,
    pub implementation_time: Duration,
    pub llm_calls_used: u32,
}

impl SuccessMetrics {
    pub fn calculate_confidence(&self) -> f32 {
        let mut score = 0.0;

        if self.compilation_success {
            score += 0.3;
        }
        if self.tests_passed {
            score += 0.3;
        }
        if self.performance_met {
            score += 0.2;
        }
        if self.security_passed {
            score += 0.1;
        }

        score += self.specification_coverage * 0.1;

        score
    }

    pub fn calculate_overall_score(&self) -> f32 {
        let mut score = self.calculate_confidence();

        let time_penalty = (self.implementation_time.as_secs() as f32 / 3600.0).min(1.0) * 0.1;
        score -= time_penalty;

        let llm_penalty = (self.llm_calls_used as f32 / 100.0).min(1.0) * 0.1;
        score -= llm_penalty;

        score.max(0.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SuccessPattern {
    record_id: Uuid,
    pattern_type: String,
    confidence: f32,
    features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReinforcementEvent {
    id: Uuid,
    event_id: Uuid,
    timestamp: DateTime<Utc>,
    reinforcement_type: ReinforcementType,
    strength: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ReinforcementType {
    PositiveOutcome,
    TestSuccess,
    PerformanceImprovement,
    UserApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessfulApproach {
    pub approach_id: Uuid,
    pub confidence: f32,
    pub metrics: SuccessMetrics,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationHint {
    pub hint_type: String,
    pub description: String,
    pub impact: f32,
    pub implementation_suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SuccessOptimization {
    implementation_signature: String,
    optimization_strategy: String,
    expected_improvement: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessStatistics {
    pub total_successes: usize,
    pub success_rate: f32,
    pub average_implementation_time: Duration,
    pub most_successful_patterns: Vec<(String, f32)>,
    pub improvement_over_time: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SuccessMetricsAggregator {
    total_attempts: u32,
    successful_compilations: u32,
    successful_tests: u32,
    total_implementation_time: Duration,
    total_llm_calls: u32,
    optimizations: Vec<SuccessOptimization>,
}

impl SuccessMetricsAggregator {
    fn new() -> Self {
        Self {
            total_attempts: 0,
            successful_compilations: 0,
            successful_tests: 0,
            total_implementation_time: Duration::from_secs(0),
            total_llm_calls: 0,
            optimizations: Vec::new(),
        }
    }

    fn add_metrics(&mut self, metrics: &SuccessMetrics) {
        self.total_attempts += 1;
        if metrics.compilation_success {
            self.successful_compilations += 1;
        }
        if metrics.tests_passed {
            self.successful_tests += 1;
        }
        self.total_implementation_time += metrics.implementation_time;
        self.total_llm_calls += metrics.llm_calls_used;
    }

    fn get_overall_success_rate(&self) -> f32 {
        if self.total_attempts == 0 {
            return 0.0;
        }

        let compilation_rate = self.successful_compilations as f32 / self.total_attempts as f32;
        let test_rate = self.successful_tests as f32 / self.total_attempts as f32;

        (compilation_rate * 0.6 + test_rate * 0.4)
    }

    fn get_average_implementation_time(&self) -> Duration {
        if self.total_attempts == 0 {
            Duration::from_secs(0)
        } else {
            self.total_implementation_time / self.total_attempts
        }
    }

    fn record_optimization(&mut self, optimization: SuccessOptimization) {
        self.optimizations.push(optimization);
    }
}

fn hash_specification(spec: &crate::parser::model::Specification) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    format!("{:?}", spec).hash(&mut hasher);
    hasher.finish()
}

fn hash_implementation(impl_: &Implementation) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    let code = impl_.files.iter().map(|f| f.content.as_str()).collect::<Vec<_>>().join("\n");
    code.hash(&mut hasher);
    hasher.finish()
}

fn identify_pattern_type(event: &LearningEvent) -> String {
    match event.event_type {
        crate::learning::learner::LearningEventType::ImplementationSuccess => {
            "implementation".to_string()
        }
        crate::learning::learner::LearningEventType::TestPassed => "testing".to_string(),
        crate::learning::learner::LearningEventType::PerformanceImproved => {
            "performance".to_string()
        }
        _ => "general".to_string(),
    }
}

fn extract_success_features(event: &LearningEvent) -> Vec<String> {
    let mut features = Vec::new();

    features.push(event.context.language.clone());
    if let Some(framework) = &event.context.framework {
        features.push(framework.clone());
    }
    features.push(event.context.project_type.clone());

    if event.metrics.test_coverage > 0.8 {
        features.push("high_coverage".to_string());
    }
    if event.metrics.llm_calls < 5 {
        features.push("low_llm_usage".to_string());
    }

    features
}

fn calculate_reinforcement_strength(event: &LearningEvent) -> f32 {
    let mut strength = 0.5;

    if event.outcome.success {
        strength += 0.2;
    }

    strength += event.outcome.score * 0.3;

    strength.min(1.0)
}

fn generate_signature(implementation: &Implementation) -> String {
    format!("sig_{}", hash_implementation(implementation))
}

fn determine_optimization_strategy(metrics: &SuccessMetrics) -> String {
    if metrics.llm_calls_used > 10 {
        "reduce_llm_calls".to_string()
    } else if metrics.implementation_time > Duration::from_secs(300) {
        "improve_speed".to_string()
    } else {
        "maintain_quality".to_string()
    }
}

fn calculate_expected_improvement(metrics: &SuccessMetrics) -> f32 {
    let current_score = metrics.calculate_overall_score();
    let potential_score = 1.0;

    potential_score - current_score
}

fn generate_hints_from_success(
    metrics: &SuccessMetrics,
    _features: &[String],
) -> Vec<OptimizationHint> {
    let mut hints = Vec::new();

    if metrics.llm_calls_used < 5 {
        hints.push(OptimizationHint {
            hint_type: "llm_optimization".to_string(),
            description: "This approach uses minimal LLM calls".to_string(),
            impact: 0.8,
            implementation_suggestion: "Consider caching or pattern reuse".to_string(),
        });
    }

    if metrics.implementation_time < Duration::from_secs(60) {
        hints.push(OptimizationHint {
            hint_type: "speed_optimization".to_string(),
            description: "Fast implementation approach".to_string(),
            impact: 0.7,
            implementation_suggestion: "Reuse this approach for similar tasks".to_string(),
        });
    }

    hints
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_tracking() {
        let mut tracker = SuccessTracker::new();

        let metrics = SuccessMetrics {
            compilation_success: true,
            tests_passed: true,
            performance_met: true,
            security_passed: true,
            specification_coverage: 0.9,
            implementation_time: Duration::from_secs(30),
            llm_calls_used: 3,
        };

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
            metrics: crate::learning::learner::PerformanceMetrics {
                duration: Duration::from_secs(30),
                llm_calls: 3,
                memory_used: 1000,
                cpu_usage: 0.5,
                test_coverage: 0.9,
                code_quality_score: 0.85,
            },
            context: crate::learning::learner::EventContext {
                project_type: "rust".to_string(),
                language: "rust".to_string(),
                framework: None,
                dependencies: vec![],
                environment: serde_json::json!({}),
            },
        };

        tracker.track_success(event, metrics);

        assert_eq!(tracker.successes.len(), 1);
        assert!(tracker.get_success_rate() > 0.0);
    }

    #[test]
    fn test_success_metrics_scoring() {
        let metrics = SuccessMetrics {
            compilation_success: true,
            tests_passed: true,
            performance_met: false,
            security_passed: true,
            specification_coverage: 0.8,
            implementation_time: Duration::from_secs(120),
            llm_calls_used: 10,
        };

        let confidence = metrics.calculate_confidence();
        assert!(confidence > 0.0 && confidence <= 1.0);

        let overall = metrics.calculate_overall_score();
        assert!(overall > 0.0 && overall <= 1.0);
    }
}
