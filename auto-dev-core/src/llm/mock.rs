//! Mock LLM provider for testing
//!
//! This module provides a mock implementation of the LLMProvider trait
//! for use in unit tests only. It is not available in production builds.

#![cfg(test)]

use super::errors::LLMError;
use super::traits::{LLMProvider, MockProvider};
use super::types::*;
use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

/// Mock LLM provider for testing
#[derive(Clone)]
pub struct MockLLMProvider {
    name: String,
    model: String,
    responses: Arc<Mutex<Vec<CompletionResponse>>>,
    errors: Arc<Mutex<Vec<LLMError>>>,
    call_history: Arc<Mutex<Vec<(Vec<Message>, CompletionOptions)>>>,
    available: bool,
}

impl MockLLMProvider {
    /// Create a new mock provider
    pub fn new() -> Self {
        Self {
            name: "mock".to_string(),
            model: "mock-model".to_string(),
            responses: Arc::new(Mutex::new(Vec::new())),
            errors: Arc::new(Mutex::new(Vec::new())),
            call_history: Arc::new(Mutex::new(Vec::new())),
            available: true,
        }
    }

    /// Create a mock provider with a specific name and model
    pub fn with_model(name: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            model: model.into(),
            responses: Arc::new(Mutex::new(Vec::new())),
            errors: Arc::new(Mutex::new(Vec::new())),
            call_history: Arc::new(Mutex::new(Vec::new())),
            available: true,
        }
    }

    /// Set availability status
    pub fn set_available(&mut self, available: bool) {
        self.available = available;
    }

    /// Add a response to return
    pub fn add_response(&mut self, response: CompletionResponse) {
        self.responses.lock().unwrap().push(response);
    }

    /// Add an error to return
    pub fn add_error(&mut self, error: LLMError) {
        self.errors.lock().unwrap().push(error);
    }

    /// Create a simple text response
    pub fn simple_response(content: &str) -> CompletionResponse {
        CompletionResponse {
            id: "mock-response-1".to_string(),
            model: "mock-model".to_string(),
            choices: vec![Choice {
                index: 0,
                message: Message::assistant(content),
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            }),
            created: 1234567890,
        }
    }
}

impl Default for MockLLMProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LLMProvider for MockLLMProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn is_available(&self) -> bool {
        self.available
    }

    async fn model_info(&self) -> Result<ModelInfo, LLMError> {
        if !self.available {
            return Err(LLMError::model_not_found(&self.model));
        }

        Ok(ModelInfo {
            id: self.model.clone(),
            name: self.model.clone(),
            provider: self.name.clone(),
            context_length: 4096,
            max_output_tokens: 2048,
            supports_functions: true,
            supports_streaming: true,
            cost_per_1k_prompt: 0.001,
            cost_per_1k_completion: 0.002,
        })
    }

    async fn complete(
        &self,
        messages: Vec<Message>,
        options: CompletionOptions,
    ) -> Result<CompletionResponse, LLMError> {
        // Store call in history
        self.call_history
            .lock()
            .unwrap()
            .push((messages.clone(), options.clone()));

        // Check for errors first
        if let Some(error) = self.errors.lock().unwrap().pop() {
            return Err(error);
        }

        // Return a response if available
        if let Some(response) = self.responses.lock().unwrap().pop() {
            return Ok(response);
        }

        // Default response
        Ok(CompletionResponse {
            id: format!("mock-{}", uuid::Uuid::new_v4()),
            model: self.model.clone(),
            choices: vec![Choice {
                index: 0,
                message: Message::assistant("Mock response"),
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: Some(Usage {
                prompt_tokens: messages.len() * 10,
                completion_tokens: 10,
                total_tokens: messages.len() * 10 + 10,
            }),
            created: chrono::Utc::now().timestamp(),
        })
    }

    async fn complete_stream(
        &self,
        messages: Vec<Message>,
        options: CompletionOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError> {
        // Store call in history
        self.call_history
            .lock()
            .unwrap()
            .push((messages, options));

        // Check for errors first
        if let Some(error) = self.errors.lock().unwrap().pop() {
            return Err(error);
        }

        // Create a simple stream that yields one chunk
        let chunk = StreamChunk {
            id: format!("mock-stream-{}", uuid::Uuid::new_v4()),
            model: self.model.clone(),
            choices: vec![StreamChoice {
                index: 0,
                delta: Delta {
                    role: Some(Role::Assistant),
                    content: Some("Mock streaming response".to_string()),
                    function_call: None,
                },
                finish_reason: None,
            }],
            created: chrono::Utc::now().timestamp(),
        };

        let stream = futures::stream::once(async move { Ok(chunk) });
        Ok(Box::pin(stream))
    }
}

#[async_trait]
impl MockProvider for MockLLMProvider {
    async fn set_mock_response(&mut self, response: CompletionResponse) {
        self.responses.lock().unwrap().clear();
        self.responses.lock().unwrap().push(response);
    }

    async fn set_mock_error(&mut self, error: LLMError) {
        self.errors.lock().unwrap().clear();
        self.errors.lock().unwrap().push(error);
    }

    async fn get_call_history(&self) -> Vec<(Vec<Message>, CompletionOptions)> {
        self.call_history.lock().unwrap().clone()
    }

    async fn clear_history(&mut self) {
        self.call_history.lock().unwrap().clear();
        self.responses.lock().unwrap().clear();
        self.errors.lock().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_provider_basic() {
        let mut provider = MockLLMProvider::new();
        assert_eq!(provider.name(), "mock");
        assert_eq!(provider.model(), "mock-model");
        assert!(provider.is_available().await);

        // Test with custom response
        let custom_response = MockLLMProvider::simple_response("Test response");
        provider.set_mock_response(custom_response.clone()).await;

        let messages = vec![Message::user("Test message")];
        let options = CompletionOptions::default();
        let response = provider.complete(messages.clone(), options.clone()).await.unwrap();

        assert_eq!(response.choices[0].message.content, "Test response");

        // Check call history
        let history = provider.get_call_history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].0[0].content, "Test message");
    }

    #[tokio::test]
    async fn test_mock_provider_error() {
        let mut provider = MockLLMProvider::new();
        let error = LLMError::auth("Invalid API key");
        provider.set_mock_error(error).await;

        let messages = vec![Message::user("Test")];
        let options = CompletionOptions::default();
        let result = provider.complete(messages, options).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Authentication failed"));
    }

    #[tokio::test]
    async fn test_mock_provider_streaming() {
        let provider = MockLLMProvider::new();
        let messages = vec![Message::user("Test streaming")];
        let options = CompletionOptions::default();

        let stream = provider.complete_stream(messages, options).await.unwrap();
        let chunks: Vec<_> = futures::stream::StreamExt::collect(stream).await;

        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].is_ok());
        let chunk = chunks[0].as_ref().unwrap();
        assert!(chunk.choices[0].delta.content.as_ref().unwrap().contains("Mock streaming"));
    }
}