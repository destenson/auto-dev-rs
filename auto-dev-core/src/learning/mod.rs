pub mod learner;
pub mod pattern_extractor;
pub mod success_tracker;
pub mod failure_analyzer;
pub mod knowledge_base;
pub mod decision_improver;
pub mod embeddings;

#[cfg(test)]
mod integration_tests;

pub use learner::{LearningSystem, LearningEvent, LearningEventType, LearningConfig};
pub use pattern_extractor::{PatternExtractor, Pattern, PatternContext};
pub use success_tracker::{SuccessTracker, SuccessMetrics};
pub use failure_analyzer::{FailureAnalyzer, FailureCause};
pub use knowledge_base::{KnowledgeBase, KnowledgeExport, PatternId};
pub use decision_improver::{DecisionImprover, DecisionType, Decision};