#![allow(unused)]
//! Direct model loading using Candle (no external APIs needed)

use super::{ClassificationResult, QuestionType, TinyModel};
use anyhow::{Context, Result};
use candle_core::{DType, Device, Tensor};
use candle_transformers::models::quantized_llama::ModelWeights;
use std::path::Path;
use tokenizers::Tokenizer;

/// Candle-based tiny model that runs entirely in Rust
pub struct CandleTinyModel {
    model: ModelWeights,
    tokenizer: Tokenizer,
    device: Device,
    max_tokens: usize,
}

impl CandleTinyModel {
    /// Load a GGUF model from disk
    pub fn load_gguf(model_path: &Path, tokenizer_path: &Path) -> Result<Self> {
        // Use CPU device for maximum compatibility
        let device = Device::Cpu;

        // Load the quantized model
        let model = Self::load_model_weights(model_path, &device)?;

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;

        Ok(Self { model, tokenizer, device, max_tokens: 256 })
    }

    /// Load model weights from GGUF file
    fn load_model_weights(path: &Path, device: &Device) -> Result<ModelWeights> {
        // This is a simplified version - real implementation would need proper GGUF parsing
        // For now, we'll create a stub that demonstrates the interface
        Err(anyhow::anyhow!("GGUF loading not yet implemented - use fallback classifier"))
    }

    /// Generate text from a prompt
    pub fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        // Tokenize input
        let encoding = self
            .tokenizer
            .encode(prompt, false)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;

        let input_ids = encoding.get_ids();

        // Convert to tensor
        let input = Tensor::from_vec(input_ids.to_vec(), &[1, input_ids.len()], &self.device)?;

        // Run inference (simplified)
        // Real implementation would do proper autoregressive generation

        // For now, return a placeholder
        Ok("Model inference not yet implemented".to_string())
    }
}

/// Lightweight model selector that chooses between Candle and heuristics
pub struct SmartTinyModel {
    candle_model: Option<CandleTinyModel>,
    use_heuristics: bool,
}

impl SmartTinyModel {
    /// Create a new smart model that falls back to heuristics if no model is available
    pub fn new(model_path: Option<&Path>) -> Self {
        let candle_model = if let Some(path) = model_path {
            // Try to load model, but don't fail if it doesn't work
            let tokenizer_path = path.with_extension("json"); // Assume tokenizer is alongside
            CandleTinyModel::load_gguf(path, &tokenizer_path).ok()
        } else {
            None
        };

        Self { use_heuristics: candle_model.is_none(), candle_model }
    }

    /// Check if we're using heuristics or a real model
    pub fn is_using_heuristics(&self) -> bool {
        self.use_heuristics
    }
}

#[async_trait::async_trait]
impl TinyModel for SmartTinyModel {
    async fn is_code(&self, content: &str) -> Result<bool> {
        if self.use_heuristics {
            // Use the heuristic classifier
            let classifier = crate::llm::classifier::HeuristicClassifier::new();
            Ok(classifier.is_code(content))
        } else if let Some(model) = &self.candle_model {
            // Use the model
            let prompt =
                format!("Is this code? Answer yes or no:\n{}", &content[..content.len().min(200)]);
            let response = model.generate(&prompt, 10)?;
            Ok(response.to_lowercase().contains("yes"))
        } else {
            Err(anyhow::anyhow!("No model available"))
        }
    }

    async fn classify_content(&self, content: &str) -> Result<ClassificationResult> {
        if self.use_heuristics {
            let classifier = crate::llm::classifier::HeuristicClassifier::new();
            Ok(classifier.classify_content(content))
        } else {
            // Model-based classification would go here
            Err(anyhow::anyhow!("Model classification not implemented"))
        }
    }

    async fn classify_question(&self, question: &str) -> Result<QuestionType> {
        if self.use_heuristics {
            let classifier = crate::llm::classifier::HeuristicClassifier::new();
            Ok(classifier.classify_question(question))
        } else {
            // Model-based classification
            Err(anyhow::anyhow!("Model classification not implemented"))
        }
    }

    async fn simple_answer(&self, question: &str) -> Result<Option<String>> {
        // For now, only answer definition questions with heuristics
        let classifier = crate::llm::classifier::HeuristicClassifier::new();
        let q_type = classifier.classify_question(question);

        if q_type == QuestionType::Definition {
            // Try to extract the term and provide a simple answer
            if question.to_lowercase().starts_with("what is") {
                let term = question
                    .split_whitespace()
                    .skip(2)
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim_end_matches('?')
                    .to_string();

                // Simple definitions for common programming terms
                let answer = match term.to_lowercase().as_str() {
                    "a socket" | "socket" => 
                        Some("A socket is a network communication endpoint that allows programs to send and receive data.".to_string()),
                    "a function" | "function" => 
                        Some("A function is a reusable block of code that performs a specific task.".to_string()),
                    "a variable" | "variable" => 
                        Some("A variable is a named storage location that holds data.".to_string()),
                    "an api" | "api" => 
                        Some("An API is an interface that allows different software programs to communicate.".to_string()),
                    _ => None,
                };

                return Ok(answer);
            }
        }

        Ok(None)
    }

    async fn check_requirement(&self, requirement: &str, code: &str) -> Result<bool> {
        // Simple heuristic check
        let req_lower = requirement.to_lowercase();
        let code_lower = code.to_lowercase();

        // Look for key terms from requirement in code
        let key_terms: Vec<&str> = req_lower.split_whitespace().filter(|w| w.len() > 4).collect();

        let matches = key_terms.iter().filter(|term| code_lower.contains(*term)).count();

        // If more than 30% of key terms are found, consider it satisfied
        Ok(matches > 0 && (matches as f32 / key_terms.len().max(1) as f32) > 0.3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_smart_model_fallback() {
        // Create model without a path - should use heuristics
        let model = SmartTinyModel::new(None);
        assert!(model.is_using_heuristics());

        // Test basic operations
        let is_code = model.is_code("fn main() {}").await.unwrap();
        assert!(is_code);

        let is_code = model.is_code("This is documentation").await.unwrap();
        assert!(!is_code);
    }

    #[tokio::test]
    async fn test_simple_definitions() {
        let model = SmartTinyModel::new(None);

        let answer = model.simple_answer("What is a socket?").await.unwrap();
        assert!(answer.is_some());
        assert!(answer.unwrap().contains("network"));

        let answer = model.simple_answer("How do I implement this?").await.unwrap();
        assert!(answer.is_none()); // Complex question
    }

    #[tokio::test]
    async fn test_requirement_checking() {
        let model = SmartTinyModel::new(None);

        let satisfied = model
            .check_requirement(
                "Function must validate email addresses",
                "fn validate_email(email: &str) -> bool { email.contains('@') }",
            )
            .await
            .unwrap();

        assert!(satisfied); // Contains "validate" and "email"
    }
}
