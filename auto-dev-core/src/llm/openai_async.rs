//! Enhanced OpenAI provider using async-openai with streaming support

use super::{
    ClassificationResult,
    provider::*,
    token_manager::{ConversationManager, Message as TokenMessage, TokenManager},
};
use anyhow::{Context, Result};
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
};
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tracing::{error, warn};

/// Enhanced OpenAI provider with streaming and token management
pub struct AsyncOpenAIProvider {
    client: Client<OpenAIConfig>,
    config: AsyncOpenAIConfig,
    model_tier: ModelTier,
    token_manager: Arc<Mutex<TokenManager>>,
    conversation: Arc<Mutex<ConversationManager>>,
}

impl AsyncOpenAIProvider {
    /// Create a new async OpenAI provider
    pub fn new(config: AsyncOpenAIConfig) -> Result<Self> {
        // Set up OpenAI client configuration
        let api_key = std::env::var(&config.api_key_env)
            .with_context(|| format!("OpenAI API key not found in {}", config.api_key_env))?;

        let openai_config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_org_id(config.organization_id.clone().unwrap_or_default());

        let client = Client::with_config(openai_config);

        let model_tier = match config.model.as_str() {
            "gpt-3.5-turbo" | "gpt-3.5-turbo-16k" => ModelTier::Small,
            "gpt-4" | "gpt-4-turbo-preview" => ModelTier::Medium,
            "gpt-4-32k" | "gpt-4-turbo" | "gpt-4o" => ModelTier::Large,
            "gpt-4o-mini" => ModelTier::Small,
            _ => ModelTier::Medium,
        };

        let conversation = ConversationManager::new(config.model.clone());

        Ok(Self {
            client,
            config,
            model_tier,
            token_manager: Arc::new(Mutex::new(TokenManager::new())),
            conversation: Arc::new(Mutex::new(conversation)),
        })
    }

    /// Convert internal messages to OpenAI format
    fn convert_messages(&self, messages: Vec<TokenMessage>) -> Vec<ChatCompletionRequestMessage> {
        messages
            .into_iter()
            .map(|msg| match msg.role.as_str() {
                "system" => ChatCompletionRequestSystemMessageArgs::default()
                    .content(msg.content)
                    .build()
                    .unwrap()
                    .into(),
                "user" => ChatCompletionRequestUserMessageArgs::default()
                    .content(msg.content)
                    .build()
                    .unwrap()
                    .into(),
                "assistant" => ChatCompletionRequestAssistantMessageArgs::default()
                    .content(msg.content)
                    .build()
                    .unwrap()
                    .into(),
                _ => ChatCompletionRequestUserMessageArgs::default()
                    .content(msg.content)
                    .build()
                    .unwrap()
                    .into(),
            })
            .collect()
    }

    /// Stream chat completion
    pub async fn stream_chat_completion(
        &self,
        messages: Vec<TokenMessage>,
    ) -> Result<mpsc::Receiver<String>> {
        // First trim messages if needed
        let messages_to_use = {
            let mut token_manager = self.token_manager.lock().await;
            let token_count = token_manager.count_message_tokens(&messages, &self.config.model)?;
            let limit = token_manager.get_model_limit(&self.config.model);

            if token_count + self.config.max_tokens > limit {
                warn!(
                    "Token count ({} + {}) exceeds limit ({}), trimming messages",
                    token_count, self.config.max_tokens, limit
                );
                // Trim messages if needed
                token_manager.trim_messages(
                    &messages,
                    &self.config.model,
                    self.config.max_tokens,
                    true,
                )?
            } else {
                messages
            }
        };

        let (tx, rx) = mpsc::channel(100);

        // Convert messages to OpenAI format
        let openai_messages = self.convert_messages(messages_to_use);

        // Create request
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.config.model)
            .messages(openai_messages)
            .temperature(self.config.temperature)
            .max_tokens(self.config.max_tokens as u32)
            .stream(true)
            .build()?;

        // Clone client for async task
        let client = self.client.clone();

        // Spawn task to handle streaming
        tokio::spawn(async move {
            match client.chat().create_stream(request).await {
                Ok(mut stream) => {
                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(response) => {
                                if let Some(choice) = response.choices.first() {
                                    if let Some(delta) = &choice.delta.content {
                                        if tx.send(delta.clone()).await.is_err() {
                                            break; // Receiver dropped
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Stream error: {}", e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to create stream: {}", e);
                    let _ = tx.send(format!("Error: {}", e)).await;
                }
            }
        });

        Ok(rx)
    }

    /// Regular (non-streaming) chat completion
    async fn chat_completion(&self, messages: Vec<TokenMessage>) -> Result<String> {
        // Check and trim messages if needed
        let mut token_manager = self.token_manager.lock().await;
        let messages = token_manager.trim_messages(
            &messages,
            &self.config.model,
            self.config.max_tokens,
            true,
        )?;
        drop(token_manager);

        let openai_messages = self.convert_messages(messages);

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.config.model)
            .messages(openai_messages)
            .temperature(self.config.temperature)
            .max_tokens(self.config.max_tokens as u32)
            .build()?;

        let response =
            self.client.chat().create(request).await.context("Failed to get OpenAI response")?;

        response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("No response from OpenAI"))
    }
}

#[async_trait]
impl LLMProvider for AsyncOpenAIProvider {
    fn name(&self) -> &str {
        "async-openai"
    }

    fn tier(&self) -> ModelTier {
        self.model_tier
    }

    async fn is_available(&self) -> bool {
        std::env::var(&self.config.api_key_env).is_ok()
    }

    fn cost_per_1k_tokens(&self) -> f32 {
        match self.config.model.as_str() {
            "gpt-3.5-turbo" | "gpt-3.5-turbo-16k" => 0.002,
            "gpt-4" => 0.03,
            "gpt-4-turbo-preview" | "gpt-4-turbo" => 0.01,
            "gpt-4o" => 0.005,
            "gpt-4o-mini" => 0.00015,
            _ => 0.01,
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
             Examples:\n{}\n\n\
             Generate implementation code with proper error handling and documentation.",
            context.language,
            context.framework,
            context.patterns,
            spec.content,
            spec.requirements.join("\n"),
            spec.examples.join("\n")
        );

        let messages = vec![
            TokenMessage { role: "system".to_string(), content: system_prompt, name: None },
            TokenMessage { role: "user".to_string(), content: user_prompt, name: None },
        ];

        // Use streaming if requested
        let response = if options.streaming {
            let mut rx = self.stream_chat_completion(messages).await?;
            let mut full_response = String::new();
            while let Some(chunk) = rx.recv().await {
                full_response.push_str(&chunk);
            }
            full_response
        } else {
            self.chat_completion(messages).await?
        };

        // Parse code blocks from response
        let files = extract_code_files(&response);

        // Count tokens used
        let mut token_manager = self.token_manager.lock().await;
        let tokens_used = token_manager.count_tokens(&response, &self.config.model)?;

        Ok(GeneratedCode {
            files,
            explanation: "Generated by OpenAI GPT".to_string(),
            confidence: 0.85,
            tokens_used,
            model_used: self.config.model.clone(),
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
             Code:\n```\n{}\n```\n\n\
             Specification:\n{}\n\n\
             Provide:\n\
             1. A brief summary\n\
             2. Key implementation details\n\
             3. Design decisions made\n\
             4. Any trade-offs or limitations",
            code, spec.content
        );

        let messages = vec![TokenMessage { role: "user".to_string(), content: prompt, name: None }];

        let response = self.chat_completion(messages).await?;

        // Parse the response into structured explanation
        let sections: Vec<&str> = response.split("\n\n").collect();

        Ok(Explanation {
            summary: sections.first().unwrap_or(&"").to_string(),
            details: sections.get(1).map(|s| vec![s.to_string()]).unwrap_or_default(),
            design_decisions: sections.get(2).map(|s| vec![s.to_string()]).unwrap_or_default(),
            trade_offs: sections.get(3).map(|s| vec![s.to_string()]).unwrap_or_default(),
        })
    }

    async fn review_code(&self, code: &str, requirements: &[Requirement]) -> Result<ReviewResult> {
        let req_list = requirements
            .iter()
            .map(|r| format!("{}: {} (Priority: {:?})", r.id, r.description, r.priority))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Review this code against the requirements:\n\n\
             Code:\n```\n{}\n```\n\n\
             Requirements:\n{}\n\n\
             For each requirement, indicate if it's satisfied and note any issues.\
             Also provide general code quality feedback.",
            code, req_list
        );

        let messages = vec![TokenMessage { role: "user".to_string(), content: prompt, name: None }];

        let response = self.chat_completion(messages).await?;

        // Parse response to extract issues and suggestions
        let mut issues = Vec::new();
        let mut suggestions = Vec::new();
        let mut meets_requirements = true;

        for line in response.lines() {
            let lower = line.to_lowercase();
            if lower.contains("issue:") || lower.contains("problem:") || lower.contains("error:") {
                let severity = if lower.contains("error") {
                    IssueSeverity::Error
                } else if lower.contains("warning") {
                    IssueSeverity::Warning
                } else {
                    IssueSeverity::Info
                };

                issues.push(Issue {
                    severity,
                    message: line.to_string(),
                    line: None,
                    suggestion: None,
                });

                if severity == IssueSeverity::Error {
                    meets_requirements = false;
                }
            } else if lower.contains("suggestion:") || lower.contains("recommend:") {
                suggestions.push(line.to_string());
            } else if lower.contains("not satisfied") || lower.contains("missing") {
                meets_requirements = false;
            }
        }

        Ok(ReviewResult { issues, suggestions, meets_requirements, confidence: 0.85 })
    }

    async fn answer_question(&self, question: &str) -> Result<Option<String>> {
        let messages = vec![TokenMessage {
            role: "user".to_string(),
            content: question.to_string(),
            name: None,
        }];

        let response = self.chat_completion(messages).await?;
        Ok(Some(response))
    }

    async fn classify_content(&self, content: &str) -> Result<ClassificationResult> {
        let prompt = format!(
            "Classify this content. Respond with only a JSON object:\n\
             {{\"is_code\": boolean, \"is_doc\": boolean, \"is_test\": boolean, \
              \"is_config\": boolean, \"language\": \"language name or null\"}}\n\n\
             Content:\n{}",
            &content[..content.len().min(500)]
        );

        let messages = vec![TokenMessage { role: "user".to_string(), content: prompt, name: None }];

        let response = self.chat_completion(messages).await?;

        // Extract JSON from response
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
        let json_str = &response[json_start..json_end];

        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
            return Ok(ClassificationResult {
                is_code: parsed["is_code"].as_bool().unwrap_or(false),
                is_documentation: parsed["is_doc"].as_bool().unwrap_or(false),
                is_test: parsed["is_test"].as_bool().unwrap_or(false),
                is_config: parsed["is_config"].as_bool().unwrap_or(false),
                language: parsed["language"].as_str().filter(|s| *s != "null").map(String::from),
                confidence: 0.95,
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
            "Assess the complexity of this task. Respond with one of:\n\
             - 'trivial' (can be done with heuristics)\n\
             - 'simple' (tiny model can handle)\n\
             - 'moderate' (needs small model)\n\
             - 'complex' (needs medium model)\n\
             - 'very complex' (needs large model)\n\n\
             Task: {}\n\n\
             Respond with just the complexity level and a brief reason.",
            task
        );

        let messages = vec![TokenMessage { role: "user".to_string(), content: prompt, name: None }];

        let response = self.chat_completion(messages).await?;
        let lower = response.to_lowercase();

        let tier = if lower.contains("trivial") {
            ModelTier::NoLLM
        } else if lower.contains("simple") {
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
            confidence: 0.9,
        })
    }
}

/// Configuration for async OpenAI provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncOpenAIConfig {
    pub api_key_env: String,
    pub model: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub organization_id: Option<String>,
    pub streaming: bool,
}

impl Default for AsyncOpenAIConfig {
    fn default() -> Self {
        Self {
            api_key_env: "OPENAI_API_KEY".to_string(),
            model: "gpt-4-turbo-preview".to_string(),
            max_tokens: 4096,
            temperature: 0.2,
            organization_id: None,
            streaming: true,
        }
    }
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
                let lang = line.trim_start_matches("```").trim();
                current_lang = lang.to_string();
            }
        } else if in_code_block {
            // Check for file path comment at the start
            if current_code.is_empty() {
                if line.starts_with("//") || line.starts_with("#") {
                    if line.contains("filepath:") || line.contains("file:") {
                        let path = line.split(':').nth(1).map(|s| s.trim().to_string());
                        current_path = path;
                        continue;
                    }
                }
            }
            current_code.push_str(line);
            current_code.push('\n');
        }
    }

    // Handle unclosed code block
    if in_code_block && !current_code.is_empty() {
        files.push(GeneratedFile {
            path: current_path
                .unwrap_or_else(|| format!("generated.{}", lang_to_extension(&current_lang))),
            content: current_code,
            language: current_lang,
            is_test: false,
        });
    }

    files
}

/// Convert language name to file extension
fn lang_to_extension(lang: &str) -> &str {
    match lang.to_lowercase().as_str() {
        "rust" | "rs" => "rs",
        "python" | "py" => "py",
        "javascript" | "js" => "js",
        "typescript" | "ts" => "ts",
        "go" | "golang" => "go",
        "java" => "java",
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

And a test:

```rust
// filepath: src/test.rs
#[test]
fn test_main() {
    assert_eq!(2 + 2, 4);
}
```
"#;

        let files = extract_code_files(response);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "src/main.rs");
        assert_eq!(files[1].path, "src/test.rs");
        assert!(!files[0].is_test);
        assert!(files[1].is_test);
    }
}
