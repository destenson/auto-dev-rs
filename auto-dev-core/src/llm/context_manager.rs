#![allow(unused)]
//! Context window management for efficient token usage
//!
//! This module manages context windows across different models,
//! optimizing token usage and ensuring important context is preserved.

use anyhow::{Result, Context as AnyhowContext};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tiktoken_rs::{cl100k_base, CoreBPE};

/// Manages context for LLM interactions
pub struct ContextManager {
    max_tokens: usize,
    reserved_for_response: usize,
    tokenizer: CoreBPE,
    context_history: VecDeque<ContextEntry>,
    priority_context: Vec<String>,
}

impl ContextManager {
    /// Create a new context manager
    pub fn new(max_tokens: usize, reserved_for_response: usize) -> Result<Self> {
        let tokenizer = cl100k_base()
            .map_err(|e| anyhow::anyhow!("Failed to initialize tokenizer: {}", e))?;
        
        Ok(Self {
            max_tokens,
            reserved_for_response,
            tokenizer,
            context_history: VecDeque::with_capacity(10),
            priority_context: Vec::new(),
        })
    }
    
    /// Count tokens in a string
    pub fn count_tokens(&self, text: &str) -> usize {
        self.tokenizer.encode_with_special_tokens(text).len()
    }
    
    /// Add priority context that should always be included
    pub fn add_priority_context(&mut self, context: String) {
        self.priority_context.push(context);
    }
    
    /// Add a context entry to history
    pub fn add_to_history(&mut self, entry: ContextEntry) {
        self.context_history.push_back(entry);
        
        // Keep only last 10 entries
        while self.context_history.len() > 10 {
            self.context_history.pop_front();
        }
    }
    
    /// Build optimized context for a prompt
    pub fn build_context(
        &self,
        primary_content: &str,
        project_context: Option<&ProjectContext>,
    ) -> String {
        let available_tokens = self.max_tokens - self.reserved_for_response;
        let mut used_tokens = 0;
        let mut context_parts = Vec::new();
        
        // 1. Always include priority context first
        for priority in &self.priority_context {
            let tokens = self.count_tokens(priority);
            if used_tokens + tokens < available_tokens {
                context_parts.push(priority.clone());
                used_tokens += tokens;
            }
        }
        
        // 2. Add primary content (the main prompt/spec)
        let primary_tokens = self.count_tokens(primary_content);
        if used_tokens + primary_tokens < available_tokens {
            context_parts.push(primary_content.to_string());
            used_tokens += primary_tokens;
        } else {
            // Truncate primary content if needed
            let truncated = self.truncate_to_tokens(
                primary_content,
                available_tokens - used_tokens
            );
            context_parts.push(truncated);
            return context_parts.join("

---

");
        }
        
        // 3. Add project context if available
        if let Some(proj) = project_context {
            let proj_str = self.format_project_context(proj);
            let proj_tokens = self.count_tokens(&proj_str);
            
            if used_tokens + proj_tokens < available_tokens {
                context_parts.push(proj_str);
                used_tokens += proj_tokens;
            }
        }
        
        // 4. Add relevant history (most recent first)
        for entry in self.context_history.iter().rev() {
            if entry.relevance_score > 0.5 {
                let entry_str = format!("{}: {}", entry.role, entry.content);
                let entry_tokens = self.count_tokens(&entry_str);
                
                if used_tokens + entry_tokens < available_tokens {
                    context_parts.push(entry_str);
                    used_tokens += entry_tokens;
                } else {
                    break; // No more room
                }
            }
        }
        
        context_parts.join("

---

")
    }
    
    /// Truncate text to fit within token limit
    pub fn truncate_to_tokens(&self, text: &str, max_tokens: usize) -> String {
        let tokens = self.tokenizer.encode_with_special_tokens(text);
        
        if tokens.len() <= max_tokens {
            return text.to_string();
        }
        
        // Take first max_tokens tokens
        let truncated_tokens = &tokens[..max_tokens];
        
        // Decode back to string
        self.tokenizer.decode(truncated_tokens.to_vec())
            .unwrap_or_else(|_| {
                // Fallback to character-based truncation
                let chars_per_token = text.len() / tokens.len().max(1);
                let max_chars = max_tokens * chars_per_token;
                text.chars().take(max_chars).collect()
            })
    }
    
    /// Format project context into a string
    fn format_project_context(&self, context: &ProjectContext) -> String {
        let mut parts = vec![format!("Project Language: {}", context.language)];
        
        if let Some(framework) = &context.framework {
            parts.push(format!("Framework: {}", framework));
        }
        
        if !context.patterns.is_empty() {
            parts.push(format!("Patterns in use: {}", context.patterns.join(", ")));
        }
        
        if !context.dependencies.is_empty() {
            let deps = context.dependencies.iter()
                .take(10) // Limit to 10 most important
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!("Key dependencies: {}", deps));
        }
        
        if !context.existing_files.is_empty() {
            let files = context.existing_files.iter()
                .take(20) // Limit to 20 most relevant
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!("Related files: {}", files));
        }
        
        parts.join("\n")
    }
    
    /// Optimize context for a specific model tier
    pub fn optimize_for_tier(&self, content: &str, tier: ModelTier) -> String {
        let max_tokens = match tier {
            ModelTier::NoLLM => 0,
            ModelTier::Tiny => 512,    // Very limited context for tiny models
            ModelTier::Small => 4096,   // 4K context
            ModelTier::Medium => 16384, // 16K context
            ModelTier::Large => 32768,  // 32K+ context
        };
        
        if tier == ModelTier::NoLLM {
            return String::new();
        }
        
        self.truncate_to_tokens(content, max_tokens)
    }
    
    /// Extract key information for tiny model context
    pub fn extract_key_info(&self, content: &str) -> String {
        // For tiny models, extract only the most critical information
        let lines: Vec<&str> = content.lines().collect();
        let mut key_lines = Vec::new();
        
        for line in lines.iter().take(10) { // First 10 lines often most important
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("#") {
                key_lines.push(trimmed);
            }
        }
        
        // Look for key patterns
        for line in lines.iter() {
            let lower = line.to_lowercase();
            if lower.contains("todo") || 
               lower.contains("fixme") || 
               lower.contains("important") ||
               lower.contains("required") ||
               lower.contains("must") {
                key_lines.push(line.trim());
            }
        }
        
        // Limit to ~200 tokens for tiny models
        let result = key_lines.join("\n");
        self.truncate_to_tokens(&result, 200)
    }
}

/// An entry in the context history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub role: String,
    pub content: String,
    pub relevance_score: f32,
    pub timestamp: std::time::SystemTime,
}

/// Project context for code generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub language: String,
    pub framework: Option<String>,
    pub existing_files: Vec<String>,
    pub patterns: Vec<String>,
    pub dependencies: Vec<String>,
}

/// Model tier for context optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    NoLLM,
    Tiny,
    Small,
    Medium,
    Large,
}

/// Context window configuration for different models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub model_name: String,
    pub max_tokens: usize,
    pub reserved_for_response: usize,
    pub supports_system_prompt: bool,
}

impl ContextConfig {
    /// Get config for Qwen 0.5B
    pub fn qwen_tiny() -> Self {
        Self {
            model_name: "qwen2.5-coder:0.5b".to_string(),
            max_tokens: 2048,
            reserved_for_response: 256,
            supports_system_prompt: true,
        }
    }
    
    /// Get config for GPT-4
    pub fn gpt4() -> Self {
        Self {
            model_name: "gpt-4".to_string(),
            max_tokens: 8192,
            reserved_for_response: 2048,
            supports_system_prompt: true,
        }
    }
    
    /// Get config for Claude
    pub fn claude() -> Self {
        Self {
            model_name: "claude-3".to_string(),
            max_tokens: 100000,
            reserved_for_response: 4096,
            supports_system_prompt: true,
        }
    }
}

/// Sliding window for maintaining conversation context
pub struct SlidingWindow {
    messages: VecDeque<Message>,
    max_messages: usize,
    max_tokens: usize,
    tokenizer: CoreBPE,
}

impl SlidingWindow {
    pub fn new(max_messages: usize, max_tokens: usize) -> Result<Self> {
        let tokenizer = cl100k_base()
            .map_err(|e| anyhow::anyhow!("Failed to initialize tokenizer: {}", e))?;
        
        Ok(Self {
            messages: VecDeque::with_capacity(max_messages),
            max_messages,
            max_tokens,
            tokenizer,
        })
    }
    
    /// Add a message to the window
    pub fn add_message(&mut self, role: String, content: String) {
        self.messages.push_back(Message { role, content });
        
        // Remove old messages if exceeding max
        while self.messages.len() > self.max_messages {
            self.messages.pop_front();
        }
        
        // Remove messages if exceeding token limit
        while self.total_tokens() > self.max_tokens && !self.messages.is_empty() {
            self.messages.pop_front();
        }
    }
    
    /// Get total tokens in window
    fn total_tokens(&self) -> usize {
        self.messages.iter()
            .map(|m| self.tokenizer.encode_with_special_tokens(&m.content).len())
            .sum()
    }
    
    /// Get messages as a formatted string
    pub fn to_string(&self) -> String {
        self.messages.iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("

")
    }
    
    /// Get messages for API calls
    pub fn get_messages(&self) -> Vec<Message> {
        self.messages.iter().cloned().collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_context_manager_creation() {
        let manager = ContextManager::new(4096, 1024);
        assert!(manager.is_ok());
    }
    
    #[test]
    fn test_token_counting() {
        let manager = ContextManager::new(4096, 1024).unwrap();
        let text = "Hello, world!";
        let tokens = manager.count_tokens(text);
        assert!(tokens > 0);
        assert!(tokens < 10); // Should be around 3-4 tokens
    }
    
    #[test]
    fn test_truncation() {
        let manager = ContextManager::new(4096, 1024).unwrap();
        let long_text = "a".repeat(10000);
        let truncated = manager.truncate_to_tokens(&long_text, 100);
        let token_count = manager.count_tokens(&truncated);
        assert!(token_count <= 100);
    }
    
    #[test]
    fn test_sliding_window() {
        let mut window = SlidingWindow::new(5, 1000).unwrap();
        
        for i in 0..10 {
            window.add_message("user".to_string(), format!("Message {}", i));
        }
        
        // Should only keep last 5 messages
        assert!(window.messages.len() <= 5);
    }
}
