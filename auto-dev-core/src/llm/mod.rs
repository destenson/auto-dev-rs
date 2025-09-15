//! Lightweight LLM integration for simple tasks
//!
//! This module provides integration with tiny local models for basic
//! classification and simple Q&A tasks that don't require large models.

pub mod tiny;
pub mod classifier;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Simple question types that can be handled by tiny models
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestionType {
    Definition,      // "What is X?"
    YesNo,          // "Is this X?"
    Classification, // "What type is this?"
    Simple,         // Other simple questions
    Complex,        // Needs larger model
}

/// Result of content classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub is_code: bool,
    pub is_documentation: bool,
    pub is_test: bool,
    pub is_config: bool,
    pub language: Option<String>,
    pub confidence: f32,
}

/// Trait for tiny model operations
#[async_trait::async_trait]
pub trait TinyModel: Send + Sync {
    /// Check if content is code
    async fn is_code(&self, content: &str) -> Result<bool>;
    
    /// Classify content type
    async fn classify_content(&self, content: &str) -> Result<ClassificationResult>;
    
    /// Determine question complexity
    async fn classify_question(&self, question: &str) -> Result<QuestionType>;
    
    /// Answer simple definition questions
    async fn simple_answer(&self, question: &str) -> Result<Option<String>>;
    
    /// Check if a requirement is satisfied by code
    async fn check_requirement(&self, requirement: &str, code: &str) -> Result<bool>;
}

/// Configuration for tiny models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TinyModelConfig {
    /// Model to use (e.g., "qwen2.5-coder:0.5b")
    pub model: String,
    
    /// Host for local model server (e.g., Ollama)
    pub host: String,
    
    /// Maximum tokens for response
    pub max_tokens: usize,
    
    /// Temperature for generation (0.0-1.0)
    pub temperature: f32,
    
    /// Timeout in seconds
    pub timeout_secs: u64,
}

impl Default for TinyModelConfig {
    fn default() -> Self {
        Self {
            model: "qwen2.5-coder:0.5b".to_string(),
            host: "http://localhost:11434".to_string(),
            max_tokens: 256,
            temperature: 0.1,
            timeout_secs: 10,
        }
    }
}