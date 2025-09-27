//! Traits for LLM provider implementations
//!
//! This module defines the core traits that all LLM providers must implement,
//! enabling a unified interface across different models and APIs.

use super::types::*;
use crate::llm::errors::LLMError;
use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;

/// Core trait for LLM providers
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Get the name of this provider
    fn name(&self) -> &str;

    /// Get the model ID being used
    fn model(&self) -> &str;

    /// Check if the provider is available and configured
    async fn is_available(&self) -> bool;

    /// Get information about the model
    async fn model_info(&self) -> Result<ModelInfo, LLMError>;

    /// Complete a chat conversation
    async fn complete(
        &self,
        messages: Vec<Message>,
        options: CompletionOptions,
    ) -> Result<CompletionResponse, LLMError>;

    /// Stream a chat completion
    async fn complete_stream(
        &self,
        messages: Vec<Message>,
        options: CompletionOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>;

    /// Generate embeddings for text
    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, LLMError> {
        Err(LLMError::NotSupported {
            feature: "embeddings".to_string(),
            provider: self.name().to_string(),
        })
    }

    /// Get rate limit information
    fn rate_limits(&self) -> Option<RateLimit> {
        None
    }

    /// Get provider-specific metadata
    fn metadata(&self) -> Option<ProviderMetadata> {
        None
    }
}

/// Trait for providers that support caching
#[async_trait]
pub trait CachedProvider: LLMProvider {
    /// Get a cached response if available
    async fn get_cached(
        &self,
        messages: &[Message],
        options: &CompletionOptions,
    ) -> Option<CompletionResponse>;

    /// Cache a response
    async fn cache_response(
        &self,
        messages: &[Message],
        options: &CompletionOptions,
        response: &CompletionResponse,
    ) -> Result<(), LLMError>;

    /// Clear the cache
    async fn clear_cache(&self) -> Result<(), LLMError>;

    /// Get cache statistics
    async fn cache_stats(&self) -> CacheStats;
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub size_bytes: usize,
    pub entry_count: usize,
}

/// Trait for providers with retry logic
#[async_trait]
pub trait RetryableProvider: LLMProvider {
    /// Maximum number of retries
    fn max_retries(&self) -> usize {
        3
    }

    /// Backoff multiplier for exponential backoff
    fn backoff_multiplier(&self) -> f64 {
        2.0
    }

    /// Initial retry delay in milliseconds
    fn initial_retry_delay_ms(&self) -> u64 {
        1000
    }

    /// Check if an error is retryable
    fn is_retryable_error(&self, error: &LLMError) -> bool {
        matches!(
            error,
            LLMError::RateLimitExceeded { .. }
                | LLMError::Timeout { .. }
                | LLMError::NetworkError { .. }
        )
    }
}

/// Trait for providers that support fine-tuning
#[async_trait]
pub trait FineTunableProvider: LLMProvider {
    /// List available fine-tuned models
    async fn list_fine_tuned_models(&self) -> Result<Vec<String>, LLMError>;

    /// Create a fine-tuning job
    async fn create_fine_tuning_job(
        &self,
        training_data: Vec<Message>,
        validation_data: Option<Vec<Message>>,
        hyperparameters: FineTuningParams,
    ) -> Result<String, LLMError>;

    /// Get the status of a fine-tuning job
    async fn get_fine_tuning_status(&self, job_id: &str) -> Result<FineTuningStatus, LLMError>;
}

/// Fine-tuning parameters
#[derive(Debug, Clone)]
pub struct FineTuningParams {
    pub epochs: usize,
    pub batch_size: usize,
    pub learning_rate: f32,
    pub model_suffix: Option<String>,
}

/// Fine-tuning job status
#[derive(Debug, Clone)]
pub enum FineTuningStatus {
    Pending,
    Running { progress: f32 },
    Completed { model_id: String },
    Failed { error: String },
    Cancelled,
}

/// Trait for mock providers used in testing
#[cfg(test)]
#[async_trait]
pub trait MockProvider: LLMProvider {
    /// Set a predefined response for testing
    async fn set_mock_response(&mut self, response: CompletionResponse);

    /// Set a predefined error for testing
    async fn set_mock_error(&mut self, error: LLMError);

    /// Get the history of calls made to this provider
    async fn get_call_history(&self) -> Vec<(Vec<Message>, CompletionOptions)>;

    /// Clear the call history
    async fn clear_history(&mut self);
}

/// Builder trait for creating provider instances
pub trait ProviderBuilder {
    type Provider: LLMProvider;
    type Error;

    /// Build a provider instance
    fn build(self) -> Result<Self::Provider, Self::Error>;

    /// Set the API key
    fn api_key(self, key: impl Into<String>) -> Self;

    /// Set the base URL
    fn base_url(self, url: impl Into<String>) -> Self;

    /// Set the model to use
    fn model(self, model: impl Into<String>) -> Self;

    /// Set custom headers
    fn headers(self, headers: impl IntoIterator<Item = (String, String)>) -> Self;

    /// Set timeout in seconds
    fn timeout(self, seconds: u64) -> Self;
}
