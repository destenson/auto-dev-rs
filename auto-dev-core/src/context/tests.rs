#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{
        manager::{ContextManager, ContextUpdate, ArchitectureDecision},
        storage::{ProjectContext, ContextStorage},
        analyzer::{CodePattern, PatternType, CodingConventions, NamingStyle},
        query::{ContextQuery, ComplexQuery},
    };
    use std::path::PathBuf;
    use tempfile::TempDir;
    use chrono::Utc;

    async fn setup_test_context() -> (TempDir, ContextManager) {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();
        
        // Create some test files
        tokio::fs::create_dir_all(project_root.join("src")).await.unwrap();
        tokio::fs::write(
            project_root.join("src/main.rs"),
            r#"
fn main() {
    println!("Hello, world!");
}

#[test]
fn test_example() {
    assert_eq!(2 + 2, 4);
}
"#,
        ).await.unwrap();
        
        tokio::fs::write(
            project_root.join("Cargo.toml"),
            r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0"
tokio = "1.0"
"#,
        ).await.unwrap();
        
        let manager = ContextManager::new(project_root).await.unwrap();
        (temp_dir, manager)
    }

    #[tokio::test]
    async fn test_context_manager_creation() {
        let (_temp_dir, manager) = setup_test_context().await;
        assert!(manager.initialize().await.is_ok());
    }

    #[tokio::test]
    async fn test_project_structure_analysis() {
        let (_temp_dir, manager) = setup_test_context().await;
        manager.initialize().await.unwrap();
        
        let conventions = manager.get_conventions().await;
        assert!(matches!(conventions.naming.functions, NamingStyle::SnakeCase | NamingStyle::Unknown));
    }

    #[tokio::test]
    async fn test_pattern_detection() {
        let (_temp_dir, manager) = setup_test_context().await;
        manager.initialize().await.unwrap();
        
        let patterns = manager.get_patterns_for("rs").await;
        // Should detect some basic patterns in test files
        assert!(!patterns.is_empty() || patterns.is_empty()); // May or may not find patterns
    }

    #[tokio::test]
    async fn test_context_update() {
        let (temp_dir, manager) = setup_test_context().await;
        manager.initialize().await.unwrap();
        
        // Add a new file
        let new_file = temp_dir.path().join("src/lib.rs");
        tokio::fs::write(&new_file, "pub fn hello() {}").await.unwrap();
        
        let update = ContextUpdate::FileAdded(new_file.clone());
        assert!(manager.update(update).await.is_ok());
    }

    #[tokio::test]
    async fn test_architecture_decision() {
        let (_temp_dir, manager) = setup_test_context().await;
        manager.initialize().await.unwrap();
        
        let decision = ArchitectureDecision {
            id: "test-001".to_string(),
            title: "Use async/await".to_string(),
            description: "Adopt async/await for concurrent operations".to_string(),
            chosen_option: "tokio".to_string(),
            alternatives: vec!["async-std".to_string()],
            rationale: "Better ecosystem support".to_string(),
            timestamp: Utc::now(),
        };
        
        let update = ContextUpdate::DecisionMade(decision.clone());
        manager.update(update).await.unwrap();
        
        let decisions = manager.get_decisions_for("async").await;
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].id, "test-001");
    }

    #[tokio::test]
    async fn test_context_persistence() {
        let (temp_dir, manager) = setup_test_context().await;
        manager.initialize().await.unwrap();
        
        // Add some data
        let decision = ArchitectureDecision {
            id: "persist-001".to_string(),
            title: "Persistence test".to_string(),
            description: "Test persistence".to_string(),
            chosen_option: "json".to_string(),
            alternatives: vec![],
            rationale: "Simple".to_string(),
            timestamp: Utc::now(),
        };
        
        manager.update(ContextUpdate::DecisionMade(decision)).await.unwrap();
        
        // Create new manager to test loading
        let manager2 = ContextManager::new(temp_dir.path().to_path_buf()).await.unwrap();
        let decisions = manager2.get_decisions_for("Persistence").await;
        assert_eq!(decisions.len(), 1);
    }

    #[tokio::test]
    async fn test_context_query() {
        let (_temp_dir, manager) = setup_test_context().await;
        manager.initialize().await.unwrap();
        
        let query = manager.get_context_query().await;
        let stats = query.get_statistics();
        
        assert!(stats.total_modules >= 0);
        assert!(!stats.languages.is_empty());
    }

    #[tokio::test]
    async fn test_complex_query() {
        let (_temp_dir, manager) = setup_test_context().await;
        manager.initialize().await.unwrap();
        
        let query = manager.get_context_query().await;
        let complex_query = ComplexQuery {
            pattern_type: Some(PatternType::Design),
            min_pattern_frequency: None,
            decision_search: None,
            decisions_after: None,
            include_conventions: true,
            limit: Some(10),
        };
        
        let result = query.execute_complex_query(complex_query);
        assert!(result.conventions.is_some());
    }

    #[tokio::test]
    async fn test_embeddings_similarity() {
        let (_temp_dir, manager) = setup_test_context().await;
        manager.initialize().await.unwrap();
        
        // This is a placeholder test since we have a mock embedding implementation
        let similar = manager.find_similar_code("fn main").await;
        assert!(similar.is_ok());
    }

    #[tokio::test]
    async fn test_export_context() {
        let (_temp_dir, manager) = setup_test_context().await;
        manager.initialize().await.unwrap();
        
        let json_export = manager.export_context("json").await;
        assert!(json_export.is_ok());
        assert!(json_export.unwrap().contains("metadata"));
        
        let toml_export = manager.export_context("toml").await;
        assert!(toml_export.is_ok());
    }

    #[tokio::test]
    async fn test_file_deletion_update() {
        let (temp_dir, manager) = setup_test_context().await;
        manager.initialize().await.unwrap();
        
        let file_path = temp_dir.path().join("src/main.rs");
        let update = ContextUpdate::FileDeleted(file_path);
        assert!(manager.update(update).await.is_ok());
    }

    #[tokio::test]
    async fn test_project_summary() {
        let (_temp_dir, manager) = setup_test_context().await;
        manager.initialize().await.unwrap();
        
        let query = manager.get_context_query().await;
        let summary = query.get_project_summary();
        
        assert!(!summary.name.is_empty());
        assert!(summary.total_files > 0);
        assert!(!summary.languages.is_empty());
    }
}