#![allow(unused)]
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

use crate::incremental::Implementation;
use crate::parser::model::Specification;
use crate::validation::ValidationResult;
use crate::{debug, info};

use super::decision_improver::{DecisionHistory, DecisionImprover};
use super::failure_analyzer::{FailureAnalyzer, FailureCause};
use super::knowledge_base::{KnowledgeBase, KnowledgeExport};
use super::pattern_extractor::{Pattern, PatternExtractor};
use super::success_tracker::{SuccessMetrics, SuccessTracker};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningSystem {
    pub knowledge_base: KnowledgeBase,
    pub pattern_extractor: PatternExtractor,
    pub success_tracker: SuccessTracker,
    pub failure_analyzer: FailureAnalyzer,
    pub decision_history: DecisionHistory,
    pub decision_improver: DecisionImprover,
    pub metrics: LearningMetrics,
    pub config: LearningConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    pub enabled: bool,
    pub auto_learn: bool,
    pub min_pattern_quality: f32,
    pub max_patterns: usize,
    pub learning_rate: f32,
    pub export_path: PathBuf,
    pub import_on_startup: bool,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_learn: true,
            min_pattern_quality: 0.7,
            max_patterns: 10000,
            learning_rate: 0.1,
            export_path: PathBuf::from(".auto-dev/knowledge"),
            import_on_startup: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningMetrics {
    pub patterns_learned: u32,
    pub anti_patterns_identified: u32,
    pub success_rate_trend: Vec<f32>,
    pub llm_reduction_rate: f32,
    pub decision_accuracy: f32,
    pub knowledge_base_size: usize,
    pub average_implementation_time: Duration,
    pub total_learning_events: u32,
    pub last_updated: DateTime<Utc>,
}

impl Default for LearningMetrics {
    fn default() -> Self {
        Self {
            patterns_learned: 0,
            anti_patterns_identified: 0,
            success_rate_trend: Vec::new(),
            llm_reduction_rate: 0.0,
            decision_accuracy: 0.5,
            knowledge_base_size: 0,
            average_implementation_time: Duration::from_secs(0),
            total_learning_events: 0,
            last_updated: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: LearningEventType,
    pub specification: Specification,
    pub implementation: Option<Implementation>,
    pub outcome: Outcome,
    pub metrics: PerformanceMetrics,
    pub context: EventContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningEventType {
    ImplementationSuccess,
    ImplementationFailure,
    TestPassed,
    TestFailed,
    PerformanceImproved,
    PatternIdentified,
    AntiPatternDetected,
    DecisionMade,
    ValidationCompleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    pub success: bool,
    pub score: f32,
    pub message: String,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceMetrics {
    pub duration: Duration,
    pub llm_calls: u32,
    pub memory_used: usize,
    pub cpu_usage: f32,
    pub test_coverage: f32,
    pub code_quality_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventContext {
    pub project_type: String,
    pub language: String,
    pub framework: Option<String>,
    pub dependencies: Vec<String>,
    pub environment: serde_json::Value,
}

impl LearningSystem {
    pub fn new(config: LearningConfig) -> Self {
        let export_path = config.export_path.clone();

        Self {
            knowledge_base: KnowledgeBase::new(export_path.clone()),
            pattern_extractor: PatternExtractor::new(),
            success_tracker: SuccessTracker::new(),
            failure_analyzer: FailureAnalyzer::new(),
            decision_history: DecisionHistory::new(),
            decision_improver: DecisionImprover::new(),
            metrics: LearningMetrics::default(),
            config,
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        if self.config.import_on_startup {
            self.import_knowledge().await?;
        }

        info!("Learning system initialized with {} patterns", self.knowledge_base.pattern_count());

        Ok(())
    }

    pub async fn process_event(&mut self, event: LearningEvent) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        debug!("Processing learning event: {:?}", event.event_type);

        match event.event_type {
            LearningEventType::ImplementationSuccess => {
                self.handle_success(event).await?;
            }
            LearningEventType::ImplementationFailure => {
                self.handle_failure(event).await?;
            }
            LearningEventType::TestPassed | LearningEventType::ValidationCompleted => {
                self.reinforce_success(event).await?;
            }
            LearningEventType::TestFailed => {
                self.analyze_test_failure(event).await?;
            }
            LearningEventType::PatternIdentified => {
                self.add_pattern(event).await?;
            }
            LearningEventType::AntiPatternDetected => {
                self.add_anti_pattern(event).await?;
            }
            _ => {
                self.record_event(event).await?;
            }
        }

        self.update_metrics();

        if self.should_export() {
            self.export_knowledge().await?;
        }

        Ok(())
    }

    async fn handle_success(&mut self, event: LearningEvent) -> Result<()> {
        let success_metrics = SuccessMetrics::from_event(&event);
        self.success_tracker.track_success(event.clone(), success_metrics);

        if let Some(implementation) = &event.implementation {
            let patterns = self.pattern_extractor.extract_patterns(implementation, &event.context);

            for pattern in patterns {
                if pattern.quality_score() > self.config.min_pattern_quality {
                    self.knowledge_base.add_pattern(pattern)?;
                    self.metrics.patterns_learned += 1;
                }
            }
        }

        self.decision_improver.improve_from_success(&event);

        Ok(())
    }

    async fn handle_failure(&mut self, event: LearningEvent) -> Result<()> {
        let cause = self.failure_analyzer.analyze_failure(&event);

        if let Some(anti_pattern) = self.failure_analyzer.extract_anti_pattern(&event, &cause) {
            self.knowledge_base.add_anti_pattern(anti_pattern)?;
            self.metrics.anti_patterns_identified += 1;
        }

        self.decision_improver.improve_from_failure(&event, &cause);
        self.failure_analyzer.add_guard_conditions(&event, &cause);

        Ok(())
    }

    async fn reinforce_success(&mut self, event: LearningEvent) -> Result<()> {
        if let Some(pattern_id) = self.find_used_pattern(&event) {
            self.knowledge_base.reinforce_pattern(pattern_id)?;
        }

        self.success_tracker.reinforce(&event);
        Ok(())
    }

    async fn analyze_test_failure(&mut self, event: LearningEvent) -> Result<()> {
        let cause = self.failure_analyzer.analyze_test_failure(&event);
        self.failure_analyzer.learn_from_test_failure(&event, &cause);
        Ok(())
    }

    async fn add_pattern(&mut self, event: LearningEvent) -> Result<()> {
        if let Some(implementation) = &event.implementation {
            let patterns = self.pattern_extractor.extract_patterns(implementation, &event.context);

            for pattern in patterns {
                if pattern.quality_score() > self.config.min_pattern_quality {
                    self.knowledge_base.add_pattern(pattern)?;
                    self.metrics.patterns_learned += 1;
                }
            }
        }

        Ok(())
    }

    async fn add_anti_pattern(&mut self, event: LearningEvent) -> Result<()> {
        if let Some(anti_pattern) = self.failure_analyzer.extract_anti_pattern_from_event(&event) {
            self.knowledge_base.add_anti_pattern(anti_pattern)?;
            self.metrics.anti_patterns_identified += 1;
        }

        Ok(())
    }

    async fn record_event(&mut self, event: LearningEvent) -> Result<()> {
        self.decision_history.add_event(event);
        self.metrics.total_learning_events += 1;
        Ok(())
    }

    fn find_used_pattern(&self, event: &LearningEvent) -> Option<Uuid> {
        if let Some(implementation) = &event.implementation {
            self.knowledge_base.find_matching_pattern(implementation)
        } else {
            None
        }
    }

    pub fn find_similar_patterns(&self, spec: &Specification) -> Vec<Pattern> {
        self.knowledge_base.find_similar_patterns(spec, 10)
    }

    pub fn suggest_implementation(&self, spec: &Specification) -> Option<Implementation> {
        let patterns = self.find_similar_patterns(spec);

        if patterns.is_empty() {
            return None;
        }

        let best_pattern = patterns
            .into_iter()
            .max_by(|a, b| a.success_rate.partial_cmp(&b.success_rate).unwrap())?;

        self.knowledge_base.apply_pattern(&best_pattern, spec)
    }

    fn update_metrics(&mut self) {
        self.metrics.knowledge_base_size = self.knowledge_base.size();
        self.metrics.decision_accuracy = self.decision_improver.get_accuracy();
        self.metrics.llm_reduction_rate = self.calculate_llm_reduction();
        self.metrics.last_updated = Utc::now();

        if self.metrics.success_rate_trend.len() > 100 {
            self.metrics.success_rate_trend.remove(0);
        }

        let current_success_rate = self.success_tracker.get_success_rate();
        self.metrics.success_rate_trend.push(current_success_rate);
    }

    fn calculate_llm_reduction(&self) -> f32 {
        let pattern_usage = self.knowledge_base.get_usage_stats();
        let total_implementations = pattern_usage.total_uses as f32;
        let pattern_based = pattern_usage.pattern_hits as f32;

        if total_implementations > 0.0 { pattern_based / total_implementations } else { 0.0 }
    }

    fn should_export(&self) -> bool {
        self.metrics.total_learning_events % 10 == 0
    }

    pub async fn export_knowledge(&self) -> Result<()> {
        let export = self.knowledge_base.export()?;
        let export_path = self.config.export_path.join("knowledge_export.json");

        std::fs::create_dir_all(&self.config.export_path)?;
        let json = serde_json::to_string_pretty(&export)?;
        tokio::fs::write(export_path, json).await?;

        info!("Exported knowledge base with {} patterns", export.patterns.len());
        Ok(())
    }

    pub async fn import_knowledge(&mut self) -> Result<()> {
        let import_path = self.config.export_path.join("knowledge_export.json");

        if !import_path.exists() {
            debug!("No knowledge export found at {:?}", import_path);
            return Ok(());
        }

        let json = tokio::fs::read_to_string(import_path).await?;
        let export: KnowledgeExport = serde_json::from_str(&json)?;

        self.knowledge_base.import(export)?;

        info!("Imported knowledge base with {} patterns", self.knowledge_base.pattern_count());
        Ok(())
    }

    pub fn get_metrics(&self) -> &LearningMetrics {
        &self.metrics
    }

    pub fn get_learning_report(&self) -> LearningReport {
        LearningReport {
            metrics: self.metrics.clone(),
            top_patterns: self.knowledge_base.get_top_patterns(10),
            recent_decisions: self.decision_history.get_recent(10),
            improvement_trend: self.calculate_improvement_trend(),
        }
    }

    fn calculate_improvement_trend(&self) -> f32 {
        if self.metrics.success_rate_trend.len() < 2 {
            return 0.0;
        }

        let recent = &self.metrics.success_rate_trend[self.metrics.success_rate_trend.len() - 10..];
        let older =
            &self.metrics.success_rate_trend[..10.min(self.metrics.success_rate_trend.len())];

        let recent_avg: f32 = recent.iter().sum::<f32>() / recent.len() as f32;
        let older_avg: f32 = older.iter().sum::<f32>() / older.len() as f32;

        recent_avg - older_avg
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningReport {
    pub metrics: LearningMetrics,
    pub top_patterns: Vec<Pattern>,
    pub recent_decisions: Vec<LearningEvent>,
    pub improvement_trend: f32,
}

impl SuccessMetrics {
    fn from_event(event: &LearningEvent) -> Self {
        Self {
            compilation_success: event.outcome.success,
            tests_passed: matches!(event.event_type, LearningEventType::TestPassed),
            performance_met: event.metrics.code_quality_score > 0.8,
            security_passed: true,
            specification_coverage: event.metrics.test_coverage,
            implementation_time: event.metrics.duration,
            llm_calls_used: event.metrics.llm_calls,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_learning_system_initialization() {
        let config = LearningConfig::default();
        let mut system = LearningSystem::new(config);

        assert!(system.initialize().await.is_ok());
        assert_eq!(system.metrics.patterns_learned, 0);
    }

    #[tokio::test]
    async fn test_process_success_event() {
        let config = LearningConfig::default();
        let mut system = LearningSystem::new(config);

        let event = LearningEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: LearningEventType::ImplementationSuccess,
            specification: Specification::default(),
            implementation: Some(Implementation::default()),
            outcome: Outcome {
                success: true,
                score: 0.9,
                message: "Success".to_string(),
                details: serde_json::json!({}),
            },
            metrics: PerformanceMetrics {
                duration: Duration::from_secs(10),
                llm_calls: 5,
                memory_used: 1000,
                cpu_usage: 0.5,
                test_coverage: 0.8,
                code_quality_score: 0.85,
            },
            context: EventContext {
                project_type: "rust".to_string(),
                language: "rust".to_string(),
                framework: None,
                dependencies: vec![],
                environment: serde_json::json!({}),
            },
        };

        assert!(system.process_event(event).await.is_ok());
        assert!(system.metrics.total_learning_events > 0);
    }
}
