//! OpenRouter gateway provider implementation
//!
//! This module provides access to 400+ LLM models from 60+ providers through
//! the OpenRouter unified API, with automatic failover and cost optimization.

use super::{
    provider::*,
    ClassificationResult,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use openrouter_rs::{
    OpenRouterClient,
    api::chat::{ChatCompletionRequest, Message as OpenRouterMessage},
    types::Role,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// OpenRouter provider for unified access to 400+ models
pub struct OpenRouterProvider {
    client: OpenRouterClient,
    config: OpenRouterConfig,
    model_tier: ModelTier,
    model_catalog: Arc<Mutex<ModelCatalog>>,
    usage_tracker: Arc<Mutex<UsageTracker>>,
}

impl OpenRouterProvider {
    /// Create a new OpenRouter provider
    pub async fn new(config: OpenRouterConfig) -> Result<Self> {
        // Get API key from environment
        let api_key = std::env::var(&config.api_key_env)
            .with_context(|| format!("OpenRouter API key not found in {}", config.api_key_env))?;

        // Build client with configuration
        let client = if let Some(base_url) = &config.base_url {
            OpenRouterClient::builder()
                .api_key(api_key)
                .base_url(base_url.clone())
                .build()
                .context("Failed to build OpenRouter client")?
        } else {
            OpenRouterClient::builder()
                .api_key(api_key)
                .build()
                .context("Failed to build OpenRouter client")?
        };

        // Determine model tier based on default model
        let model_tier = Self::determine_tier(&config.default_model);

        // Initialize model catalog and usage tracker
        let model_catalog = Arc::new(Mutex::new(ModelCatalog::new()));
        let usage_tracker = Arc::new(Mutex::new(UsageTracker::new()));

        // Load model catalog asynchronously
        let catalog_clone = model_catalog.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::load_model_catalog(catalog_clone).await {
                warn!("Failed to load model catalog: {}", e);
            }
        });

        Ok(Self {
            client,
            config,
            model_tier,
            model_catalog,
            usage_tracker,
        })
    }

    /// Determine model tier from model ID
    fn determine_tier(model_id: &str) -> ModelTier {
        // Check for opus first, as it's a more specific match
        match model_id {
            id if id.contains("opus") || id.contains("o1") => ModelTier::Large,
            id if id.contains("haiku") || id.contains("gpt-3.5") || id.contains("llama-3.2") => ModelTier::Small,
            id if id.contains("gpt-4") || id.contains("claude-3") || id.contains("sonnet") => ModelTier::Medium,
            _ => ModelTier::Medium,
        }
    }

    /// Load available models into catalog
    async fn load_model_catalog(catalog: Arc<Mutex<ModelCatalog>>) -> Result<()> {
        // In a real implementation, this would fetch from OpenRouter API
        // For now, we'll populate with known models
        let mut catalog = catalog.lock().await;
        
        // OpenAI models
        catalog.add_model("openai/gpt-4", 0.03, 0.06, ModelCategory::General);
        catalog.add_model("openai/gpt-3.5-turbo", 0.001, 0.002, ModelCategory::General);
        catalog.add_model("openai/o1-preview", 0.015, 0.06, ModelCategory::Reasoning);
        
        // Anthropic models
        catalog.add_model("anthropic/claude-3-opus", 0.015, 0.075, ModelCategory::General);
        catalog.add_model("anthropic/claude-3-sonnet", 0.003, 0.015, ModelCategory::General);
        catalog.add_model("anthropic/claude-3-haiku", 0.00025, 0.00125, ModelCategory::Fast);
        
        // Meta models
        catalog.add_model("meta-llama/llama-3.2-70b", 0.0, 0.0, ModelCategory::Free);
        catalog.add_model("meta-llama/codellama-70b", 0.0, 0.0, ModelCategory::Code);
        
        // Mistral models
        catalog.add_model("mistralai/mistral-7b", 0.0, 0.0, ModelCategory::Free);
        catalog.add_model("mistralai/mixtral-8x7b", 0.0003, 0.0006, ModelCategory::General);
        
        // DeepSeek models
        catalog.add_model("deepseek/deepseek-r1", 0.0, 0.0, ModelCategory::Reasoning);
        catalog.add_model("deepseek/deepseek-coder", 0.0, 0.0, ModelCategory::Code);
        
        info!("Loaded {} models into catalog", catalog.models.len());
        Ok(())
    }

    /// Select the best model based on task and optimization mode
    async fn select_model(&self, task_type: TaskType) -> Result<String> {
        let catalog = self.model_catalog.lock().await;
        
        match self.config.optimization_mode {
            OptimizationMode::Cheapest => {
                catalog.find_cheapest_model(task_type)
                    .ok_or_else(|| anyhow::anyhow!("No suitable model found"))
            }
            OptimizationMode::Balanced => {
                // Balance between cost and capability
                match task_type {
                    TaskType::Code => Ok(self.config.code_models.first()
                        .cloned()
                        .unwrap_or_else(|| "anthropic/claude-3-sonnet".to_string())),
                    TaskType::Reasoning => Ok(self.config.reasoning_models.first()
                        .cloned()
                        .unwrap_or_else(|| "openai/o1-preview".to_string())),
                    _ => Ok(self.config.default_model.clone()),
                }
            }
            OptimizationMode::Quality => {
                // Always use best models regardless of cost
                match task_type {
                    TaskType::Code => Ok("anthropic/claude-3-opus".to_string()),
                    TaskType::Reasoning => Ok("openai/o1-preview".to_string()),
                    _ => Ok("anthropic/claude-3-opus".to_string()),
                }
            }
        }
    }

    /// Convert messages to OpenRouter format
    fn convert_messages(&self, prompt: &str, system_prompt: Option<&str>) -> Vec<OpenRouterMessage> {
        let mut messages = Vec::new();
        
        if let Some(system) = system_prompt.or(self.config.system_prompt.as_deref()) {
            messages.push(OpenRouterMessage {
                role: Role::System,
                content: system.to_string(),
            });
        }
        
        messages.push(OpenRouterMessage {
            role: Role::User,
            content: prompt.to_string(),
        });
        
        messages
    }

    /// Track usage and costs
    async fn track_usage(&self, model: &str, tokens: usize, cost: f32) {
        let mut tracker = self.usage_tracker.lock().await;
        tracker.add_usage(model, tokens, cost);
        
        // Log if approaching cost limit
        if let Some(max_cost) = self.config.max_cost_per_request {
            if cost > max_cost * 0.8 {
                warn!("Request cost ${:.4} approaching limit ${:.2}", cost, max_cost);
            }
        }
    }
}

#[async_trait]
impl LLMProvider for OpenRouterProvider {
    fn name(&self) -> &str {
        "openrouter"
    }

    fn tier(&self) -> ModelTier {
        self.model_tier
    }

    async fn is_available(&self) -> bool {
        std::env::var(&self.config.api_key_env).is_ok()
    }

    fn cost_per_1k_tokens(&self) -> f32 {
        // Return average cost based on default model
        match self.config.default_model.as_str() {
            m if m.contains("gpt-3.5") => 0.0015,
            m if m.contains("claude-3-haiku") => 0.00075,
            m if m.contains("mistral-7b") => 0.0,
            m if m.contains("gpt-4") => 0.045,
            m if m.contains("claude-3-opus") => 0.045,
            _ => 0.01, // Default average
        }
    }

    async fn generate_code(
        &self,
        spec: &Specification,
        context: &ProjectContext,
        options: &GenerationOptions,
    ) -> Result<GeneratedCode> {
        // Select appropriate model for code generation
        let model = self.select_model(TaskType::Code).await?;
        debug!("Selected model {} for code generation", model);

        let prompt = format!(
            "Generate code based on the following specification:\n\n\
            Language: {}\n\
            Framework: {:?}\n\
            Dependencies: {:?}\n\n\
            Specification:\n{}\n\n\
            Requirements:\n{}\n\n\
            Examples:\n{}\n\n\
            Generate clean, well-documented code that follows {} best practices.\n\
            Format your response with code blocks marked with the language.",
            context.language,
            context.framework,
            context.dependencies,
            spec.content,
            spec.requirements.join("\n"),
            spec.examples.join("\n"),
            context.language
        );

        let messages = self.convert_messages(&prompt, None);
        
        let request = ChatCompletionRequest::builder()
            .model(model.clone())
            .messages(messages)
            .temperature(options.temperature as f64)
            .max_tokens(options.max_tokens as u32)
            .build()?;

        let response = self.client.send_chat_completion(&request).await
            .context("Failed to generate code via OpenRouter")?;

        // OpenRouter returns CompletionsResponse with content directly
        let content = response.choices.first()
            .and_then(|c| c.content())
            .ok_or_else(|| anyhow::anyhow!("No response from model"))?;

        // Extract code files from response
        let files = extract_code_files(&content);
        
        // Track usage
        let tokens_used = response.usage.as_ref()
            .map(|u| u.total_tokens as usize)
            .unwrap_or(content.len() / 4);
        
        let cost = self.calculate_cost(&model, tokens_used);
        self.track_usage(&model, tokens_used, cost).await;

        Ok(GeneratedCode {
            files,
            explanation: format!("Generated by {} via OpenRouter", model),
            confidence: 0.85,
            tokens_used,
            model_used: model,
            cached: false,
        })
    }

    async fn explain_implementation(
        &self,
        code: &str,
        spec: &Specification,
    ) -> Result<Explanation> {
        let model = self.select_model(TaskType::General).await?;
        
        let prompt = format!(
            "Explain how this code implements the given specification:\n\n\
            Code:\n```\n{}\n```\n\n\
            Specification:\n{}\n\n\
            Provide:\n\
            1. A brief summary\n\
            2. Key implementation details\n\
            3. Design decisions made\n\
            4. Any trade-offs or limitations",
            code, spec.content
        );

        let messages = self.convert_messages(&prompt, None);
        
        let request = ChatCompletionRequest::builder()
            .model(model.clone())
            .messages(messages)
            .temperature(0.3)
            .max_tokens(2048)
            .build()?;

        let response = self.client.send_chat_completion(&request).await?;
        let content = response.choices.first()
            .and_then(|c| c.content())
            .unwrap_or_default();

        // Parse explanation from response
        let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        
        Ok(Explanation {
            summary: lines.first().cloned().unwrap_or_default(),
            details: lines.iter().skip(1).take(3).cloned().collect(),
            design_decisions: vec![],
            trade_offs: vec![],
        })
    }

    async fn review_code(
        &self,
        code: &str,
        requirements: &[Requirement],
    ) -> Result<ReviewResult> {
        let model = self.select_model(TaskType::Code).await?;
        
        let req_list = requirements.iter()
            .map(|r| format!("- {}: {}", r.id, r.description))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Review this code against the requirements:\n\n\
            Code:\n```\n{}\n```\n\n\
            Requirements:\n{}\n\n\
            For each requirement, indicate if it's satisfied.\n\
            Also provide code quality feedback and suggestions.",
            code, req_list
        );

        let messages = self.convert_messages(&prompt, None);
        
        let request = ChatCompletionRequest::builder()
            .model(model)
            .messages(messages)
            .temperature(0.2)
            .max_tokens(2048)
            .build()?;

        let response = self.client.send_chat_completion(&request).await?;
        let content = response.choices.first()
            .and_then(|c| c.content())
            .unwrap_or_default();

        // Parse review results
        let mut issues = Vec::new();
        let mut suggestions = Vec::new();
        let mut meets_requirements = true;

        for line in content.lines() {
            let lower = line.to_lowercase();
            if lower.contains("issue:") || lower.contains("error:") {
                issues.push(Issue {
                    severity: IssueSeverity::Warning,
                    message: line.to_string(),
                    line: None,
                    suggestion: None,
                });
            } else if lower.contains("suggestion:") {
                suggestions.push(line.to_string());
            } else if lower.contains("not satisfied") {
                meets_requirements = false;
            }
        }

        Ok(ReviewResult {
            issues,
            suggestions,
            meets_requirements,
            confidence: 0.85,
        })
    }

    async fn answer_question(&self, question: &str) -> Result<Option<String>> {
        let model = self.config.chat_models.first()
            .cloned()
            .unwrap_or_else(|| self.config.default_model.clone());

        let messages = self.convert_messages(question, None);
        
        let request = ChatCompletionRequest::builder()
            .model(model)
            .messages(messages)
            .temperature(0.7)
            .max_tokens(1024)
            .build()?;

        let response = self.client.send_chat_completion(&request).await?;
        
        Ok(response.choices.first()
            .and_then(|c| c.content())
            .map(|s| s.to_string()))
    }

    async fn classify_content(&self, content: &str) -> Result<ClassificationResult> {
        // Use a cheap model for classification
        let model = self.config.chat_models.first()
            .cloned()
            .unwrap_or_else(|| "mistralai/mistral-7b".to_string());

        let prompt = format!(
            "Classify this content as JSON: {{\"is_code\": bool, \"is_doc\": bool, \
            \"is_test\": bool, \"is_config\": bool, \"language\": \"name or null\"}}\n\n\
            Content:\n{}",
            &content[..content.len().min(500)]
        );

        let messages = self.convert_messages(&prompt, None);
        
        let request = ChatCompletionRequest::builder()
            .model(model)
            .messages(messages)
            .temperature(0.1)
            .max_tokens(100)
            .build()?;

        let response = self.client.send_chat_completion(&request).await?;
        let content = response.choices.first()
            .and_then(|c| c.content())
            .unwrap_or_default();

        // Parse JSON response
        if let Some(json_start) = content.find('{') {
            if let Some(json_end) = content.rfind('}') {
                let json_str = &content[json_start..=json_end];
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                    return Ok(ClassificationResult {
                        is_code: parsed["is_code"].as_bool().unwrap_or(false),
                        is_documentation: parsed["is_doc"].as_bool().unwrap_or(false),
                        is_test: parsed["is_test"].as_bool().unwrap_or(false),
                        is_config: parsed["is_config"].as_bool().unwrap_or(false),
                        language: parsed["language"].as_str()
                            .filter(|s| *s != "null")
                            .map(String::from),
                        confidence: 0.8,
                    });
                }
            }
        }

        Ok(ClassificationResult {
            is_code: false,
            is_documentation: false,
            is_test: false,
            is_config: false,
            language: None,
            confidence: 0.5,
        })
    }

    async fn assess_complexity(&self, task: &str) -> Result<TaskComplexity> {
        let model = self.config.default_model.clone();
        
        let prompt = format!(
            "Assess the complexity of this task:\n{}\n\n\
            Respond with: trivial/simple/moderate/complex/very complex",
            task
        );

        let messages = self.convert_messages(&prompt, None);
        
        let request = ChatCompletionRequest::builder()
            .model(model)
            .messages(messages)
            .temperature(0.3)
            .max_tokens(100)
            .build()?;

        let response = self.client.send_chat_completion(&request).await?;
        let content = response.choices.first()
            .and_then(|c| c.content())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        let tier = if content.contains("trivial") {
            ModelTier::NoLLM
        } else if content.contains("simple") {
            ModelTier::Tiny
        } else if content.contains("very complex") {
            ModelTier::Large
        } else if content.contains("complex") {
            ModelTier::Medium
        } else {
            ModelTier::Small
        };

        Ok(TaskComplexity {
            tier,
            reasoning: content,
            estimated_tokens: task.len() / 4 * 2,
            confidence: 0.8,
        })
    }
}

impl OpenRouterProvider {
    /// Calculate cost for token usage
    fn calculate_cost(&self, model: &str, tokens: usize) -> f32 {
        let catalog = self.model_catalog.try_lock();
        if let Ok(catalog) = catalog {
            if let Some(info) = catalog.models.get(model) {
                return (info.cost_per_1k_prompt + info.cost_per_1k_completion) / 2.0 
                    * (tokens as f32 / 1000.0);
            }
        }
        
        // Fallback to estimated cost
        self.cost_per_1k_tokens() * (tokens as f32 / 1000.0)
    }
}

/// OpenRouter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterConfig {
    pub api_key_env: String,
    pub base_url: Option<String>,
    pub default_model: String,
    pub optimization_mode: OptimizationMode,
    pub enable_fallback: bool,
    pub track_usage: bool,
    pub max_cost_per_request: Option<f32>,
    pub system_prompt: Option<String>,
    pub code_models: Vec<String>,
    pub chat_models: Vec<String>,
    pub reasoning_models: Vec<String>,
    pub prefer_free_tier: bool,
}

impl Default for OpenRouterConfig {
    fn default() -> Self {
        Self {
            api_key_env: "OPENROUTER_API_KEY".to_string(),
            base_url: Some("https://openrouter.ai/api/v1".to_string()),
            default_model: "anthropic/claude-3-sonnet".to_string(),
            optimization_mode: OptimizationMode::Balanced,
            enable_fallback: true,
            track_usage: true,
            max_cost_per_request: Some(0.10),
            system_prompt: Some("You are an expert software engineer.".to_string()),
            code_models: vec![
                "anthropic/claude-3-opus".to_string(),
                "openai/gpt-4".to_string(),
                "deepseek/deepseek-coder".to_string(),
            ],
            chat_models: vec![
                "anthropic/claude-3-haiku".to_string(),
                "meta-llama/llama-3.2-70b".to_string(),
                "mistralai/mistral-7b".to_string(),
            ],
            reasoning_models: vec![
                "openai/o1-preview".to_string(),
                "deepseek/deepseek-r1".to_string(),
                "anthropic/claude-3-opus".to_string(),
            ],
            prefer_free_tier: false,
        }
    }
}

/// Optimization mode for model selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationMode {
    Cheapest,  // Always use cheapest suitable model
    Balanced,  // Balance cost and quality
    Quality,   // Always use best model regardless of cost
}

/// Task type for model selection
#[derive(Debug, Clone, Copy)]
pub enum TaskType {
    Code,
    Reasoning,
    Chat,
    General,
}

/// Model catalog for tracking available models
#[derive(Debug, Default)]
pub struct ModelCatalog {
    models: HashMap<String, ModelEntry>,
}

impl ModelCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_model(
        &mut self,
        id: &str,
        cost_prompt: f32,
        cost_completion: f32,
        category: ModelCategory,
    ) {
        self.models.insert(
            id.to_string(),
            ModelEntry {
                id: id.to_string(),
                cost_per_1k_prompt: cost_prompt,
                cost_per_1k_completion: cost_completion,
                category,
                available: true,
            },
        );
    }

    pub fn find_cheapest_model(&self, task_type: TaskType) -> Option<String> {
        let category_filter = match task_type {
            TaskType::Code => ModelCategory::Code,
            TaskType::Reasoning => ModelCategory::Reasoning,
            TaskType::Chat => ModelCategory::Fast,
            TaskType::General => ModelCategory::General,
        };

        self.models
            .values()
            .filter(|m| m.available && (m.category == category_filter || m.category == ModelCategory::General))
            .min_by(|a, b| {
                let a_cost = a.cost_per_1k_prompt + a.cost_per_1k_completion;
                let b_cost = b.cost_per_1k_prompt + b.cost_per_1k_completion;
                a_cost.partial_cmp(&b_cost).unwrap()
            })
            .map(|m| m.id.clone())
    }
}

/// Model entry in catalog
#[derive(Debug, Clone)]
struct ModelEntry {
    id: String,
    cost_per_1k_prompt: f32,
    cost_per_1k_completion: f32,
    category: ModelCategory,
    available: bool,
}

/// Model category for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModelCategory {
    General,
    Code,
    Reasoning,
    Fast,
    Free,
}

/// Usage tracking for cost monitoring
#[derive(Debug, Default)]
pub struct UsageTracker {
    total_tokens: usize,
    total_cost: f32,
    model_usage: HashMap<String, ModelUsage>,
}

impl UsageTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_usage(&mut self, model: &str, tokens: usize, cost: f32) {
        self.total_tokens += tokens;
        self.total_cost += cost;
        
        let entry = self.model_usage
            .entry(model.to_string())
            .or_insert_with(ModelUsage::default);
        
        entry.tokens += tokens;
        entry.cost += cost;
        entry.requests += 1;
    }

    pub fn get_summary(&self) -> String {
        format!(
            "Total usage: {} tokens, ${:.4} cost across {} models",
            self.total_tokens,
            self.total_cost,
            self.model_usage.len()
        )
    }
}

/// Per-model usage statistics
#[derive(Debug, Default)]
struct ModelUsage {
    tokens: usize,
    cost: f32,
    requests: usize,
}

/// Extract code files from response
fn extract_code_files(response: &str) -> Vec<GeneratedFile> {
    let mut files = Vec::new();
    let mut in_code_block = false;
    let mut current_code = String::new();
    let mut current_lang = String::new();
    let mut current_path = None;

    for line in response.lines() {
        if line.starts_with("```") {
            if in_code_block {
                // End of code block
                if !current_code.is_empty() {
                    files.push(GeneratedFile {
                        path: current_path.unwrap_or_else(|| {
                            format!("generated.{}", lang_to_extension(&current_lang))
                        }),
                        content: current_code.clone(),
                        language: current_lang.clone(),
                        is_test: current_code.contains("#[test]")
                            || current_code.contains("describe(")
                            || current_code.contains("test("),
                    });
                }
                current_code.clear();
                current_lang.clear();
                current_path = None;
                in_code_block = false;
            } else {
                // Start of code block
                in_code_block = true;
                current_lang = line.trim_start_matches("```").trim().to_string();
            }
        } else if in_code_block {
            // Check for file path comment
            if line.starts_with("//") || line.starts_with("#") {
                if line.contains("filepath:") || line.contains("file:") {
                    let path = line.split(':').nth(1).map(|s| s.trim().to_string());
                    current_path = path;
                    continue;
                }
            }
            current_code.push_str(line);
            current_code.push('\n');
        }
    }

    // Handle unclosed code block
    if in_code_block && !current_code.is_empty() {
        files.push(GeneratedFile {
            path: current_path.unwrap_or_else(|| "generated.txt".to_string()),
            content: current_code,
            language: current_lang,
            is_test: false,
        });
    }

    files
}

/// Convert language to file extension
fn lang_to_extension(lang: &str) -> &'static str {
    match lang.to_lowercase().as_str() {
        "rust" | "rs" => "rs",
        "python" | "py" => "py",
        "javascript" | "js" => "js",
        "typescript" | "ts" => "ts",
        "java" => "java",
        "c" => "c",
        "cpp" | "c++" => "cpp",
        "go" => "go",
        "ruby" | "rb" => "rb",
        "shell" | "bash" | "sh" => "sh",
        _ => "txt",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_code_files() {
        let response = r#"
Here's the implementation:

```rust
// filepath: src/main.rs
fn main() {
    println!("Hello, world!");
}
```

And here's a test:

```rust
#[test]
fn test_something() {
    assert_eq!(1 + 1, 2);
}
```
        "#;

        let files = extract_code_files(response);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "src/main.rs");
        assert!(files[0].content.contains("Hello, world!"));
        assert!(!files[0].is_test);
        assert!(files[1].is_test);
    }

    #[test]
    fn test_model_catalog() {
        let mut catalog = ModelCatalog::new();
        catalog.add_model("test/cheap", 0.001, 0.002, ModelCategory::General);
        catalog.add_model("test/expensive", 0.01, 0.02, ModelCategory::General);
        catalog.add_model("test/free", 0.0, 0.0, ModelCategory::Free);

        // Free tier model has category Free, not General, so when searching for General,
        // we should get the cheap General model
        let cheapest = catalog.find_cheapest_model(TaskType::General);
        assert_eq!(cheapest, Some("test/cheap".to_string()));
    }

    #[test]
    fn test_usage_tracker() {
        let mut tracker = UsageTracker::new();
        tracker.add_usage("model1", 1000, 0.01);
        tracker.add_usage("model1", 500, 0.005);
        tracker.add_usage("model2", 2000, 0.02);

        assert_eq!(tracker.total_tokens, 3500);
        assert_eq!(tracker.total_cost, 0.035);
        assert_eq!(tracker.model_usage.len(), 2);
    }

    #[test]
    fn test_determine_tier() {
        assert_eq!(OpenRouterProvider::determine_tier("gpt-3.5-turbo"), ModelTier::Small);
        // Now opus is checked first, so claude-3-opus returns Large
        assert_eq!(OpenRouterProvider::determine_tier("claude-3-opus"), ModelTier::Large);
        assert_eq!(OpenRouterProvider::determine_tier("gpt-4"), ModelTier::Medium);
        assert_eq!(OpenRouterProvider::determine_tier("unknown"), ModelTier::Medium);
    }
}