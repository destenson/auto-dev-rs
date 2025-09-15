#![allow(unused)]
//! OpenAI-compatible provider for Groq, Lambda Labs, and other services
//!
//! Many providers offer OpenAI-compatible APIs with different base URLs

use super::{
    provider::*,
    openai::{extract_code_files, ChatMessage, ChatCompletionRequest, ChatCompletionResponse},
    ClassificationResult,
};
use anyhow::{Result, Context};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// OpenAI-compatible provider that can work with multiple services
pub struct OpenAICompatProvider {
    client: Client,
    config: OpenAICompatConfig,
    model_tier: ModelTier,
}

impl OpenAICompatProvider {
    pub fn new(config: OpenAICompatConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()?;
        
        // Determine tier based on provider and model
        let model_tier = Self::determine_tier(&config.provider, &config.model);
        
        Ok(Self {
            client,
            config,
            model_tier,
        })
    }
    
    /// Create a Groq provider
    pub fn groq(model: String, api_key_env: String) -> Result<Self> {
        let config = OpenAICompatConfig {
            provider: "groq".to_string(),
            base_url: "https://api.groq.com/openai/v1".to_string(),
            api_key_env,
            model,
            max_tokens: 4096,
            temperature: 0.2,
            timeout_secs: 30,
            auth_header: "Authorization".to_string(),
            auth_prefix: "Bearer".to_string(),
        };
        Self::new(config)
    }
    
    /// Create a Lambda Labs provider
    pub fn lambda_labs(model: String, api_key_env: String) -> Result<Self> {
        let config = OpenAICompatConfig {
            provider: "lambda".to_string(),
            base_url: "https://api.lambdalabs.com/v1".to_string(),
            api_key_env,
            model,
            max_tokens: 4096,
            temperature: 0.2,
            timeout_secs: 60,
            auth_header: "Authorization".to_string(),
            auth_prefix: "Bearer".to_string(),
        };
        Self::new(config)
    }
    
    /// Create a Together AI provider
    pub fn together(model: String, api_key_env: String) -> Result<Self> {
        let config = OpenAICompatConfig {
            provider: "together".to_string(),
            base_url: "https://api.together.xyz/v1".to_string(),
            api_key_env,
            model,
            max_tokens: 4096,
            temperature: 0.2,
            timeout_secs: 60,
            auth_header: "Authorization".to_string(),
            auth_prefix: "Bearer".to_string(),
        };
        Self::new(config)
    }
    
    /// Create a Perplexity provider
    pub fn perplexity(model: String, api_key_env: String) -> Result<Self> {
        let config = OpenAICompatConfig {
            provider: "perplexity".to_string(),
            base_url: "https://api.perplexity.ai".to_string(),
            api_key_env,
            model,
            max_tokens: 4096,
            temperature: 0.2,
            timeout_secs: 60,
            auth_header: "Authorization".to_string(),
            auth_prefix: "Bearer".to_string(),
        };
        Self::new(config)
    }
    
    /// Create a custom OpenAI-compatible provider
    pub fn custom(config: OpenAICompatConfig) -> Result<Self> {
        Self::new(config)
    }
    
    fn determine_tier(provider: &str, model: &str) -> ModelTier {
        let model_lower = model.to_lowercase();
        
        // Groq models
        if provider == "groq" {
            if model_lower.contains("mixtral-8x7b") || model_lower.contains("gemma-7b") {
                return ModelTier::Small;
            } else if model_lower.contains("llama3-70b") || model_lower.contains("mixtral-8x22b") {
                return ModelTier::Medium;
            } else if model_lower.contains("llama3-8b") || model_lower.contains("llama2-70b") {
                return ModelTier::Small;
            }
        }
        
        // Lambda Labs models
        if provider == "lambda" {
            if model_lower.contains("hermes-3-llama-3.1-405b") {
                return ModelTier::Large;
            }
        }
        
        // Together AI models
        if provider == "together" {
            if model_lower.contains("qwen") && model_lower.contains("0.5b") {
                return ModelTier::Tiny;
            } else if model_lower.contains("7b") || model_lower.contains("8b") {
                return ModelTier::Small;
            } else if model_lower.contains("70b") || model_lower.contains("72b") {
                return ModelTier::Medium;
            } else if model_lower.contains("405b") || model_lower.contains("175b") {
                return ModelTier::Large;
            }
        }
        
        // Generic size detection
        if model_lower.contains("0.5b") || model_lower.contains("1b") || model_lower.contains("tiny") {
            ModelTier::Tiny
        } else if model_lower.contains("7b") || model_lower.contains("8b") || model_lower.contains("small") {
            ModelTier::Small
        } else if model_lower.contains("70b") || model_lower.contains("34b") || model_lower.contains("medium") {
            ModelTier::Medium
        } else if model_lower.contains("405b") || model_lower.contains("175b") || model_lower.contains("large") {
            ModelTier::Large
        } else {
            ModelTier::Medium // Default
        }
    }
    
    async fn chat_completion(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let api_key = std::env::var(&self.config.api_key_env)
            .context(format!("{} API key not found", self.config.provider))?;
        
        let request = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages,
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
        };
        
        let mut req = self.client
            .post(format!("{}/chat/completions", self.config.base_url));
        
        // Add authentication header
        if self.config.auth_prefix.is_empty() {
            req = req.header(&self.config.auth_header, api_key);
        } else {
            req = req.header(&self.config.auth_header, 
                           format!("{} {}", self.config.auth_prefix, api_key));
        }
        
        let response = req
            .json(&request)
            .send()
            .await
            .context(format!("Failed to send request to {}", self.config.provider))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("{} API error: {}", self.config.provider, error_text));
        }
        
        let result: ChatCompletionResponse = response.json().await
            .context(format!("Failed to parse {} response", self.config.provider))?;
        
        result.choices.first()
            .and_then(|c| Some(c.message.content.clone()))
            .ok_or_else(|| anyhow::anyhow!("No response from {}", self.config.provider))
    }
}

#[async_trait]
impl LLMProvider for OpenAICompatProvider {
    fn name(&self) -> &str {
        &self.config.provider
    }
    
    fn tier(&self) -> ModelTier {
        self.model_tier
    }
    
    async fn is_available(&self) -> bool {
        std::env::var(&self.config.api_key_env).is_ok()
    }
    
    fn cost_per_1k_tokens(&self) -> f32 {
        // Rough estimates for different providers
        match self.config.provider.as_str() {
            "groq" => 0.001, // Groq is generally cheaper
            "lambda" => 0.002,
            "together" => 0.001,
            "perplexity" => 0.002,
            _ => 0.002,
        }
    }
    
    async fn generate_code(
        &self,
        spec: &Specification,
        context: &ProjectContext,
        options: &GenerationOptions,
    ) -> Result<GeneratedCode> {
        let system_prompt = format!(
            "You are an expert {} developer. Generate clean, well-documented code \
             that follows best practices and existing project patterns.",
            context.language
        );
        
        let user_prompt = format!(
            "Project context:\n\
             - Language: {}\n\
             - Framework: {:?}\n\
             - Existing patterns: {:?}\n\n\
             Specification:\n{}\n\n\
             Requirements:\n{}\n\n\
             Generate implementation code.",
            context.language,
            context.framework,
            context.patterns,
            spec.content,
            spec.requirements.join("\n")
        );
        
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_prompt,
            },
        ];
        
        let response = self.chat_completion(messages).await?;
        
        // Parse code blocks from response
        let files = extract_code_files(&response);
        
        Ok(GeneratedCode {
            files,
            explanation: format!("Generated by {} ({})", self.config.provider, self.config.model),
            confidence: 0.8,
            tokens_used: response.len() / 4, // Rough estimate
            model_used: format!("{}:{}", self.config.provider, self.config.model),
            cached: false,
        })
    }
    
    async fn explain_implementation(
        &self,
        code: &str,
        spec: &Specification,
    ) -> Result<Explanation> {
        let prompt = format!(
            "Explain how this code implements the specification:\n\n\
             Code:\n{}\n\n\
             Specification:\n{}",
            code, spec.content
        );
        
        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ];
        
        let response = self.chat_completion(messages).await?;
        
        Ok(Explanation {
            summary: response.clone(),
            details: vec![],
            design_decisions: vec![],
            trade_offs: vec![],
        })
    }
    
    async fn review_code(
        &self,
        code: &str,
        requirements: &[Requirement],
    ) -> Result<ReviewResult> {
        let req_list = requirements.iter()
            .map(|r| format!("- {}: {}", r.id, r.description))
            .collect::<Vec<_>>()
            .join("\n");
        
        let prompt = format!(
            "Review this code against the requirements:\n\n\
             Code:\n{}\n\n\
             Requirements:\n{}",
            code, req_list
        );
        
        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ];
        
        let response = self.chat_completion(messages).await?;
        
        // Simple parsing
        let meets_requirements = !response.to_lowercase().contains("not met") && 
                                !response.to_lowercase().contains("missing");
        
        Ok(ReviewResult {
            issues: vec![],
            suggestions: vec![response],
            meets_requirements,
            confidence: 0.75,
        })
    }
    
    async fn answer_question(&self, question: &str) -> Result<Option<String>> {
        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: question.to_string(),
            },
        ];
        
        let response = self.chat_completion(messages).await?;
        Ok(Some(response))
    }
    
    async fn classify_content(&self, content: &str) -> Result<ClassificationResult> {
        let prompt = format!(
            "Classify this content. Return JSON: \
             {{\"is_code\": bool, \"is_doc\": bool, \"is_test\": bool, \
              \"is_config\": bool, \"language\": \"name or null\"}}\n\n\
             Content:\n{}",
            &content[..content.len().min(500)]
        );
        
        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ];
        
        let response = self.chat_completion(messages).await?;
        
        // Try to parse JSON
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&response) {
            return Ok(ClassificationResult {
                is_code: parsed["is_code"].as_bool().unwrap_or(false),
                is_documentation: parsed["is_doc"].as_bool().unwrap_or(false),
                is_test: parsed["is_test"].as_bool().unwrap_or(false),
                is_config: parsed["is_config"].as_bool().unwrap_or(false),
                language: parsed["language"].as_str().map(String::from),
                confidence: 0.8,
            });
        }
        
        // Fallback
        Ok(ClassificationResult {
            is_code: false,
            is_documentation: false,
            is_test: false,
            is_config: false,
            language: None,
            confidence: 0.1,
        })
    }
    
    async fn assess_complexity(&self, task: &str) -> Result<TaskComplexity> {
        let prompt = format!(
            "Assess the complexity of this task. Answer with: \
             simple, moderate, complex, or very complex.\n\n\
             Task: {}",
            task
        );
        
        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ];
        
        let response = self.chat_completion(messages).await?;
        let lower = response.to_lowercase();
        
        let tier = if lower.contains("simple") {
            ModelTier::Tiny
        } else if lower.contains("moderate") {
            ModelTier::Small
        } else if lower.contains("very complex") {
            ModelTier::Large
        } else {
            ModelTier::Medium
        };
        
        Ok(TaskComplexity {
            tier,
            reasoning: response,
            estimated_tokens: task.len() / 4 * 2,
            confidence: 0.75,
        })
    }
}

/// Configuration for OpenAI-compatible providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAICompatConfig {
    pub provider: String,
    pub base_url: String,
    pub api_key_env: String,
    pub model: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub timeout_secs: u64,
    pub auth_header: String,
    pub auth_prefix: String,
}

impl OpenAICompatConfig {
    /// Create config for Groq
    pub fn groq(model: &str) -> Self {
        Self {
            provider: "groq".to_string(),
            base_url: "https://api.groq.com/openai/v1".to_string(),
            api_key_env: "GROQ_API_KEY".to_string(),
            model: model.to_string(),
            max_tokens: 4096,
            temperature: 0.2,
            timeout_secs: 30,
            auth_header: "Authorization".to_string(),
            auth_prefix: "Bearer".to_string(),
        }
    }
    
    /// Create config for Lambda Labs
    pub fn lambda(model: &str) -> Self {
        Self {
            provider: "lambda".to_string(),
            base_url: "https://api.lambdalabs.com/v1".to_string(),
            api_key_env: "LAMBDA_API_KEY".to_string(),
            model: model.to_string(),
            max_tokens: 4096,
            temperature: 0.2,
            timeout_secs: 60,
            auth_header: "Authorization".to_string(),
            auth_prefix: "Bearer".to_string(),
        }
    }
}
