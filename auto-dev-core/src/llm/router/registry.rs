//! Model registry for managing available LLM providers

use crate::llm::provider::{ModelTier, ProviderCapabilities};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Central registry for all available models
pub struct ModelRegistry {
    models: Arc<RwLock<HashMap<String, ModelConfig>>>,
    providers: Arc<RwLock<HashMap<String, ProviderConfig>>>,
    tier_mappings: Arc<RwLock<HashMap<ModelTier, Vec<String>>>>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            models: Arc::new(RwLock::new(HashMap::new())),
            providers: Arc::new(RwLock::new(HashMap::new())),
            tier_mappings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load configuration from file
    pub async fn load_config(&self, config_path: &str) -> Result<()> {
        let config_str = tokio::fs::read_to_string(config_path).await?;
        let config: RegistryConfig = toml::from_str(&config_str)?;
        
        for model in config.models {
            self.register_model(model).await?;
        }
        
        for provider in config.providers {
            self.register_provider(provider).await?;
        }
        
        info!("Loaded {} models and {} providers from config",
              self.models.read().await.len(),
              self.providers.read().await.len());
        
        Ok(())
    }

    /// Register a new model
    pub async fn register_model(&self, config: ModelConfig) -> Result<()> {
        let model_id = config.id.clone();
        let tier = config.tier;
        
        // Add to models map
        self.models.write().await.insert(model_id.clone(), config.clone());
        
        // Update tier mappings
        self.tier_mappings.write().await
            .entry(tier)
            .or_insert_with(Vec::new)
            .push(model_id.clone());
        
        info!("Registered model: {} (tier: {:?})", model_id, tier);
        Ok(())
    }

    /// Register a provider configuration
    pub async fn register_provider(&self, config: ProviderConfig) -> Result<()> {
        self.providers.write().await.insert(config.name.clone(), config.clone());
        info!("Registered provider: {}", config.name);
        Ok(())
    }

    /// Get models for a specific tier
    pub async fn get_models_for_tier(&self, tier: ModelTier) -> Vec<ModelConfig> {
        let models = self.models.read().await;
        let tier_mappings = self.tier_mappings.read().await;
        
        if let Some(model_ids) = tier_mappings.get(&tier) {
            model_ids.iter()
                .filter_map(|id| models.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get all models sorted by tier
    pub async fn get_all_models(&self) -> Vec<ModelConfig> {
        let mut models: Vec<_> = self.models.read().await.values().cloned().collect();
        models.sort_by_key(|m| m.tier);
        models
    }

    /// Get a specific model by ID
    pub async fn get_model(&self, model_id: &str) -> Option<ModelConfig> {
        self.models.read().await.get(model_id).cloned()
    }

    /// Get provider configuration
    pub async fn get_provider(&self, provider_name: &str) -> Option<ProviderConfig> {
        self.providers.read().await.get(provider_name).cloned()
    }

    /// Find cheapest model for a tier
    pub async fn find_cheapest_model(&self, tier: ModelTier) -> Option<ModelConfig> {
        self.get_models_for_tier(tier).await
            .into_iter()
            .min_by(|a, b| {
                a.cost_per_1k_tokens.partial_cmp(&b.cost_per_1k_tokens).unwrap()
            })
    }

    /// Find fastest model for a tier
    pub async fn find_fastest_model(&self, tier: ModelTier) -> Option<ModelConfig> {
        self.get_models_for_tier(tier).await
            .into_iter()
            .min_by_key(|m| m.average_latency_ms)
    }

    /// Find model with largest context window
    pub async fn find_largest_context_model(&self, tier: ModelTier) -> Option<ModelConfig> {
        self.get_models_for_tier(tier).await
            .into_iter()
            .max_by_key(|m| m.context_window)
    }

    /// Update model availability
    pub async fn update_availability(&self, model_id: &str, available: bool) {
        if let Some(model) = self.models.write().await.get_mut(model_id) {
            model.available = available;
            if !available {
                warn!("Model {} marked as unavailable", model_id);
            }
        }
    }

    /// Get available models for a tier
    pub async fn get_available_models(&self, tier: ModelTier) -> Vec<ModelConfig> {
        self.get_models_for_tier(tier).await
            .into_iter()
            .filter(|m| m.available)
            .collect()
    }

    /// Create default registry with common models
    pub async fn create_default() -> Self {
        let registry = Self::new();
        
        // Register default models
        let default_models = vec![
            // Tier 0: No LLM
            ModelConfig {
                id: "heuristic".to_string(),
                name: "Heuristic Processor".to_string(),
                tier: ModelTier::NoLLM,
                provider: Provider::Local,
                cost_per_1k_tokens: 0.0,
                average_latency_ms: 1,
                context_window: 0,
                capabilities: vec![Capability::PatternMatching, Capability::Templates],
                available: true,
                local_path: None,
                api_endpoint: None,
                requires_auth: false,
            },
            // Tier 1: Tiny models
            ModelConfig {
                id: "qwen-0.5b".to_string(),
                name: "Qwen 2.5 Coder 0.5B".to_string(),
                tier: ModelTier::Tiny,
                provider: Provider::Local,
                cost_per_1k_tokens: 0.0,
                average_latency_ms: 50,
                context_window: 2048,
                capabilities: vec![
                    Capability::Classification,
                    Capability::SimpleQuestions,
                    Capability::RequirementChecking,
                ],
                available: true,
                local_path: Some(PathBuf::from("models/qwen2.5-coder-0.5b-instruct-q4_k_m.gguf")),
                api_endpoint: None,
                requires_auth: false,
            },
            ModelConfig {
                id: "phi-2".to_string(),
                name: "Microsoft Phi-2".to_string(),
                tier: ModelTier::Tiny,
                provider: Provider::Ollama,
                cost_per_1k_tokens: 0.0,
                average_latency_ms: 100,
                context_window: 2048,
                capabilities: vec![
                    Capability::Classification,
                    Capability::SimpleQuestions,
                ],
                available: false,
                local_path: None,
                api_endpoint: Some("http://localhost:11434".to_string()),
                requires_auth: false,
            },
            // Tier 2: Small models
            ModelConfig {
                id: "codellama-7b".to_string(),
                name: "Code Llama 7B".to_string(),
                tier: ModelTier::Small,
                provider: Provider::Ollama,
                cost_per_1k_tokens: 0.0,
                average_latency_ms: 500,
                context_window: 4096,
                capabilities: vec![
                    Capability::CodeGeneration,
                    Capability::CodeReview,
                    Capability::Testing,
                ],
                available: false,
                local_path: None,
                api_endpoint: Some("http://localhost:11434".to_string()),
                requires_auth: false,
            },
            ModelConfig {
                id: "mistral-7b".to_string(),
                name: "Mistral 7B".to_string(),
                tier: ModelTier::Small,
                provider: Provider::Together,
                cost_per_1k_tokens: 0.0002,
                average_latency_ms: 800,
                context_window: 8192,
                capabilities: vec![
                    Capability::CodeGeneration,
                    Capability::Questions,
                ],
                available: false,
                local_path: None,
                api_endpoint: Some("https://api.together.xyz".to_string()),
                requires_auth: true,
            },
            // Tier 3: Medium models
            ModelConfig {
                id: "mixtral-8x7b".to_string(),
                name: "Mixtral 8x7B".to_string(),
                tier: ModelTier::Medium,
                provider: Provider::Together,
                cost_per_1k_tokens: 0.0006,
                average_latency_ms: 1500,
                context_window: 32768,
                capabilities: vec![
                    Capability::CodeGeneration,
                    Capability::CodeReview,
                    Capability::Architecture,
                    Capability::Explanation,
                ],
                available: false,
                local_path: None,
                api_endpoint: Some("https://api.together.xyz".to_string()),
                requires_auth: true,
            },
            ModelConfig {
                id: "codellama-34b".to_string(),
                name: "Code Llama 34B".to_string(),
                tier: ModelTier::Medium,
                provider: Provider::Together,
                cost_per_1k_tokens: 0.0008,
                average_latency_ms: 2000,
                context_window: 16384,
                capabilities: vec![
                    Capability::CodeGeneration,
                    Capability::ComplexCode,
                    Capability::Testing,
                ],
                available: false,
                local_path: None,
                api_endpoint: Some("https://api.together.xyz".to_string()),
                requires_auth: true,
            },
            // Tier 4: Large models
            ModelConfig {
                id: "claude-3-opus".to_string(),
                name: "Claude 3 Opus".to_string(),
                tier: ModelTier::Large,
                provider: Provider::Anthropic,
                cost_per_1k_tokens: 0.015,
                average_latency_ms: 3000,
                context_window: 200000,
                capabilities: vec![
                    Capability::Architecture,
                    Capability::ComplexCode,
                    Capability::SystemDesign,
                    Capability::Creativity,
                ],
                available: false,
                local_path: None,
                api_endpoint: Some("https://api.anthropic.com".to_string()),
                requires_auth: true,
            },
            ModelConfig {
                id: "gpt-4-turbo".to_string(),
                name: "GPT-4 Turbo".to_string(),
                tier: ModelTier::Large,
                provider: Provider::OpenAI,
                cost_per_1k_tokens: 0.01,
                average_latency_ms: 2500,
                context_window: 128000,
                capabilities: vec![
                    Capability::Architecture,
                    Capability::ComplexCode,
                    Capability::SystemDesign,
                ],
                available: false,
                local_path: None,
                api_endpoint: Some("https://api.openai.com".to_string()),
                requires_auth: true,
            },
        ];
        
        for model in default_models {
            registry.register_model(model).await.unwrap();
        }
        
        info!("Created default registry with {} models", 
              registry.models.read().await.len());
        
        registry
    }
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub name: String,
    pub tier: ModelTier,
    pub provider: Provider,
    pub cost_per_1k_tokens: f64,
    pub average_latency_ms: u32,
    pub context_window: usize,
    pub capabilities: Vec<Capability>,
    pub available: bool,
    pub local_path: Option<PathBuf>,
    pub api_endpoint: Option<String>,
    pub requires_auth: bool,
}

impl ModelConfig {
    pub fn can_handle(&self, required_capabilities: &[Capability]) -> bool {
        required_capabilities.iter()
            .all(|cap| self.capabilities.contains(cap))
    }

    pub fn is_local(&self) -> bool {
        matches!(self.provider, Provider::Local | Provider::Ollama)
    }

    pub fn estimate_cost(&self, tokens: usize) -> f64 {
        (tokens as f64 / 1000.0) * self.cost_per_1k_tokens
    }
}

/// Provider types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Provider {
    Local,
    Ollama,
    OpenAI,
    Anthropic,
    Together,
    Replicate,
    Custom,
}

/// Model capabilities
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Capability {
    PatternMatching,
    Templates,
    Classification,
    SimpleQuestions,
    Questions,
    RequirementChecking,
    CodeGeneration,
    ComplexCode,
    CodeReview,
    Testing,
    Architecture,
    SystemDesign,
    Explanation,
    Creativity,
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub provider_type: Provider,
    pub api_key_env: Option<String>,
    pub base_url: Option<String>,
    pub default_timeout_ms: u32,
    pub max_retries: u32,
    pub rate_limit_rpm: Option<u32>,
}

/// Registry configuration file format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub models: Vec<ModelConfig>,
    pub providers: Vec<ProviderConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = ModelRegistry::create_default().await;
        
        let all_models = registry.get_all_models().await;
        assert!(all_models.len() > 5);
        
        // Check we have models for each tier
        for tier in [ModelTier::NoLLM, ModelTier::Tiny, ModelTier::Small,
                    ModelTier::Medium, ModelTier::Large] {
            let tier_models = registry.get_models_for_tier(tier).await;
            assert!(!tier_models.is_empty(), "No models for tier {:?}", tier);
        }
    }

    #[tokio::test]
    async fn test_model_selection() {
        let registry = ModelRegistry::create_default().await;
        
        // Find cheapest small model
        let cheapest = registry.find_cheapest_model(ModelTier::Small).await;
        assert!(cheapest.is_some());
        
        // Find fastest tiny model
        let fastest = registry.find_fastest_model(ModelTier::Tiny).await;
        assert!(fastest.is_some());
        
        // Check local models have zero cost
        let local_model = registry.get_model("qwen-0.5b").await.unwrap();
        assert_eq!(local_model.cost_per_1k_tokens, 0.0);
    }
}