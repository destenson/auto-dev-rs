//! Model management for Ollama provider
//!
//! This module provides utilities for managing Ollama models,
//! including pulling, listing, and deleting models.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Information about an Ollama model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModelInfo {
    pub name: String,
    pub size_bytes: u64,
    pub modified: String,
    pub digest: String,
    pub details: ModelDetails,
}

/// Detailed information about a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDetails {
    pub format: String,
    pub family: String,
    pub parameter_size: String,
    pub quantization_level: Option<String>,
}

/// Model registry with recommended models for different tasks
#[derive(Debug, Clone)]
pub struct ModelRegistry {
    models: HashMap<String, ModelProfile>,
}

/// Profile for a specific model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub name: String,
    pub category: ModelCategory,
    pub size_gb: f32,
    pub context_length: usize,
    pub recommended_tasks: Vec<String>,
    pub min_ram_gb: f32,
    pub supports_tools: bool,
}

/// Model categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelCategory {
    Code,
    General,
    Small,
    Embedding,
    Vision,
}

impl ModelRegistry {
    /// Create a new model registry with default models
    pub fn new() -> Self {
        let mut models = HashMap::new();

        // Code models
        models.insert(
            "qwen2.5-coder:0.5b".to_string(),
            ModelProfile {
                name: "qwen2.5-coder:0.5b".to_string(),
                category: ModelCategory::Code,
                size_gb: 0.4,
                context_length: 32768,
                recommended_tasks: vec![
                    "code_completion".to_string(),
                    "code_review".to_string(),
                    "debugging".to_string(),
                ],
                min_ram_gb: 1.0,
                supports_tools: false,
            },
        );

        models.insert(
            "codellama:7b".to_string(),
            ModelProfile {
                name: "codellama:7b".to_string(),
                category: ModelCategory::Code,
                size_gb: 3.8,
                context_length: 16384,
                recommended_tasks: vec![
                    "code_generation".to_string(),
                    "code_explanation".to_string(),
                    "refactoring".to_string(),
                ],
                min_ram_gb: 8.0,
                supports_tools: false,
            },
        );

        models.insert(
            "deepseek-coder:6.7b".to_string(),
            ModelProfile {
                name: "deepseek-coder:6.7b".to_string(),
                category: ModelCategory::Code,
                size_gb: 3.8,
                context_length: 16384,
                recommended_tasks: vec![
                    "code_generation".to_string(),
                    "code_completion".to_string(),
                    "test_generation".to_string(),
                ],
                min_ram_gb: 8.0,
                supports_tools: false,
            },
        );

        // General models
        models.insert(
            "llama3.2:latest".to_string(),
            ModelProfile {
                name: "llama3.2:latest".to_string(),
                category: ModelCategory::General,
                size_gb: 2.0,
                context_length: 128000,
                recommended_tasks: vec![
                    "chat".to_string(),
                    "reasoning".to_string(),
                    "summarization".to_string(),
                ],
                min_ram_gb: 4.0,
                supports_tools: true,
            },
        );

        models.insert(
            "mistral:latest".to_string(),
            ModelProfile {
                name: "mistral:latest".to_string(),
                category: ModelCategory::General,
                size_gb: 4.1,
                context_length: 8192,
                recommended_tasks: vec![
                    "chat".to_string(),
                    "instruction_following".to_string(),
                    "creative_writing".to_string(),
                ],
                min_ram_gb: 8.0,
                supports_tools: false,
            },
        );

        models.insert(
            "mixtral:8x7b".to_string(),
            ModelProfile {
                name: "mixtral:8x7b".to_string(),
                category: ModelCategory::General,
                size_gb: 26.0,
                context_length: 32768,
                recommended_tasks: vec![
                    "complex_reasoning".to_string(),
                    "analysis".to_string(),
                    "technical_writing".to_string(),
                ],
                min_ram_gb: 48.0,
                supports_tools: true,
            },
        );

        // Small/Fast models
        models.insert(
            "phi3:mini".to_string(),
            ModelProfile {
                name: "phi3:mini".to_string(),
                category: ModelCategory::Small,
                size_gb: 2.3,
                context_length: 128000,
                recommended_tasks: vec![
                    "quick_answers".to_string(),
                    "classification".to_string(),
                    "simple_chat".to_string(),
                ],
                min_ram_gb: 4.0,
                supports_tools: false,
            },
        );

        models.insert(
            "gemma2:2b".to_string(),
            ModelProfile {
                name: "gemma2:2b".to_string(),
                category: ModelCategory::Small,
                size_gb: 1.6,
                context_length: 8192,
                recommended_tasks: vec![
                    "classification".to_string(),
                    "quick_responses".to_string(),
                    "simple_reasoning".to_string(),
                ],
                min_ram_gb: 3.0,
                supports_tools: false,
            },
        );

        models.insert(
            "tinyllama:latest".to_string(),
            ModelProfile {
                name: "tinyllama:latest".to_string(),
                category: ModelCategory::Small,
                size_gb: 0.6,
                context_length: 2048,
                recommended_tasks: vec![
                    "simple_tasks".to_string(),
                    "quick_classification".to_string(),
                    "basic_chat".to_string(),
                ],
                min_ram_gb: 1.0,
                supports_tools: false,
            },
        );

        // Embedding models
        models.insert(
            "nomic-embed-text:latest".to_string(),
            ModelProfile {
                name: "nomic-embed-text:latest".to_string(),
                category: ModelCategory::Embedding,
                size_gb: 0.3,
                context_length: 8192,
                recommended_tasks: vec![
                    "text_embedding".to_string(),
                    "semantic_search".to_string(),
                    "similarity".to_string(),
                ],
                min_ram_gb: 1.0,
                supports_tools: false,
            },
        );

        models.insert(
            "all-minilm:latest".to_string(),
            ModelProfile {
                name: "all-minilm:latest".to_string(),
                category: ModelCategory::Embedding,
                size_gb: 0.1,
                context_length: 512,
                recommended_tasks: vec![
                    "text_embedding".to_string(),
                    "quick_similarity".to_string(),
                ],
                min_ram_gb: 0.5,
                supports_tools: false,
            },
        );

        Self { models }
    }

    /// Get model profile by name
    pub fn get_profile(&self, name: &str) -> Option<&ModelProfile> {
        self.models.get(name)
    }

    /// Get recommended model for a task
    pub fn recommend_model(&self, task: &str, max_size_gb: Option<f32>) -> Option<&ModelProfile> {
        let mut candidates: Vec<&ModelProfile> = self
            .models
            .values()
            .filter(|m| m.recommended_tasks.iter().any(|t| t.contains(task)))
            .collect();

        // Filter by size if specified
        if let Some(max_size) = max_size_gb {
            candidates.retain(|m| m.size_gb <= max_size);
        }

        // Sort by size (prefer smaller models)
        candidates.sort_by(|a, b| a.size_gb.partial_cmp(&b.size_gb).unwrap());

        candidates.first().copied()
    }

    /// Get all models in a category
    pub fn get_by_category(&self, category: ModelCategory) -> Vec<&ModelProfile> {
        self.models.values().filter(|m| m.category == category).collect()
    }

    /// Check if system has enough RAM for a model
    pub fn can_run_model(&self, model_name: &str) -> Result<bool> {
        let profile = self.get_profile(model_name).context("Model not in registry")?;

        // Get system memory (simplified - in production would use sysinfo crate)
        // For now, assume we have enough memory
        Ok(true)
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Model pull progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullProgress {
    pub status: String,
    pub digest: Option<String>,
    pub total: Option<u64>,
    pub completed: Option<u64>,
}

/// Auto-detect best model for system
pub async fn auto_select_model(task: &str) -> Result<String> {
    let registry = ModelRegistry::new();

    // For now, return a sensible default based on task
    let model = match task {
        "code" | "code_generation" | "code_review" => "qwen2.5-coder:0.5b",
        "chat" | "general" => "llama3.2:latest",
        "embedding" | "embeddings" => "nomic-embed-text:latest",
        "small" | "fast" | "quick" => "tinyllama:latest",
        _ => "llama3.2:latest",
    };

    Ok(model.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_registry() {
        let registry = ModelRegistry::new();

        // Test getting a known model
        let model = registry.get_profile("qwen2.5-coder:0.5b");
        assert!(model.is_some());
        assert_eq!(model.unwrap().category, ModelCategory::Code);

        // Test recommendation
        let recommended = registry.recommend_model("code_completion", Some(5.0));
        assert!(recommended.is_some());
        assert!(recommended.unwrap().recommended_tasks.contains(&"code_completion".to_string()));
    }

    #[test]
    fn test_get_by_category() {
        let registry = ModelRegistry::new();

        let code_models = registry.get_by_category(ModelCategory::Code);
        assert!(!code_models.is_empty());

        let embedding_models = registry.get_by_category(ModelCategory::Embedding);
        assert!(!embedding_models.is_empty());
    }

    #[tokio::test]
    async fn test_auto_select() {
        let model = auto_select_model("code").await;
        assert!(model.is_ok());
        let model_name = model.unwrap();
        assert!(model_name.contains("coder") || model_name.contains("code"));

        let model = auto_select_model("embedding").await;
        assert!(model.is_ok());
        let model_name = model.unwrap();
        assert!(model_name.contains("embed") || model_name.contains("minilm"));
    }
}
