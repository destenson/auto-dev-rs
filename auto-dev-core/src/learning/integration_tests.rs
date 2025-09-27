#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::dev_loop::{Event, EventType, LoopConfig, Orchestrator};
    use crate::incremental::Implementation;
    use crate::parser::model::Specification;
    use chrono::Utc;
    use std::path::PathBuf;
    use tokio::sync::mpsc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_learning_system_integration() {
        let config = LearningConfig::default();
        let mut learning_system = LearningSystem::new(config);

        assert!(learning_system.initialize().await.is_ok());

        let event = LearningEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: LearningEventType::ImplementationSuccess,
            specification: Specification::default(),
            implementation: Some(Implementation::default()),
            outcome: learner::Outcome {
                success: true,
                score: 0.9,
                message: "Test success".to_string(),
                details: serde_json::json!({}),
            },
            metrics: learner::PerformanceMetrics {
                duration: std::time::Duration::from_secs(10),
                llm_calls: 5,
                memory_used: 1000,
                cpu_usage: 0.5,
                test_coverage: 0.8,
                code_quality_score: 0.85,
            },
            context: learner::EventContext {
                project_type: "rust".to_string(),
                language: "rust".to_string(),
                framework: None,
                dependencies: vec![],
                environment: serde_json::json!({}),
            },
        };

        assert!(learning_system.process_event(event).await.is_ok());

        let metrics = learning_system.get_metrics();
        assert!(metrics.total_learning_events > 0);
    }

    #[tokio::test]
    async fn test_orchestrator_with_learning() {
        let config = LoopConfig::default();
        let (_tx, rx) = mpsc::channel(1);
        let orchestrator = Orchestrator::new(config, rx);

        let event = Event::new(EventType::SpecificationChanged, PathBuf::from("test.md"));

        assert!(orchestrator.queue_event(event).await.is_ok());

        let metrics = orchestrator.get_metrics().await;
        assert_eq!(metrics.events_processed, 0);
    }

    #[tokio::test]
    async fn test_pattern_learning_and_reuse() {
        let config = LearningConfig::default();
        let mut learning_system = LearningSystem::new(config);
        learning_system.initialize().await.unwrap();

        let specification = Specification::default();

        let patterns_before = learning_system.find_similar_patterns(&specification);
        assert_eq!(patterns_before.len(), 0);

        let event = LearningEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: LearningEventType::PatternIdentified,
            specification: specification.clone(),
            implementation: Some(Implementation::default()),
            outcome: learner::Outcome {
                success: true,
                score: 0.95,
                message: "Pattern identified".to_string(),
                details: serde_json::json!({}),
            },
            metrics: learner::PerformanceMetrics {
                duration: std::time::Duration::from_secs(5),
                llm_calls: 0,
                memory_used: 500,
                cpu_usage: 0.3,
                test_coverage: 0.9,
                code_quality_score: 0.9,
            },
            context: learner::EventContext {
                project_type: "rust".to_string(),
                language: "rust".to_string(),
                framework: None,
                dependencies: vec![],
                environment: serde_json::json!({}),
            },
        };

        learning_system.process_event(event).await.unwrap();

        let patterns_after = learning_system.find_similar_patterns(&specification);
        assert!(patterns_after.len() > 0 || learning_system.knowledge_base.pattern_count() > 0);
    }

    #[tokio::test]
    async fn test_failure_learning() {
        let config = LearningConfig::default();
        let mut learning_system = LearningSystem::new(config);
        learning_system.initialize().await.unwrap();

        let event = LearningEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: LearningEventType::ImplementationFailure,
            specification: Specification::default(),
            implementation: Some(Implementation::default()),
            outcome: learner::Outcome {
                success: false,
                score: 0.2,
                message: "Compilation error: syntax error".to_string(),
                details: serde_json::json!({
                    "error": "unexpected token"
                }),
            },
            metrics: learner::PerformanceMetrics {
                duration: std::time::Duration::from_secs(2),
                llm_calls: 3,
                memory_used: 800,
                cpu_usage: 0.4,
                test_coverage: 0.0,
                code_quality_score: 0.3,
            },
            context: learner::EventContext {
                project_type: "rust".to_string(),
                language: "rust".to_string(),
                framework: None,
                dependencies: vec![],
                environment: serde_json::json!({}),
            },
        };

        learning_system.process_event(event).await.unwrap();

        let metrics = learning_system.get_metrics();
        assert_eq!(metrics.total_learning_events, 1);
    }

    #[tokio::test]
    async fn test_decision_improvement() {
        let config = LearningConfig::default();
        let mut learning_system = LearningSystem::new(config);
        learning_system.initialize().await.unwrap();

        let initial_accuracy = learning_system.decision_improver.get_accuracy();

        let success_event = LearningEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: LearningEventType::ImplementationSuccess,
            specification: Specification::default(),
            implementation: Some(Implementation::default()),
            outcome: learner::Outcome {
                success: true,
                score: 0.9,
                message: "Success".to_string(),
                details: serde_json::json!({}),
            },
            metrics: learner::PerformanceMetrics {
                duration: std::time::Duration::from_secs(10),
                llm_calls: 2,
                memory_used: 1000,
                cpu_usage: 0.5,
                test_coverage: 0.85,
                code_quality_score: 0.9,
            },
            context: learner::EventContext {
                project_type: "rust".to_string(),
                language: "rust".to_string(),
                framework: None,
                dependencies: vec![],
                environment: serde_json::json!({}),
            },
        };

        learning_system.process_event(success_event).await.unwrap();

        let decision_type = decision_improver::DecisionType::Implementation;
        let confidence = learning_system.decision_improver.get_decision_confidence(&decision_type);

        assert!(confidence >= 0.0 && confidence <= 1.0);
    }

    #[tokio::test]
    async fn test_knowledge_persistence() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage_path = temp_dir.path().to_path_buf();

        let config = LearningConfig { export_path: storage_path.clone(), ..Default::default() };

        let mut learning_system = LearningSystem::new(config.clone());
        learning_system.initialize().await.unwrap();

        let event = LearningEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: LearningEventType::PatternIdentified,
            specification: Specification::default(),
            implementation: Some(Implementation::default()),
            outcome: learner::Outcome {
                success: true,
                score: 0.9,
                message: "Pattern learned".to_string(),
                details: serde_json::json!({}),
            },
            metrics: learner::PerformanceMetrics {
                duration: std::time::Duration::from_secs(5),
                llm_calls: 1,
                memory_used: 500,
                cpu_usage: 0.3,
                test_coverage: 0.8,
                code_quality_score: 0.85,
            },
            context: learner::EventContext {
                project_type: "rust".to_string(),
                language: "rust".to_string(),
                framework: None,
                dependencies: vec![],
                environment: serde_json::json!({}),
            },
        };

        learning_system.process_event(event).await.unwrap();
        learning_system.export_knowledge().await.unwrap();

        let mut new_learning_system = LearningSystem::new(config);
        new_learning_system.initialize().await.unwrap();
        new_learning_system.import_knowledge().await.unwrap();

        assert!(new_learning_system.knowledge_base.pattern_count() > 0);
    }
}
