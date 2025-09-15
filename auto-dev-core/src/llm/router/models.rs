//! Model-specific implementations and configurations

use crate::llm::provider::{ModelTier, LLMProvider};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Model capability definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub model_id: String,
    pub tier: ModelTier,
    pub strengths: Vec<TaskStrength>,
    pub weaknesses: Vec<TaskWeakness>,
    pub optimal_use_cases: Vec<String>,
    pub avoid_use_cases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStrength {
    FastInference,
    LowCost,
    LargeContext,
    CodeGeneration,
    NaturalLanguage,
    Reasoning,
    CreativeWriting,
    StructuredOutput,
    FunctionCalling,
    Multilingual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskWeakness {
    SlowInference,
    HighCost,
    SmallContext,
    PoorCodeQuality,
    LimitedReasoning,
    Hallucinations,
    InconsistentOutput,
}

/// Model profiles for common models
pub fn get_model_profiles() -> HashMap<String, ModelCapabilities> {
    let mut profiles = HashMap::new();
    
    // Qwen 0.5B profile
    profiles.insert("qwen-0.5b".to_string(), ModelCapabilities {
        model_id: "qwen-0.5b".to_string(),
        tier: ModelTier::Tiny,
        strengths: vec![
            TaskStrength::FastInference,
            TaskStrength::LowCost,
        ],
        weaknesses: vec![
            TaskWeakness::SmallContext,
            TaskWeakness::LimitedReasoning,
        ],
        optimal_use_cases: vec![
            "Simple classification".to_string(),
            "Pattern matching".to_string(),
            "Basic requirement checking".to_string(),
        ],
        avoid_use_cases: vec![
            "Complex code generation".to_string(),
            "Architectural design".to_string(),
        ],
    });
    
    // Code Llama 7B profile
    profiles.insert("codellama-7b".to_string(), ModelCapabilities {
        model_id: "codellama-7b".to_string(),
        tier: ModelTier::Small,
        strengths: vec![
            TaskStrength::CodeGeneration,
            TaskStrength::LowCost,
            TaskStrength::FastInference,
        ],
        weaknesses: vec![
            TaskWeakness::SmallContext,
        ],
        optimal_use_cases: vec![
            "Function implementation".to_string(),
            "Unit test generation".to_string(),
            "Code completion".to_string(),
        ],
        avoid_use_cases: vec![
            "Large system design".to_string(),
            "Complex documentation".to_string(),
        ],
    });
    
    // Mixtral 8x7B profile
    profiles.insert("mixtral-8x7b".to_string(), ModelCapabilities {
        model_id: "mixtral-8x7b".to_string(),
        tier: ModelTier::Medium,
        strengths: vec![
            TaskStrength::CodeGeneration,
            TaskStrength::LargeContext,
            TaskStrength::Reasoning,
        ],
        weaknesses: vec![
            TaskWeakness::SlowInference,
        ],
        optimal_use_cases: vec![
            "Module implementation".to_string(),
            "Integration code".to_string(),
            "API design".to_string(),
        ],
        avoid_use_cases: vec![
            "Simple tasks that don't need complexity".to_string(),
        ],
    });
    
    // Claude 3 Opus profile
    profiles.insert("claude-3-opus".to_string(), ModelCapabilities {
        model_id: "claude-3-opus".to_string(),
        tier: ModelTier::Large,
        strengths: vec![
            TaskStrength::LargeContext,
            TaskStrength::Reasoning,
            TaskStrength::CodeGeneration,
            TaskStrength::CreativeWriting,
        ],
        weaknesses: vec![
            TaskWeakness::HighCost,
            TaskWeakness::SlowInference,
        ],
        optimal_use_cases: vec![
            "System architecture".to_string(),
            "Complex refactoring".to_string(),
            "Documentation generation".to_string(),
            "Code review".to_string(),
        ],
        avoid_use_cases: vec![
            "Simple formatting".to_string(),
            "Basic classification".to_string(),
        ],
    });
    
    // GPT-4 Turbo profile
    profiles.insert("gpt-4-turbo".to_string(), ModelCapabilities {
        model_id: "gpt-4-turbo".to_string(),
        tier: ModelTier::Large,
        strengths: vec![
            TaskStrength::LargeContext,
            TaskStrength::Reasoning,
            TaskStrength::FunctionCalling,
            TaskStrength::StructuredOutput,
        ],
        weaknesses: vec![
            TaskWeakness::HighCost,
        ],
        optimal_use_cases: vec![
            "Complex problem solving".to_string(),
            "API integration".to_string(),
            "System design".to_string(),
        ],
        avoid_use_cases: vec![
            "High-volume simple tasks".to_string(),
        ],
    });
    
    profiles
}

/// Model-specific prompt templates
pub struct PromptTemplates {
    templates: HashMap<String, HashMap<String, String>>,
}

impl PromptTemplates {
    pub fn new() -> Self {
        let mut templates = HashMap::new();
        
        // Qwen templates
        let mut qwen_templates = HashMap::new();
        qwen_templates.insert("classification".to_string(), 
            "Classify the following text into one of these categories: {categories}\n\nText: {text}\n\nCategory:".to_string());
        qwen_templates.insert("simple_question".to_string(),
            "Answer briefly: {question}".to_string());
        templates.insert("qwen-0.5b".to_string(), qwen_templates);
        
        // Code Llama templates
        let mut codellama_templates = HashMap::new();
        codellama_templates.insert("code_generation".to_string(),
            "[INST] Write code for: {specification}\n\nLanguage: {language} [/INST]".to_string());
        codellama_templates.insert("test_generation".to_string(),
            "[INST] Write unit tests for:\n\n{code}\n\nUse {framework} framework [/INST]".to_string());
        templates.insert("codellama-7b".to_string(), codellama_templates);
        
        Self { templates }
    }
    
    pub fn get_template(&self, model_id: &str, task_type: &str) -> Option<String> {
        self.templates.get(model_id)
            .and_then(|model_templates| model_templates.get(task_type))
            .cloned()
    }
}

/// Model-specific parameter configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParameters {
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: Option<u32>,
    pub max_tokens: usize,
    pub stop_sequences: Vec<String>,
    pub frequency_penalty: f32,
    pub presence_penalty: f32,
}

impl ModelParameters {
    pub fn for_model(model_id: &str, task_type: &str) -> Self {
        match (model_id, task_type) {
            ("qwen-0.5b", "classification") => Self {
                temperature: 0.1,
                top_p: 0.9,
                top_k: Some(10),
                max_tokens: 50,
                stop_sequences: vec!["\n".to_string()],
                frequency_penalty: 0.0,
                presence_penalty: 0.0,
            },
            ("codellama-7b", "code_generation") => Self {
                temperature: 0.2,
                top_p: 0.95,
                top_k: None,
                max_tokens: 2048,
                stop_sequences: vec!["[/INST]".to_string()],
                frequency_penalty: 0.1,
                presence_penalty: 0.0,
            },
            (_, "creative") => Self {
                temperature: 0.8,
                top_p: 0.95,
                top_k: None,
                max_tokens: 4096,
                stop_sequences: vec![],
                frequency_penalty: 0.3,
                presence_penalty: 0.3,
            },
            _ => Self::default(),
        }
    }
}

impl Default for ModelParameters {
    fn default() -> Self {
        Self {
            temperature: 0.3,
            top_p: 0.95,
            top_k: None,
            max_tokens: 1024,
            stop_sequences: vec![],
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
        }
    }
}

/// Model health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHealth {
    pub model_id: String,
    pub status: HealthStatus,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub response_time_ms: Option<u64>,
    pub error_message: Option<String>,
    pub consecutive_failures: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Model pool for load balancing
pub struct ModelPool {
    models: Vec<String>,
    current_index: usize,
    health_statuses: HashMap<String, ModelHealth>,
}

impl ModelPool {
    pub fn new(models: Vec<String>) -> Self {
        let health_statuses = models.iter()
            .map(|id| (id.clone(), ModelHealth {
                model_id: id.clone(),
                status: HealthStatus::Unknown,
                last_check: chrono::Utc::now(),
                response_time_ms: None,
                error_message: None,
                consecutive_failures: 0,
            }))
            .collect();
        
        Self {
            models,
            current_index: 0,
            health_statuses,
        }
    }
    
    /// Get next healthy model using round-robin
    pub fn get_next_healthy(&mut self) -> Option<String> {
        let start_index = self.current_index;
        
        loop {
            let model_id = &self.models[self.current_index];
            self.current_index = (self.current_index + 1) % self.models.len();
            
            if let Some(health) = self.health_statuses.get(model_id) {
                if health.status == HealthStatus::Healthy || health.status == HealthStatus::Unknown {
                    return Some(model_id.clone());
                }
            }
            
            // We've checked all models
            if self.current_index == start_index {
                break;
            }
        }
        
        None
    }
    
    /// Update model health status
    pub fn update_health(&mut self, model_id: &str, success: bool, response_time_ms: Option<u64>) {
        if let Some(health) = self.health_statuses.get_mut(model_id) {
            health.last_check = chrono::Utc::now();
            health.response_time_ms = response_time_ms;
            
            if success {
                health.status = HealthStatus::Healthy;
                health.consecutive_failures = 0;
                health.error_message = None;
            } else {
                health.consecutive_failures += 1;
                if health.consecutive_failures >= 3 {
                    health.status = HealthStatus::Unhealthy;
                } else {
                    health.status = HealthStatus::Degraded;
                }
            }
        }
    }
}