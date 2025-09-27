//! Ollama provider for local and remote model execution
//!
//! This module provides integration with Ollama for running LLMs locally
//! or connecting to remote Ollama instances. It supports model management,
//! streaming responses, and embeddings generation.

use super::traits::{LLMProvider, RetryableProvider};
use super::types::*;
use crate::llm::config::OllamaConfig;
use crate::llm::errors::LLMError;
use async_trait::async_trait;
use futures::stream::Stream;
use ollama_rs::generation::chat::{ChatMessage, request::ChatMessageRequest};
use ollama_rs::generation::embeddings::request::{GenerateEmbeddingsRequest, EmbeddingsInput};
use ollama_rs::generation::embeddings::GenerateEmbeddingsResponse;
use ollama_rs::models::ModelOptions;
use ollama_rs::Ollama;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// Ollama provider implementation
pub struct OllamaProvider {
    client: Arc<Ollama>,
    config: OllamaConfig,
    model_name: String,
    model_cache: Arc<RwLock<Option<ModelInfo>>>,
}

impl OllamaProvider {
    /// Create a new Ollama provider instance (doesn't connect yet)
    pub fn new(config: OllamaConfig) -> Self {
        // Build the URL from host and port with appropriate protocol
        let protocol = if config.use_https { "https" } else { "http" };
        // TODO: check that Ollama::new() expects the url to contain the port
        let url = format!("{}://{}:{}", protocol, config.host, config.port);
        let client = Ollama::new(url, config.port);
        
        Self {
            client: Arc::new(client),
            config: config.clone(),
            model_name: config.default_model.clone(),
            model_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new Ollama provider from configuration
    pub fn create(config: OllamaConfig) -> Result<Self, LLMError> {
        Ok(Self::new(config))
    }

    /// Create provider with custom model
    pub fn with_model(mut self, model: String) -> Self {
        self.model_name = model;
        self
    }

    /// Auto-detect local Ollama installation
    pub async fn detect_local() -> Option<Self> {
        let default_config = OllamaConfig::default();
        
        // Try to connect to default local endpoint
        let url = format!("http://{}:{}", default_config.host, default_config.port);
        let client = Ollama::new(url, default_config.port);
        
        // Try to list models to verify connection
        match client.list_local_models().await {
            Ok(_) => Some(Self::new(default_config)),
            Err(_) => None,
        }
    }

    /// Convert our Message type to Ollama's ChatMessage
    fn convert_message(&self, msg: &Message) -> ChatMessage {
        match msg.role {
            Role::System => ChatMessage::system(msg.content.clone()),
            Role::User => ChatMessage::user(msg.content.clone()),
            Role::Assistant => ChatMessage::assistant(msg.content.clone()),
            Role::Function => ChatMessage::assistant(format!("Function output: {}", msg.content)),
        }
    }

    /// Convert Ollama options from our CompletionOptions
    fn convert_options(&self, options: &CompletionOptions) -> ModelOptions {
        let mut model_options = ModelOptions::default();
        
        if let Some(temp) = options.temperature {
            model_options = model_options.temperature(temp);
        }
        
        if let Some(max_tokens) = options.max_tokens {
            // Convert usize to i32, clamping to i32::MAX if needed
            let max_tokens_i32 = if max_tokens > i32::MAX as usize {
                i32::MAX
            } else {
                max_tokens as i32
            };
            model_options = model_options.num_predict(max_tokens_i32);
        }
        
        if let Some(top_p) = options.top_p {
            model_options = model_options.top_p(top_p as f32);
        }
        
        if let Some(stop) = &options.stop {
            model_options = model_options.stop(stop.clone());
        }
        
        model_options
    }

    /// Check if a model is available locally
    pub async fn is_model_available(&self, model: &str) -> bool {
        match self.client.list_local_models().await {
            Ok(models) => models.iter().any(|m| m.name == model),
            Err(_) => false,
        }
    }

    /// Pull a model if not available
    pub async fn ensure_model(&self, model: &str) -> Result<(), LLMError> {
        if !self.is_model_available(model).await {
            self.client
                .pull_model(model.to_string(), false)
                .await
                .map_err(|_e| LLMError::ModelNotFound {
                    model: model.to_string(),
                })?;
        }
        Ok(())
    }
}

/// Model management functionality
impl OllamaProvider {
    /// List available models
    pub async fn list_models(&self) -> Result<Vec<String>, LLMError> {
        let models = self.client
            .list_local_models()
            .await
            .map_err(|e| LLMError::NetworkError {
                message: e.to_string(),
            })?;
        
        Ok(models.into_iter().map(|m| m.name).collect())
    }

    /// Pull a model from the Ollama library
    pub async fn pull_model(&self, model: &str) -> Result<(), LLMError> {
        self.client
            .pull_model(model.to_string(), false)
            .await
            .map(|_status| ()) // Convert PullModelStatus to ()
            .map_err(|_e| LLMError::ModelNotFound {
                model: model.to_string(),
            })
    }

    /// Delete a model
    pub async fn delete_model(&self, model: &str) -> Result<(), LLMError> {
        self.client
            .delete_model(model.to_string())
            .await
            .map_err(|e| LLMError::NetworkError {
                message: format!("Failed to delete model: {}", e),
            })
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    fn model(&self) -> &str {
        &self.model_name
    }

    async fn is_available(&self) -> bool {
        // Check if we can connect and list models
        self.client.list_local_models().await.is_ok()
    }

    async fn model_info(&self) -> Result<ModelInfo, LLMError> {
        // Check cache first
        if let Some(info) = self.model_cache.read().await.as_ref() {
            return Ok(info.clone());
        }

        // Get model details from Ollama
        let models = self.client
            .list_local_models()
            .await
            .map_err(|e| LLMError::NetworkError {
                message: e.to_string(),
            })?;

        let _model = models
            .iter()
            .find(|m| m.name == self.model_name)
            .ok_or_else(|| LLMError::ModelNotFound {
                model: self.model_name.clone(),
            })?;

        let info = ModelInfo {
            id: self.model_name.clone(),
            name: self.model_name.clone(),
            provider: "ollama".to_string(),
            context_length: self.config.default_context_length,
            max_output_tokens: self.config.default_context_length,
            supports_functions: false,
            supports_streaming: true,
            cost_per_1k_prompt: 0.0, // Local models are free
            cost_per_1k_completion: 0.0,
        };

        // Cache the info
        *self.model_cache.write().await = Some(info.clone());

        Ok(info)
    }

    async fn complete(
        &self,
        messages: Vec<Message>,
        options: CompletionOptions,
    ) -> Result<CompletionResponse, LLMError> {
        // Ensure model is available
        self.ensure_model(&self.model_name).await?;

        // Convert messages
        let chat_messages: Vec<ChatMessage> = messages.iter().map(|m| self.convert_message(m)).collect();

        // Create request
        let request = ChatMessageRequest::new(
            self.model_name.clone(),
            chat_messages,
        ).options(self.convert_options(&options));

        // Send request
        let response = self.client
            .send_chat_messages(request)
            .await
            .map_err(|e| LLMError::NetworkError {
                message: e.to_string(),
            })?;

        // Extract content from response
        let content = response.message.content;

        // Extract usage information if available
        let usage = response.final_data.as_ref().map(|data| {
            Usage {
                prompt_tokens: data.eval_count as usize,
                completion_tokens: data.prompt_eval_count as usize,
                total_tokens: (data.eval_count + data.prompt_eval_count) as usize,
            }
        });

        // Convert response
        Ok(CompletionResponse {
            id: uuid::Uuid::new_v4().to_string(),
            model: self.model_name.clone(),
            created: chrono::Utc::now().timestamp(),
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: Role::Assistant,
                    content,
                    function_call: None,
                    name: None,
                },
                finish_reason: Some(FinishReason::Stop),
            }],
            usage,
        })
    }

    async fn complete_stream(
        &self,
        messages: Vec<Message>,
        options: CompletionOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError> {
        // Ensure model is available
        self.ensure_model(&self.model_name).await?;

        // Convert messages
        let chat_messages: Vec<ChatMessage> = messages.iter().map(|m| self.convert_message(m)).collect();

        // Create request
        let request = ChatMessageRequest::new(
            self.model_name.clone(),
            chat_messages,
        ).options(self.convert_options(&options));

        // Note: Ollama-rs requires the "stream" feature to be enabled for streaming
        // For now, we'll return a simple non-streaming implementation wrapped in a stream
        let response = self.client
            .send_chat_messages(request)
            .await
            .map_err(|e| LLMError::NetworkError {
                message: e.to_string(),
            })?;

        // Create a single-item stream with the full response
        let model_name = self.model_name.clone();
        let content = response.message.content;
        
        let converted_stream = async_stream::stream! {
            yield Ok(StreamChunk {
                id: uuid::Uuid::new_v4().to_string(),
                model: model_name.clone(),
                created: chrono::Utc::now().timestamp(),
                choices: vec![StreamChoice {
                    index: 0,
                    delta: Delta {
                        content: Some(content),
                        role: None,
                        function_call: None,
                    },
                    finish_reason: Some(FinishReason::Stop),
                }],
            });
        };

        Ok(Box::pin(converted_stream))
    }

    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, LLMError> {
        // Use embedding model if specified, otherwise use default
        let model = request.model.unwrap_or_else(|| "nomic-embed-text".to_string());
        
        // Ensure embedding model is available
        self.ensure_model(&model).await?;

        // Convert input to EmbeddingsInput
        let embeddings_input = if request.input.len() == 1 {
            EmbeddingsInput::Single(request.input[0].clone())
        } else {
            EmbeddingsInput::Multiple(request.input.clone())
        };

        // Create Ollama embedding request
        let ollama_request = GenerateEmbeddingsRequest::new(
            model.clone(),
            embeddings_input,
        );

        // Send request
        let response: GenerateEmbeddingsResponse = self.client
            .generate_embeddings(ollama_request)
            .await
            .map_err(|e| LLMError::NetworkError {
                message: format!("Embedding generation failed: {}", e),
            })?;

        // Convert response - Ollama returns Vec<Vec<f32>>, we need the first one
        let embedding = response.embeddings.first()
            .ok_or_else(|| LLMError::NetworkError {
                message: "No embeddings returned".to_string(),
            })?
            .clone();

        Ok(EmbeddingResponse {
            data: vec![Embedding {
                embedding,
                index: 0,
            }],
            model,
            usage: Usage {
                prompt_tokens: request.input.iter().map(|s| s.len() / 4).sum(), // Rough token estimate
                completion_tokens: 0,
                total_tokens: request.input.iter().map(|s| s.len() / 4).sum(),
            },
        })
    }

    fn rate_limits(&self) -> Option<RateLimit> {
        // Local models don't have rate limits
        None
    }

    fn metadata(&self) -> Option<ProviderMetadata> {
        Some(ProviderMetadata {
            provider: "Ollama".to_string(),
            api_version: Some("v1".to_string()),
            region: Some("local".to_string()),
            custom_fields: HashMap::new(),
        })
    }
}

impl RetryableProvider for OllamaProvider {
    // Use default retry settings
}

/// Builder for OllamaProvider
pub struct OllamaProviderBuilder {
    config: OllamaConfig,
    model: Option<String>,
}

impl OllamaProviderBuilder {
    pub fn new() -> Self {
        Self {
            config: OllamaConfig::default(),
            model: None,
        }
    }

    pub fn host(mut self, host: String) -> Self {
        self.config.host = host;
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    pub fn model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }

    pub fn timeout(mut self, timeout: u64) -> Self {
        self.config.timeout_secs = timeout;
        self
    }

    pub fn use_https(mut self, use_https: bool) -> Self {
        self.config.use_https = use_https;
        self
    }

    pub fn build(self) -> Result<OllamaProvider, LLMError> {
        let mut provider = OllamaProvider::new(self.config);
        if let Some(model) = self.model {
            provider = provider.with_model(model);
        }
        Ok(provider)
    }
}

impl Default for OllamaProviderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ollama_provider_creation() {
        let config = OllamaConfig::default();
        let provider = OllamaProvider::create(config);
        assert!(provider.is_ok());
    }

    #[tokio::test]
    async fn test_ollama_provider_https() {
        let mut config = OllamaConfig::default();
        config.use_https = true;
        let provider = OllamaProvider::create(config);
        assert!(provider.is_ok());
    }

    #[tokio::test]
    async fn test_builder() {
        let provider = OllamaProviderBuilder::new()
            .host("localhost".to_string())
            .port(11434)
            .model("llama3.2".to_string())
            .build();
        
        assert!(provider.is_ok());
        if let Ok(p) = provider {
            assert_eq!(p.model(), "llama3.2");
            assert_eq!(p.name(), "ollama");
        }
    }

    #[tokio::test]
    #[ignore] // Requires Ollama to be running
    async fn test_local_detection() {
        let provider = OllamaProvider::detect_local().await;
        // This test will pass if Ollama is running locally
        if provider.is_some() {
            let p = provider.unwrap();
            assert!(p.is_available().await);
        }
    }

    #[tokio::test]
    #[ignore] // Requires Ollama to be running
    async fn test_model_listing() {
        let config = OllamaConfig::default();
        if let Ok(provider) = OllamaProvider::create(config) {
            if provider.is_available().await {
                let models = provider.list_models().await;
                assert!(models.is_ok());
            }
        }
    }
}
