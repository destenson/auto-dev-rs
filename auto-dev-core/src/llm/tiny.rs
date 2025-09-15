//! Tiny model implementation using Ollama or similar local servers

use super::{ClassificationResult, QuestionType, TinyModel, TinyModelConfig};
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Ollama-based tiny model implementation
pub struct OllamaTinyModel {
    config: TinyModelConfig,
    client: reqwest::Client,
}

impl OllamaTinyModel {
    /// Create a new Ollama tiny model instance
    pub fn new(config: TinyModelConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()?;
        
        Ok(Self { config, client })
    }

    /// Send a prompt to the model
    async fn prompt(&self, prompt: &str) -> Result<String> {
        let request = OllamaRequest {
            model: self.config.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
            options: OllamaOptions {
                temperature: self.config.temperature,
                num_predict: self.config.max_tokens as i32,
            },
        };

        let response = self.client
            .post(&format!("{}/api/generate", self.config.host))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Ollama")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Ollama request failed: {}",
                response.status()
            ));
        }

        let result: OllamaResponse = response.json().await
            .context("Failed to parse Ollama response")?;

        Ok(result.response)
    }
}

#[async_trait::async_trait]
impl TinyModel for OllamaTinyModel {
    async fn is_code(&self, content: &str) -> Result<bool> {
        // Take first 500 chars to keep prompt small
        let sample = &content[..content.len().min(500)];
        
        let prompt = format!(
            "Is this code? Answer only 'yes' or 'no'.\n\n{}",
            sample
        );

        let response = self.prompt(&prompt).await?;
        let answer = response.trim().to_lowercase();
        
        Ok(answer.contains("yes"))
    }

    async fn classify_content(&self, content: &str) -> Result<ClassificationResult> {
        let sample = &content[..content.len().min(500)];
        
        let prompt = format!(
            "Classify this content. Answer in JSON format:\n\
            {{\"is_code\": bool, \"is_doc\": bool, \"is_test\": bool, \"is_config\": bool, \"language\": \"name or null\"}}\n\n\
            Content:\n{}",
            sample
        );

        let response = self.prompt(&prompt).await?;
        
        // Try to parse JSON response
        if let Ok(parsed) = serde_json::from_str::<SimpleClassification>(&response) {
            return Ok(ClassificationResult {
                is_code: parsed.is_code,
                is_documentation: parsed.is_doc,
                is_test: parsed.is_test,
                is_config: parsed.is_config,
                language: parsed.language,
                confidence: 0.7, // Fixed confidence for tiny models
            });
        }

        // Fallback to simple heuristics
        Ok(ClassificationResult {
            is_code: response.contains("code"),
            is_documentation: response.contains("doc"),
            is_test: response.contains("test"),
            is_config: response.contains("config"),
            language: None,
            confidence: 0.3,
        })
    }

    async fn classify_question(&self, question: &str) -> Result<QuestionType> {
        let prompt = format!(
            "Classify this question type. Answer with one word: \
            'definition', 'yesno', 'classification', 'simple', or 'complex'.\n\n\
            Question: {}",
            question
        );

        let response = self.prompt(&prompt).await?;
        let answer = response.trim().to_lowercase();

        Ok(match answer.as_str() {
            s if s.contains("definition") => QuestionType::Definition,
            s if s.contains("yesno") || s.contains("yes") => QuestionType::YesNo,
            s if s.contains("classification") => QuestionType::Classification,
            s if s.contains("complex") => QuestionType::Complex,
            _ => QuestionType::Simple,
        })
    }

    async fn simple_answer(&self, question: &str) -> Result<Option<String>> {
        // Only answer if it's a simple question
        let q_type = self.classify_question(question).await?;
        
        if q_type == QuestionType::Complex {
            return Ok(None);
        }

        let prompt = format!(
            "Answer this simple question in one sentence:\n{}",
            question
        );

        let response = self.prompt(&prompt).await?;
        Ok(Some(response.trim().to_string()))
    }

    async fn check_requirement(&self, requirement: &str, code: &str) -> Result<bool> {
        let code_sample = &code[..code.len().min(500)];
        
        let prompt = format!(
            "Does this code satisfy the requirement? Answer 'yes' or 'no'.\n\n\
            Requirement: {}\n\n\
            Code:\n{}",
            requirement, code_sample
        );

        let response = self.prompt(&prompt).await?;
        Ok(response.to_lowercase().contains("yes"))
    }
}

/// Ollama API request structure
#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: i32,
}

/// Ollama API response structure
#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
}

/// Simple classification response structure
#[derive(Debug, Deserialize)]
struct SimpleClassification {
    is_code: bool,
    is_doc: bool,
    is_test: bool,
    is_config: bool,
    language: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = TinyModelConfig::default();
        assert_eq!(config.model, "qwen2.5-coder:0.5b");
        assert_eq!(config.temperature, 0.1);
    }
}