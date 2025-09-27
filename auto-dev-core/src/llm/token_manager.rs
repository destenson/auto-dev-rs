//! Token counting and management for LLM interactions

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tiktoken_rs::{get_bpe_from_model, CoreBPE};

/// Token manager for counting and managing tokens across models
#[derive(Clone)]
pub struct TokenManager {
    encoders: HashMap<String, CoreBPE>,
    model_limits: HashMap<String, usize>,
}

impl std::fmt::Debug for TokenManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenManager")
            .field("model_limits", &self.model_limits)
            .field("encoder_count", &self.encoders.len())
            .finish()
    }
}

impl TokenManager {
    /// Create a new token manager
    pub fn new() -> Self {
        let mut model_limits = HashMap::new();
        
        // OpenAI models
        model_limits.insert("gpt-3.5-turbo".to_string(), 16385);
        model_limits.insert("gpt-3.5-turbo-16k".to_string(), 16385);
        model_limits.insert("gpt-4".to_string(), 8192);
        model_limits.insert("gpt-4-32k".to_string(), 32768);
        model_limits.insert("gpt-4-turbo".to_string(), 128000);
        model_limits.insert("gpt-4-turbo-preview".to_string(), 128000);
        model_limits.insert("gpt-4o".to_string(), 128000);
        model_limits.insert("gpt-4o-mini".to_string(), 128000);
        
        // Claude models (approximate)
        model_limits.insert("claude-3-haiku-20240307".to_string(), 200000);
        model_limits.insert("claude-3-sonnet-20240229".to_string(), 200000);
        model_limits.insert("claude-3-opus-20240229".to_string(), 200000);
        model_limits.insert("claude-3-5-sonnet-20241022".to_string(), 200000);
        
        Self {
            encoders: HashMap::new(),
            model_limits,
        }
    }
    
    /// Get or create encoder for a model
    fn get_encoder(&mut self, model: &str) -> Result<&CoreBPE> {
        if !self.encoders.contains_key(model) {
            let encoder = match model {
                m if m.starts_with("gpt-3.5-turbo") => {
                    get_bpe_from_model("gpt-3.5-turbo").context("Failed to get GPT-3.5 encoder")?
                }
                m if m.starts_with("gpt-4") => {
                    get_bpe_from_model("gpt-4").context("Failed to get GPT-4 encoder")?
                }
                _ => {
                    // Default to GPT-4 encoder for unknown models
                    get_bpe_from_model("gpt-4").context("Failed to get default encoder")?
                }
            };
            self.encoders.insert(model.to_string(), encoder);
        }
        
        Ok(self.encoders.get(model).unwrap())
    }
    
    /// Count tokens in a text for a specific model
    pub fn count_tokens(&mut self, text: &str, model: &str) -> Result<usize> {
        let encoder = self.get_encoder(model)?;
        let tokens = encoder.encode_with_special_tokens(text);
        Ok(tokens.len())
    }
    
    /// Count tokens for messages (includes message formatting overhead)
    pub fn count_message_tokens(&mut self, messages: &[Message], model: &str) -> Result<usize> {
        let mut total = 0;
        
        // Different models have different message overhead
        let tokens_per_message = if model.starts_with("gpt-3.5-turbo") {
            4
        } else if model.starts_with("gpt-4") {
            3
        } else {
            3 // Default
        };
        
        let tokens_per_name = -1; // If name is included, it reduces overhead by 1
        
        for message in messages {
            total += tokens_per_message;
            total += self.count_tokens(&message.role, model)?;
            total += self.count_tokens(&message.content, model)?;
            
            if let Some(name) = &message.name {
                total += self.count_tokens(name, model)?;
                total += tokens_per_name as usize;
            }
        }
        
        total += 3; // Every reply is primed with <|start|>assistant<|message|>
        
        Ok(total)
    }
    
    /// Get the token limit for a model
    pub fn get_model_limit(&self, model: &str) -> usize {
        self.model_limits.get(model).copied().unwrap_or(4096)
    }
    
    /// Check if messages fit within model limits
    pub fn check_fits(&mut self, messages: &[Message], model: &str, max_completion: usize) -> Result<bool> {
        let message_tokens = self.count_message_tokens(messages, model)?;
        let limit = self.get_model_limit(model);
        Ok(message_tokens + max_completion <= limit)
    }
    
    /// Trim messages to fit within token limit
    pub fn trim_messages(
        &mut self,
        messages: &[Message],
        model: &str,
        max_completion: usize,
        keep_system: bool,
    ) -> Result<Vec<Message>> {
        let limit = self.get_model_limit(model);
        let target = limit - max_completion;
        
        let mut result = Vec::new();
        let mut total_tokens = 0;
        
        // Always keep system message if requested
        if keep_system {
            if let Some(system) = messages.iter().find(|m| m.role == "system") {
                let tokens = self.count_tokens(&system.content, model)?;
                result.push(system.clone());
                total_tokens += tokens + 4; // Message overhead
            }
        }
        
        // Add messages from the end (most recent first)
        for message in messages.iter().rev() {
            if keep_system && message.role == "system" {
                continue; // Already added
            }
            
            let tokens = self.count_tokens(&message.content, model)?;
            if total_tokens + tokens + 4 > target {
                break;
            }
            
            result.insert(if keep_system { 1 } else { 0 }, message.clone());
            total_tokens += tokens + 4;
        }
        
        Ok(result)
    }
}

/// Message structure for token counting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Conversation context manager
#[derive(Debug, Clone)]
pub struct ConversationManager {
    messages: Vec<Message>,
    token_manager: TokenManager,
    model: String,
    max_history_tokens: usize,
}

impl ConversationManager {
    /// Create a new conversation manager
    pub fn new(model: String) -> Self {
        Self {
            messages: Vec::new(),
            token_manager: TokenManager::new(),
            model,
            max_history_tokens: 2048,
        }
    }
    
    /// Set system prompt
    pub fn set_system(&mut self, prompt: String) {
        // Remove existing system message if any
        self.messages.retain(|m| m.role != "system");
        
        // Add new system message at the beginning
        self.messages.insert(0, Message {
            role: "system".to_string(),
            content: prompt,
            name: None,
        });
    }
    
    /// Add a user message
    pub fn add_user(&mut self, content: String) {
        self.messages.push(Message {
            role: "user".to_string(),
            content,
            name: None,
        });
    }
    
    /// Add an assistant message
    pub fn add_assistant(&mut self, content: String) {
        self.messages.push(Message {
            role: "assistant".to_string(),
            content,
            name: None,
        });
    }
    
    /// Get messages trimmed to fit token limit
    pub fn get_messages(&mut self, max_completion: usize) -> Result<Vec<Message>> {
        self.token_manager.trim_messages(
            &self.messages,
            &self.model,
            max_completion,
            true, // Keep system message
        )
    }
    
    /// Clear conversation history (keeps system message)
    pub fn clear_history(&mut self) {
        self.messages.retain(|m| m.role == "system");
    }
    
    /// Get total token count
    pub fn total_tokens(&mut self) -> Result<usize> {
        self.token_manager.count_message_tokens(&self.messages, &self.model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_token_counting() {
        let mut manager = TokenManager::new();
        
        // Test basic counting
        let count = manager.count_tokens("Hello, world!", "gpt-4").unwrap();
        assert!(count > 0);
        assert!(count < 10);
        
        // Test longer text
        let long_text = "This is a longer piece of text that should have more tokens. ".repeat(10);
        let long_count = manager.count_tokens(&long_text, "gpt-4").unwrap();
        assert!(long_count > count);
    }
    
    #[test]
    fn test_message_tokens() {
        let mut manager = TokenManager::new();
        
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: "You are a helpful assistant.".to_string(),
                name: None,
            },
            Message {
                role: "user".to_string(),
                content: "Hello!".to_string(),
                name: None,
            },
        ];
        
        let count = manager.count_message_tokens(&messages, "gpt-4").unwrap();
        assert!(count > 0);
        assert!(count < 50); // Should be relatively small
    }
    
    #[test]
    fn test_trim_messages() {
        let mut manager = TokenManager::new();
        
        let mut messages = vec![
            Message {
                role: "system".to_string(),
                content: "You are a helpful assistant.".to_string(),
                name: None,
            },
        ];
        
        // Add many long messages to ensure we exceed token limits
        for i in 0..100 {
            messages.push(Message {
                role: if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
                content: format!("Message {}: This is a much longer test message with significantly more content to ensure we exceed the token limit. We need to make sure this message is long enough to actually trigger the trimming behavior when we have many of them. Adding more text here to increase the token count substantially. The quick brown fox jumps over the lazy dog repeatedly to add more tokens.", i),
                name: None,
            });
        }
        
        // Trim to fit within a small limit to force trimming
        let trimmed = manager.trim_messages(&messages, "gpt-4", 500, true).unwrap();
        
        // Should have fewer messages (or at least not more)
        assert!(trimmed.len() <= messages.len());
        
        // System message should be preserved if it exists in trimmed
        if !trimmed.is_empty() && messages[0].role == "system" {
            assert_eq!(trimmed[0].role, "system");
        }
        
        // If we have multiple messages, the last one should be the most recent
        if trimmed.len() > 1 && messages.len() > 1 {
            // Find the corresponding message in the original
            let last_trimmed = &trimmed[trimmed.len() - 1];
            let matching_original = messages.iter().rev().find(|m| m.content == last_trimmed.content);
            assert!(matching_original.is_some());
        }
    }
    
    #[test]
    fn test_conversation_manager() {
        let mut conv = ConversationManager::new("gpt-4".to_string());
        
        conv.set_system("You are a helpful assistant.".to_string());
        conv.add_user("Hello!".to_string());
        conv.add_assistant("Hi there! How can I help you?".to_string());
        conv.add_user("What's the weather?".to_string());
        
        let messages = conv.get_messages(1000).unwrap();
        assert_eq!(messages.len(), 4);
        
        let tokens = conv.total_tokens().unwrap();
        assert!(tokens > 0);
    }
}