//! Error types for LLM operations
//!
//! This module defines strongly-typed errors for all LLM operations,
//! using thiserror for automatic error trait implementations.

use std::time::Duration;
use thiserror::Error;

/// Main error type for LLM operations
#[derive(Debug, Error)]
pub enum LLMError {
    /// API key is missing or invalid
    #[error("Authentication failed: {message}")]
    AuthenticationError { message: String },

    /// Rate limit has been exceeded
    #[error("Rate limit exceeded: {message}. Retry after {retry_after:?}")]
    RateLimitExceeded { message: String, retry_after: Option<Duration> },

    /// Request timed out
    #[error("Request timed out after {duration:?}")]
    Timeout { duration: Duration },

    /// Network error occurred
    #[error("Network error: {message}")]
    NetworkError { message: String },

    /// API returned an error
    #[error("API error (status {status}): {message}")]
    ApiError { status: u16, message: String },

    /// Invalid request parameters
    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },

    /// Model not found or not available
    #[error("Model '{model}' not found or not available")]
    ModelNotFound { model: String },

    /// Token limit exceeded
    #[error("Token limit exceeded: used {used} tokens, limit is {limit}")]
    TokenLimitExceeded { used: usize, limit: usize },

    /// Content was filtered
    #[error("Content filtered: {reason}")]
    ContentFiltered { reason: String },

    /// Feature not supported by provider
    #[error("Feature '{feature}' is not supported by provider '{provider}'")]
    NotSupported { feature: String, provider: String },

    /// Configuration error
    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },

    /// Parsing or serialization error
    #[error("Parsing error: {message}")]
    ParseError { message: String },

    /// Provider-specific error
    #[error("Provider error ({provider}): {message}")]
    ProviderError { provider: String, message: String },

    /// Cache operation failed
    #[error("Cache error: {message}")]
    CacheError { message: String },

    /// Streaming error
    #[error("Streaming error: {message}")]
    StreamingError { message: String },

    /// Function calling error
    #[error("Function calling error: {message}")]
    FunctionCallingError { message: String },

    /// Fine-tuning error
    #[error("Fine-tuning error: {message}")]
    FineTuningError { message: String },

    /// Generic error with context
    #[error("{context}: {message}")]
    Other { context: String, message: String },
}

impl LLMError {
    /// Create an authentication error
    pub fn auth(message: impl Into<String>) -> Self {
        Self::AuthenticationError { message: message.into() }
    }

    /// Create a rate limit error
    pub fn rate_limit(message: impl Into<String>, retry_after: Option<Duration>) -> Self {
        Self::RateLimitExceeded { message: message.into(), retry_after }
    }

    /// Create a timeout error
    pub fn timeout(duration: Duration) -> Self {
        Self::Timeout { duration }
    }

    /// Create a network error
    pub fn network(message: impl Into<String>) -> Self {
        Self::NetworkError { message: message.into() }
    }

    /// Create an API error
    pub fn api(status: u16, message: impl Into<String>) -> Self {
        Self::ApiError { status, message: message.into() }
    }

    /// Create an invalid request error
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest { message: message.into() }
    }

    /// Create a model not found error
    pub fn model_not_found(model: impl Into<String>) -> Self {
        Self::ModelNotFound { model: model.into() }
    }

    /// Create a token limit error
    pub fn token_limit(used: usize, limit: usize) -> Self {
        Self::TokenLimitExceeded { used, limit }
    }

    /// Create a content filtered error
    pub fn content_filtered(reason: impl Into<String>) -> Self {
        Self::ContentFiltered { reason: reason.into() }
    }

    /// Create a not supported error
    pub fn not_supported(feature: impl Into<String>, provider: impl Into<String>) -> Self {
        Self::NotSupported { feature: feature.into(), provider: provider.into() }
    }

    /// Create a configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::ConfigurationError { message: message.into() }
    }

    /// Create a parse error
    pub fn parse(message: impl Into<String>) -> Self {
        Self::ParseError { message: message.into() }
    }

    /// Create a provider-specific error
    pub fn provider(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ProviderError { provider: provider.into(), message: message.into() }
    }

    /// Create a cache error
    pub fn cache(message: impl Into<String>) -> Self {
        Self::CacheError { message: message.into() }
    }

    /// Create a streaming error
    pub fn streaming(message: impl Into<String>) -> Self {
        Self::StreamingError { message: message.into() }
    }

    /// Create a function calling error
    pub fn function_calling(message: impl Into<String>) -> Self {
        Self::FunctionCallingError { message: message.into() }
    }

    /// Create a fine-tuning error
    pub fn fine_tuning(message: impl Into<String>) -> Self {
        Self::FineTuningError { message: message.into() }
    }

    /// Create a generic error with context
    pub fn other(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Other { context: context.into(), message: message.into() }
    }
}

/// Result type for LLM operations
pub type LLMResult<T> = Result<T, LLMError>;

/// Convert from standard IO errors
impl From<std::io::Error> for LLMError {
    fn from(err: std::io::Error) -> Self {
        Self::NetworkError { message: err.to_string() }
    }
}

/// Convert from JSON errors
impl From<serde_json::Error> for LLMError {
    fn from(err: serde_json::Error) -> Self {
        Self::ParseError { message: err.to_string() }
    }
}

// Note: anyhow automatically provides From<LLMError> for anyhow::Error
// because LLMError implements std::error::Error through thiserror

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = LLMError::auth("Invalid API key");
        assert_eq!(err.to_string(), "Authentication failed: Invalid API key");

        let err = LLMError::rate_limit("Too many requests", Some(Duration::from_secs(60)));
        assert!(err.to_string().contains("Rate limit exceeded"));

        let err = LLMError::model_not_found("gpt-5");
        assert_eq!(err.to_string(), "Model 'gpt-5' not found or not available");
    }

    #[test]
    fn test_error_conversion() {
        let io_err =
            std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "Connection failed");
        let llm_err: LLMError = io_err.into();
        assert!(matches!(llm_err, LLMError::NetworkError { .. }));

        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let llm_err: LLMError = json_err.into();
        assert!(matches!(llm_err, LLMError::ParseError { .. }));
    }
}
