pub mod decision_improver;
pub mod embeddings;
pub mod failure_analyzer;
pub mod knowledge_base;
pub mod learner;
pub mod pattern_extractor;
pub mod success_tracker;

#[cfg(test)]
mod integration_tests;

pub use decision_improver::{Decision, DecisionImprover, DecisionType};
pub use failure_analyzer::{FailureAnalyzer, FailureCause};
pub use knowledge_base::{KnowledgeBase, KnowledgeExport, PatternId};
pub use learner::{LearningConfig, LearningEvent, LearningEventType, LearningSystem};
pub use pattern_extractor::{Pattern, PatternContext, PatternExtractor};
pub use success_tracker::{SuccessMetrics, SuccessTracker};
