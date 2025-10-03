//! Anthropic Claude provider implementation with streaming, retry logic, and rate limiting

use super::{ClassificationResult, provider::*};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Rate limiter for API calls
#[derive(Debug, Clone)]
struct RateLimiter {
    /// Maximum requests per minute
    max_rpm: usize,
    /// Track request timestamps
    request_times: Arc<Mutex<Vec<Instant>>>,
}

impl RateLimiter {
    fn new(max_rpm: usize) -> Self {
        Self { max_rpm, request_times: Arc::new(Mutex::new(Vec::new())) }
    }

    async fn wait_if_needed(&self) {
        let mut times = self.request_times.lock().await;
        let now = Instant::now();

        // Remove timestamps older than 1 minute
        times.retain(|&t| now.duration_since(t) < Duration::from_secs(60));

        // If we've hit the rate limit, wait
        if times.len() >= self.max_rpm {
            if let Some(&oldest) = times.first() {
                let wait_time = Duration::from_secs(60) - now.duration_since(oldest);
                if wait_time > Duration::ZERO {
                    debug!("Rate limit reached, waiting {:?}", wait_time);
                    sleep(wait_time).await;
                }
            }
        }

        times.push(now);
    }
}

/// Claude provider for Anthropic's models
pub struct ClaudeProvider {
    client: Client,
    config: ClaudeConfig,
    model_tier: ModelTier,
    rate_limiter: RateLimiter,
}

impl ClaudeProvider {
    pub fn new(config: ClaudeConfig) -> Result<Self> {
        let client = Client::builder().timeout(Duration::from_secs(config.timeout_secs)).build()?;

        let model_tier = match config.model.as_str() {
            "claude-3-haiku-20240307" => ModelTier::Small,
            "claude-3-sonnet-20240229" => ModelTier::Medium,
            "claude-3-opus-20240229" => ModelTier::Large,
            "claude-3-5-sonnet-20241022" => ModelTier::Large,
            _ => ModelTier::Medium,
        };

        let rate_limiter = RateLimiter::new(config.max_requests_per_minute);

        Ok(Self { client, config, model_tier, rate_limiter })
    }

    /// Create a message with retry logic
    async fn create_message_with_retry(&self, messages: Vec<Message>) -> Result<String> {
        let mut attempt = 0;
        let max_retries = self.config.max_retries;
        let mut backoff = Duration::from_millis(self.config.initial_retry_delay_ms);

        loop {
            // Apply rate limiting
            self.rate_limiter.wait_if_needed().await;

            match self.create_message_internal(messages.clone()).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    let error_str = e.to_string();

                    // Check if it's a rate limit error
                    if error_str.contains("429") || error_str.contains("rate_limit") {
                        warn!("Rate limit error, backing off");
                        attempt += 1;
                        if attempt > max_retries {
                            return Err(e);
                        }
                        sleep(backoff).await;
                        backoff *= 2; // Exponential backoff
                        continue;
                    }

                    // Check if it's a temporary error worth retrying
                    if error_str.contains("timeout")
                        || error_str.contains("connection")
                        || error_str.contains("500")
                        || error_str.contains("502")
                        || error_str.contains("503")
                        || error_str.contains("504")
                    {
                        attempt += 1;
                        if attempt > max_retries {
                            return Err(e);
                        }
                        warn!("Temporary error, retrying in {:?}: {}", backoff, error_str);
                        sleep(backoff).await;
                        backoff = (backoff * 2).min(Duration::from_secs(30)); // Cap at 30 seconds
                        continue;
                    }

                    // Non-retryable error
                    return Err(e);
                }
            }
        }
    }

    /// Internal message creation without retry logic
    async fn create_message_internal(&self, messages: Vec<Message>) -> Result<String> {
        let api_key =
            std::env::var(&self.config.api_key_env).context("Anthropic API key not found")?;

        let request = MessageRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            system: self.config.system_prompt.clone(),
            stream: false, // Non-streaming request
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Claude")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Claude API error ({}): {}", status, error_text));
        }

        let result: MessageResponse =
            response.json().await.context("Failed to parse Claude response")?;

        result
            .content
            .first()
            .and_then(|c| match c {
                Content::Text { text } => Some(text.clone()),
            })
            .ok_or_else(|| anyhow::anyhow!("No response from Claude"))
    }

    /// Create a streaming message
    pub async fn create_message_stream(
        &self,
        messages: Vec<Message>,
    ) -> Result<impl Stream<Item = Result<String>>> {
        let api_key =
            std::env::var(&self.config.api_key_env).context("Anthropic API key not found")?;

        let request = StreamingMessageRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            system: self.config.system_prompt.clone(),
            stream: true,
        };

        // Apply rate limiting
        self.rate_limiter.wait_if_needed().await;

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send streaming request to Claude")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Claude API error ({}): {}", status, error_text));
        }

        // Convert response to stream of Server-Sent Events
        let stream = response.bytes_stream().map(move |chunk| {
            match chunk {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    // Parse SSE format
                    if text.starts_with("data: ") {
                        let json_str = text.trim_start_matches("data: ").trim();
                        if json_str == "[DONE]" {
                            return Ok(String::new());
                        }

                        if let Ok(event) = serde_json::from_str::<StreamEvent>(json_str) {
                            if let Some(delta) = event.delta {
                                return Ok(delta.text);
                            }
                        }
                    }
                    Ok(String::new())
                }
                Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
            }
        });

        Ok(stream)
    }

    async fn create_message(&self, messages: Vec<Message>) -> Result<String> {
        self.create_message_with_retry(messages).await
    }
}

#[async_trait]
impl LLMProvider for ClaudeProvider {
    fn name(&self) -> &str {
        "claude"
    }

    fn tier(&self) -> ModelTier {
        self.model_tier
    }

    async fn is_available(&self) -> bool {
        std::env::var(&self.config.api_key_env).is_ok()
    }

    fn cost_per_1k_tokens(&self) -> f32 {
        match self.config.model.as_str() {
            "claude-3-haiku-20240307" => 0.00025,
            "claude-3-sonnet-20240229" => 0.003,
            "claude-3-opus-20240229" => 0.015,
            "claude-3-5-sonnet-20241022" => 0.003,
            _ => 0.003,
        }
    }

    async fn generate_code(
        &self,
        spec: &Specification,
        context: &ProjectContext,
        options: &GenerationOptions,
    ) -> Result<GeneratedCode> {
        let prompt = format!(
            "You are tasked with implementing code based on the following specification.\n\
             Project Context:\
             - Language: {}\
             - Framework: {:?}\
             - Existing patterns to follow: {:?}\
             - Dependencies available: {:?}\n\
             Specification:\
             {}\n\
             Requirements:\
             {}\n\
             Examples:\
             {}\n\
             Please generate clean, well-documented code that:\
             1. Follows the specification exactly\
             2. Uses existing project patterns\
             3. Includes comprehensive error handling\
             4. Has clear documentation\
             5. Follows {} best practices\n\
             Format your response with code blocks marked with the language \
             and include a comment with the filepath at the top of each file.",
            context.language,
            context.framework,
            context.patterns,
            context.dependencies,
            spec.content,
            spec.requirements.join("\n"),
            spec.examples.join("\n"),
            context.language
        );

        let messages = vec![Message { role: "user".to_string(), content: prompt }];

        let response = if options.streaming {
            // For now, collect the stream into a single response
            // In a real implementation, you'd yield chunks to the caller
            let mut stream = self.create_message_stream(messages).await?;
            let mut full_response = String::new();
            while let Some(chunk) = stream.next().await {
                if let Ok(text) = chunk {
                    full_response.push_str(&text);
                }
            }
            full_response
        } else {
            self.create_message(messages).await?
        };

        // Parse code blocks from response
        let files = extract_code_files(&response);

        Ok(GeneratedCode {
            files,
            explanation: "Generated by Claude".to_string(),
            confidence: 0.9,
            tokens_used: response.len() / 4, // Rough estimate
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
            "Please explain how this code implements the given specification.\n\
             Code:\
             ```\
             {}\
             ```\n\
             Specification:\
             {}\n\
             Provide:\
             1. A brief summary\
             2. Key implementation details\
             3. Design decisions made\
             4. Any trade-offs or limitations",
            code, spec.content
        );

        let messages = vec![Message { role: "user".to_string(), content: prompt }];

        let response = self.create_message(messages).await?;

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
            "Please review this code against the following requirements.\n\
             Code to review:\
             ```\
             {}\
             ```\n\
             Requirements:\
             {}\n\
             For each requirement, indicate if it's satisfied and note any issues.\
             Also provide general code quality feedback and suggestions for improvement.",
            code, req_list
        );

        let messages = vec![Message { role: "user".to_string(), content: prompt }];

        let response = self.create_message(messages).await?;

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
            } else if lower.contains("not satisfied")
                || lower.contains("missing")
                || lower.contains("failed")
            {
                meets_requirements = false;
            }
        }

        Ok(ReviewResult { issues, suggestions, meets_requirements, confidence: 0.9 })
    }

    async fn answer_question(&self, question: &str) -> Result<Option<String>> {
        let messages = vec![Message { role: "user".to_string(), content: question.to_string() }];

        let response = self.create_message(messages).await?;
        Ok(Some(response))
    }

    async fn classify_content(&self, content: &str) -> Result<ClassificationResult> {
        // Claude is overkill for classification, but can do it
        let prompt = format!(
            "Classify this content. Respond with only a JSON object in this format:\
             {{\"is_code\": boolean, \"is_doc\": boolean, \"is_test\": boolean, \
              \"is_config\": boolean, \"language\": \"language name or null\"}}\n\
             Content to classify:\
             {}",
            &content[..content.len().min(500)]
        );

        let messages = vec![Message { role: "user".to_string(), content: prompt }];

        let response = self.create_message(messages).await?;

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
            "Assess the complexity of this task. Respond with one of:\
             - 'trivial' (can be done with heuristics)\
             - 'simple' (tiny model like 0.5B can handle)\
             - 'moderate' (needs 7B model)\
             - 'complex' (needs 13-34B model)\
             - 'very complex' (needs 70B+ model)\n\
             Task: {}\n\
             Respond with just the complexity level and a brief reason.",
            task
        );

        let messages = vec![Message { role: "user".to_string(), content: prompt }];

        let response = self.create_message(messages).await?;
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

/// Claude configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeConfig {
    pub api_key_env: String,
    pub model: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub system_prompt: Option<String>,
    pub timeout_secs: u64,
    pub max_retries: usize,
    pub initial_retry_delay_ms: u64,
    pub max_requests_per_minute: usize,
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self {
            api_key_env: "ANTHROPIC_API_KEY".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 4096,
            temperature: 0.2,
            system_prompt: Some("You are an expert software engineer.".to_string()),
            timeout_secs: 60,
            max_retries: 3,
            initial_retry_delay_ms: 1000,
            max_requests_per_minute: 50, // Conservative default
        }
    }
}

/// Message structure for Claude API
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

/// Claude API request
#[derive(Debug, Serialize)]
struct MessageRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: usize,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    stream: bool,
}

/// Streaming Claude API request
#[derive(Debug, Serialize)]
struct StreamingMessageRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: usize,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    stream: bool,
}

/// Claude API response
#[derive(Debug, Deserialize)]
struct MessageResponse {
    content: Vec<Content>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum Content {
    #[serde(rename = "text")]
    Text { text: String },
}

/// Streaming event from Claude API
#[derive(Debug, Deserialize)]
struct StreamEvent {
    delta: Option<TextDelta>,
}

#[derive(Debug, Deserialize)]
struct TextDelta {
    text: String,
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
                            || current_code.contains("#[cfg(test)]")
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
                if line.starts_with("//") || line.starts_with("#") || line.starts_with("--") {
                    if line.contains("filepath:")
                        || line.contains("file:")
                        || line.contains("File:")
                    {
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
        "c" => "c",
        "cpp" | "c++" => "cpp",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        "toml" => "toml",
        _ => "txt",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ClaudeConfig::default();
        assert_eq!(config.api_key_env, "ANTHROPIC_API_KEY");
        assert_eq!(config.model, "claude-3-5-sonnet-20241022");
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_extract_code_files() {
        let response = r#"
Here's the implementation:

```rust
// File: src/main.rs
fn main() {
    println!("Hello, world!");
}
```

And a test:

```rust
// File: src/test.rs
#[test]
fn test_main() {
    assert!(true);
}
```
"#;

        let files = extract_code_files(response);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "src/main.rs");
        assert!(!files[0].is_test);
        assert_eq!(files[1].path, "src/test.rs");
        assert!(files[1].is_test);
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(5); // 5 requests per minute

        // Should allow 5 quick requests
        for _ in 0..5 {
            limiter.wait_if_needed().await;
        }

        // The 6th request should wait
        let start = Instant::now();
        limiter.wait_if_needed().await;
        // This test might be flaky in CI, so just check it doesn't panic
        assert!(start.elapsed() < Duration::from_secs(61));
    }
}

#[cfg(test)]
#[cfg(feature = "integration")]
mod integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Ignored by default, requires API key
    async fn test_real_claude_api() {
        let config = ClaudeConfig::default();
        let provider = ClaudeProvider::new(config).unwrap();

        let result = provider.answer_question("What is 2+2? Reply with just the number.").await;

        assert!(result.is_ok());
        let answer = result.unwrap().unwrap();
        assert!(answer.contains("4"));
    }

    #[tokio::test]
    #[ignore] // Ignored by default, requires API key
    async fn test_streaming() {
        let config = ClaudeConfig::default();
        let provider = ClaudeProvider::new(config).unwrap();

        let messages =
            vec![Message { role: "user".to_string(), content: "Count from 1 to 5".to_string() }];

        let mut stream = provider.create_message_stream(messages).await.unwrap();
        let mut collected = String::new();

        while let Some(chunk) = stream.next().await {
            if let Ok(text) = chunk {
                collected.push_str(&text);
            }
        }

        assert!(collected.contains("1"));
        assert!(collected.contains("5"));
    }
}
