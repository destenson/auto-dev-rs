#![allow(unused)]
//! LLM Provider trait and implementations
//!
//! This module provides a unified interface for different LLM providers,
//! inspired by OpenRouter's approach but optimized for local models.

use super::{ClassificationResult, QuestionType};
use anyhow::{Result, Context};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Specification from parsed documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Specification {
    pub content: String,
    pub requirements: Vec<String>,
    pub examples: Vec<String>,
    pub acceptance_criteria: Vec<String>,
}

/// Project context for code generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub language: String,
    pub framework: Option<String>,
    pub existing_files: Vec<String>,
    pub patterns: Vec<String>,
    pub dependencies: Vec<String>,
}

/// Options for code generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationOptions {
    pub incremental: bool,
    pub test_first: bool,
    pub max_tokens: usize,
    pub temperature: f32,
    pub include_tests: bool,
}

impl Default for GenerationOptions {
    fn default() -> Self {
        Self {
            incremental: true,
            test_first: false,
            max_tokens: 4096,
            temperature: 0.2,
            include_tests: true,
        }
    }
}

/// Generated code result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedCode {
    pub files: Vec<GeneratedFile>,
    pub explanation: String,
    pub confidence: f32,
    pub tokens_used: usize,
    pub model_used: String,
    pub cached: bool,
}

/// A single generated file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
    pub language: String,
    pub is_test: bool,
}

/// Code review result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewResult {
    pub issues: Vec<Issue>,
    pub suggestions: Vec<String>,
    pub meets_requirements: bool,
    pub confidence: f32,
}

/// An issue found during code review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub severity: IssueSeverity,
    pub message: String,
    pub line: Option<usize>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

/// Requirement for code implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub id: String,
    pub description: String,
    pub priority: Priority,
    pub satisfied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

/// Explanation of implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Explanation {
    pub summary: String,
    pub details: Vec<String>,
    pub design_decisions: Vec<String>,
    pub trade_offs: Vec<String>,
}

/// Model tier for routing decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ModelTier {
    NoLLM,      // Heuristics only
    Tiny,       // 0.5B models like Qwen
    Small,      // 7B models
    Medium,     // 13-34B models
    Large,      // 70B+ models
}

/// Task complexity assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskComplexity {
    pub tier: ModelTier,
    pub reasoning: String,
    pub estimated_tokens: usize,
    pub confidence: f32,
}

/// Common trait for all LLM providers
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Get the name of this provider
    fn name(&self) -> &str;
    
    /// Get the model tier this provider serves
    fn tier(&self) -> ModelTier;
    
    /// Check if the provider is available
    async fn is_available(&self) -> bool;
    
    /// Get estimated cost per 1K tokens (in cents)
    fn cost_per_1k_tokens(&self) -> f32;
    
    /// Generate code from specification
    async fn generate_code(
        &self,
        spec: &Specification,
        context: &ProjectContext,
        options: &GenerationOptions,
    ) -> Result<GeneratedCode>;
    
    /// Explain implementation details
    async fn explain_implementation(
        &self,
        code: &str,
        spec: &Specification,
    ) -> Result<Explanation>;
    
    /// Review code against requirements
    async fn review_code(
        &self,
        code: &str,
        requirements: &[Requirement],
    ) -> Result<ReviewResult>;
    
    /// Simple Q&A (mainly for tiny models)
    async fn answer_question(&self, question: &str) -> Result<Option<String>>;
    
    /// Classify content (mainly for tiny models)
    async fn classify_content(&self, content: &str) -> Result<ClassificationResult>;
    
    /// Assess task complexity to determine which tier to use
    async fn assess_complexity(&self, task: &str) -> Result<TaskComplexity>;
    
    /// Simple text completion for prompts
    async fn complete_prompt(&self, prompt: &str) -> Result<String> {
        // Default implementation using answer_question
        self.answer_question(prompt).await
            .and_then(|opt| opt.ok_or_else(|| anyhow::anyhow!("No response from LLM")))
    }
}

/// Provider capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub can_generate_code: bool,
    pub can_explain: bool,
    pub can_review: bool,
    pub can_answer_questions: bool,
    pub can_classify: bool,
    pub supports_streaming: bool,
    pub max_context_tokens: usize,
    pub supports_function_calling: bool,
}

/// Provider status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub available: bool,
    pub response_time_ms: Option<u64>,
    pub error_message: Option<String>,
    pub requests_today: usize,
    pub tokens_used_today: usize,
}
