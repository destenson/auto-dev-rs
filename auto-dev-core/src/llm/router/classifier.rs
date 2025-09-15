//! Task complexity classification for intelligent routing

use crate::llm::provider::{ModelTier, TaskComplexity};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main complexity classifier that combines rule-based and ML approaches
pub struct ComplexityClassifier {
    rule_classifier: RuleBasedClassifier,
    ml_classifier: Option<MLClassifier>,
    feature_extractor: FeatureExtractor,
}

impl ComplexityClassifier {
    pub fn new() -> Self {
        Self {
            rule_classifier: RuleBasedClassifier::new(),
            ml_classifier: None,
            feature_extractor: FeatureExtractor::new(),
        }
    }

    pub fn with_ml_classifier(mut self, ml_classifier: MLClassifier) -> Self {
        self.ml_classifier = Some(ml_classifier);
        self
    }

    pub fn classify(&self, task: &Task) -> Result<ModelTier> {
        // First try rule-based classification
        let rule_tier = self.rule_classifier.classify(task);

        // If ML classifier is available and confidence is low, use it
        if let Some(ml) = &self.ml_classifier {
            let features = self.feature_extractor.extract(task);
            let (ml_tier, confidence) = ml.predict(&features)?;

            // If ML confidence is high, prefer it over rules
            if confidence > 0.7 {
                return Ok(ml_tier);
            }

            // For medium confidence, average the two
            if confidence > 0.4 {
                return Ok(self.combine_tiers(rule_tier, ml_tier));
            }
        }

        Ok(rule_tier)
    }

    fn combine_tiers(&self, tier1: ModelTier, tier2: ModelTier) -> ModelTier {
        // Take the higher tier for safety
        if tier1 > tier2 { tier1 } else { tier2 }
    }
}

/// Rule-based classifier for quick decisions
pub struct RuleBasedClassifier {
    patterns: HashMap<String, ModelTier>,
    keywords: HashMap<String, ModelTier>,
}

impl RuleBasedClassifier {
    pub fn new() -> Self {
        let mut patterns = HashMap::new();
        let mut keywords = HashMap::new();

        // Tier 0: No LLM patterns
        patterns.insert("format".to_string(), ModelTier::NoLLM);
        patterns.insert("template".to_string(), ModelTier::NoLLM);
        patterns.insert("rename".to_string(), ModelTier::NoLLM);

        // Tier 1: Tiny model keywords
        keywords.insert("simple".to_string(), ModelTier::Tiny);
        keywords.insert("basic".to_string(), ModelTier::Tiny);
        keywords.insert("comment".to_string(), ModelTier::Tiny);
        keywords.insert("classify".to_string(), ModelTier::Tiny);

        // Tier 2: Small model keywords
        keywords.insert("function".to_string(), ModelTier::Small);
        keywords.insert("method".to_string(), ModelTier::Small);
        keywords.insert("test".to_string(), ModelTier::Small);

        // Tier 3: Medium model keywords
        keywords.insert("module".to_string(), ModelTier::Medium);
        keywords.insert("class".to_string(), ModelTier::Medium);
        keywords.insert("integration".to_string(), ModelTier::Medium);

        // Tier 4: Large model keywords
        keywords.insert("architecture".to_string(), ModelTier::Large);
        keywords.insert("design".to_string(), ModelTier::Large);
        keywords.insert("system".to_string(), ModelTier::Large);
        keywords.insert("complex".to_string(), ModelTier::Large);

        Self { patterns, keywords }
    }

    pub fn classify(&self, task: &Task) -> ModelTier {
        // Check if task can use pattern matching
        if self.can_use_pattern(task) || self.can_use_template(task) {
            return ModelTier::NoLLM;
        }

        // Check task type
        let tier = match task.task_type {
            TaskType::Formatting => ModelTier::NoLLM,
            TaskType::SimpleRefactor => ModelTier::Tiny,
            TaskType::CommentGeneration => ModelTier::Tiny,
            TaskType::SingleFunction => ModelTier::Small,
            TaskType::SimpleTest => ModelTier::Small,
            TaskType::MultiFunction => ModelTier::Medium,
            TaskType::Integration => ModelTier::Medium,
            TaskType::Architecture => ModelTier::Large,
            TaskType::ApiDesign => ModelTier::Large,
            TaskType::Unknown => self.classify_by_size(task),
        };

        // Check for keyword overrides
        let keyword_tier = self.check_keywords(&task.description);
        if keyword_tier > tier {
            return keyword_tier;
        }

        tier
    }

    fn can_use_pattern(&self, task: &Task) -> bool {
        task.description.to_lowercase().contains("pattern")
            || task.description.to_lowercase().contains("regex")
            || task.description.to_lowercase().contains("find and replace")
    }

    fn can_use_template(&self, task: &Task) -> bool {
        task.description.to_lowercase().contains("template")
            || task.description.to_lowercase().contains("boilerplate")
    }

    fn classify_by_size(&self, task: &Task) -> ModelTier {
        if task.estimated_lines < 10 {
            ModelTier::Tiny
        } else if task.estimated_lines < 50 {
            ModelTier::Small
        } else if task.estimated_lines < 200 {
            ModelTier::Medium
        } else {
            ModelTier::Large
        }
    }

    fn check_keywords(&self, description: &str) -> ModelTier {
        let lower = description.to_lowercase();
        let mut highest_tier = ModelTier::NoLLM;

        for (keyword, tier) in &self.keywords {
            if lower.contains(keyword) && *tier > highest_tier {
                highest_tier = *tier;
            }
        }

        highest_tier
    }
}

/// ML-based classifier for more nuanced decisions
#[derive(Clone)]
pub struct MLClassifier {
    model: ClassificationModel,
    threshold: f32,
}

impl MLClassifier {
    pub fn new(model_path: Option<&str>) -> Result<Self> {
        Ok(Self { model: ClassificationModel::load(model_path)?, threshold: 0.7 })
    }

    pub fn predict(&self, features: &TaskFeatures) -> Result<(ModelTier, f32)> {
        let (tier, confidence) = self.model.predict(features)?;

        // Adjust based on confidence
        if confidence < self.threshold {
            // Bump up one tier for safety
            let safer_tier = self.next_tier(tier);
            Ok((safer_tier, confidence))
        } else {
            Ok((tier, confidence))
        }
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

/// Simple classification model (placeholder for real ML)
#[derive(Clone)]
pub struct ClassificationModel {
    weights: Vec<f32>,
}

impl ClassificationModel {
    pub fn load(_model_path: Option<&str>) -> Result<Self> {
        // Placeholder: In production, load actual model weights
        Ok(Self { weights: vec![0.1, 0.2, 0.3, 0.4, 0.5] })
    }

    pub fn predict(&self, features: &TaskFeatures) -> Result<(ModelTier, f32)> {
        // Simple weighted scoring (placeholder)
        let score = features.token_count as f32 * self.weights[0]
            + features.complexity_score * self.weights[1]
            + (features.has_architecture_keywords as u8 as f32) * self.weights[2]
            + (features.requires_context as u8 as f32) * self.weights[3]
            + features.estimated_difficulty * self.weights[4];

        let tier = if score < 10.0 {
            ModelTier::NoLLM
        } else if score < 50.0 {
            ModelTier::Tiny
        } else if score < 200.0 {
            ModelTier::Small
        } else if score < 500.0 {
            ModelTier::Medium
        } else {
            ModelTier::Large
        };

        let confidence = (1.0 - (score % 50.0) / 50.0).min(0.95).max(0.3);

        Ok((tier, confidence))
    }
}

/// Feature extraction for ML classification
pub struct FeatureExtractor {
    architecture_keywords: Vec<String>,
    complexity_patterns: Vec<String>,
}

impl FeatureExtractor {
    pub fn new() -> Self {
        Self {
            architecture_keywords: vec![
                "design".to_string(),
                "architecture".to_string(),
                "pattern".to_string(),
                "system".to_string(),
                "interface".to_string(),
                "api".to_string(),
            ],
            complexity_patterns: vec![
                "async".to_string(),
                "concurrent".to_string(),
                "distributed".to_string(),
                "optimize".to_string(),
                "performance".to_string(),
            ],
        }
    }

    pub fn extract(&self, task: &Task) -> TaskFeatures {
        let lower_desc = task.description.to_lowercase();

        TaskFeatures {
            token_count: task.description.len() / 4,
            has_architecture_keywords: self
                .architecture_keywords
                .iter()
                .any(|k| lower_desc.contains(k)),
            requires_context: task.context_size > 1000,
            complexity_score: self.calculate_complexity(&task),
            estimated_difficulty: self.estimate_difficulty(&task),
            has_tests: lower_desc.contains("test"),
            is_refactoring: lower_desc.contains("refactor"),
            involves_multiple_files: task.affected_files > 1,
            requires_creativity: lower_desc.contains("create")
                || lower_desc.contains("design")
                || lower_desc.contains("implement"),
        }
    }

    fn calculate_complexity(&self, task: &Task) -> f32 {
        let mut score = 0.0;
        let lower_desc = task.description.to_lowercase();

        // Check for complexity indicators
        for pattern in &self.complexity_patterns {
            if lower_desc.contains(pattern) {
                score += 10.0;
            }
        }

        // Factor in size
        score += (task.estimated_lines as f32).log2();

        // Factor in context requirements
        if task.context_size > 5000 {
            score += 20.0;
        }

        score
    }

    fn estimate_difficulty(&self, task: &Task) -> f32 {
        let mut difficulty = 1.0;

        // Increase for certain task types
        difficulty *= match task.task_type {
            TaskType::Architecture => 5.0,
            TaskType::ApiDesign => 4.0,
            TaskType::Integration => 3.0,
            TaskType::MultiFunction => 2.5,
            TaskType::SingleFunction => 1.5,
            _ => 1.0,
        };

        // Increase for multiple files
        if task.affected_files > 3 {
            difficulty *= 1.5;
        }

        difficulty
    }
}

/// Task representation for classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub description: String,
    pub task_type: TaskType,
    pub estimated_lines: usize,
    pub context_size: usize,
    pub affected_files: usize,
}

/// Task types for classification
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TaskType {
    Formatting,
    SimpleRefactor,
    CommentGeneration,
    SingleFunction,
    SimpleTest,
    MultiFunction,
    Integration,
    Architecture,
    ApiDesign,
    Unknown,
}

/// Features extracted from a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFeatures {
    pub token_count: usize,
    pub has_architecture_keywords: bool,
    pub requires_context: bool,
    pub complexity_score: f32,
    pub estimated_difficulty: f32,
    pub has_tests: bool,
    pub is_refactoring: bool,
    pub involves_multiple_files: bool,
    pub requires_creativity: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_based_classification() {
        let classifier = RuleBasedClassifier::new();

        let task = Task {
            description: "Format the code according to rustfmt".to_string(),
            task_type: TaskType::Formatting,
            estimated_lines: 100,
            context_size: 0,
            affected_files: 1,
        };

        assert_eq!(classifier.classify(&task), ModelTier::NoLLM);

        let task = Task {
            description: "Design a new microservices architecture".to_string(),
            task_type: TaskType::Architecture,
            estimated_lines: 1000,
            context_size: 10000,
            affected_files: 20,
        };

        assert_eq!(classifier.classify(&task), ModelTier::Large);
    }

    #[test]
    fn test_feature_extraction() {
        let extractor = FeatureExtractor::new();

        let task = Task {
            description: "Create a simple function to add two numbers".to_string(),
            task_type: TaskType::SingleFunction,
            estimated_lines: 10,
            context_size: 100,
            affected_files: 1,
        };

        let features = extractor.extract(&task);
        assert!(features.token_count > 0);
        assert!(!features.has_architecture_keywords);
        assert!(features.requires_creativity);
    }
}
