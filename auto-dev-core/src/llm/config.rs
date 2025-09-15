//! Configuration system for LLM integration and routing
//!
//! Provides comprehensive configuration for model selection, routing rules,
//! and task-specific optimizations with a focus on Qwen usage.

use super::provider::ModelTier;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Main configuration for the LLM system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    /// Routing configuration
    pub routing: RoutingConfig,
    
    /// Provider configurations
    pub providers: ProvidersConfig,
    
    /// Task-specific configurations
    pub tasks: TasksConfig,
    
    /// Qwen-specific optimizations
    pub qwen: QwenConfig,
    
    /// Cache configuration
    pub cache: CacheConfig,
    
    /// Monitoring and logging
    pub monitoring: MonitoringConfig,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            routing: RoutingConfig::default(),
            providers: ProvidersConfig::default(),
            tasks: TasksConfig::default(),
            qwen: QwenConfig::default(),
            cache: CacheConfig::default(),
            monitoring: MonitoringConfig::default(),
        }
    }
}

impl LLMConfig {
    /// Load configuration from file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context("Failed to read config file")?;
        
        let config: Self = toml::from_str(&content)
            .context("Failed to parse config file")?;
        
        Ok(config)
    }
    
    /// Save configuration to file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        std::fs::write(path, content)
            .context("Failed to write config file")?;
        
        Ok(())
    }
    
    /// Create an optimized config for Qwen-focused usage
    pub fn qwen_optimized() -> Self {
        let mut config = Self::default();
        
        // Optimize routing for Qwen
        config.routing.prefer_local = true;
        config.routing.task_routing.insert(
            "classification".to_string(),
            vec![ModelTier::Tiny, ModelTier::Small],
        );
        config.routing.task_routing.insert(
            "pattern_detection".to_string(),
            vec![ModelTier::Tiny, ModelTier::Small],
        );
        config.routing.task_routing.insert(
            "yes_no".to_string(),
            vec![ModelTier::Tiny],
        );
        config.routing.task_routing.insert(
            "requirement_check".to_string(),
            vec![ModelTier::Tiny, ModelTier::Small],
        );
        
        // Enable Qwen
        config.qwen.enabled = true;
        config.qwen.primary_tasks = vec![
            "classification".to_string(),
            "pattern_detection".to_string(),
            "yes_no".to_string(),
            "requirement_check".to_string(),
            "simple_completion".to_string(),
        ];
        
        config
    }
}

/// Routing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Prefer local models over cloud
    pub prefer_local: bool,
    
    /// Enable intelligent routing
    pub intelligent_routing: bool,
    
    /// Cost optimization level (0-10)
    pub cost_optimization_level: u8,
    
    /// Performance priority (0-10)
    pub performance_priority: u8,
    
    /// Task-specific routing rules
    pub task_routing: HashMap<String, Vec<ModelTier>>,
    
    /// Complexity thresholds
    pub complexity_thresholds: ComplexityThresholds,
    
    /// Fallback strategy
    pub fallback_strategy: FallbackStrategy,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        let mut task_routing = HashMap::new();
        
        // Default routing for common tasks
        task_routing.insert(
            "classification".to_string(),
            vec![ModelTier::Tiny, ModelTier::Small],
        );
        task_routing.insert(
            "code_generation".to_string(),
            vec![ModelTier::Medium, ModelTier::Large],
        );
        task_routing.insert(
            "complex_analysis".to_string(),
            vec![ModelTier::Large],
        );
        
        Self {
            prefer_local: true,
            intelligent_routing: true,
            cost_optimization_level: 7,
            performance_priority: 5,
            task_routing,
            complexity_thresholds: ComplexityThresholds::default(),
            fallback_strategy: FallbackStrategy::default(),
        }
    }
}

/// Complexity thresholds for routing decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityThresholds {
    /// Token count threshold for tiny models
    pub tiny_max_tokens: usize,
    
    /// Token count threshold for small models
    pub small_max_tokens: usize,
    
    /// Token count threshold for medium models
    pub medium_max_tokens: usize,
    
    /// Complexity score thresholds
    pub tiny_max_score: f32,
    pub small_max_score: f32,
    pub medium_max_score: f32,
}

impl Default for ComplexityThresholds {
    fn default() -> Self {
        Self {
            tiny_max_tokens: 500,
            small_max_tokens: 2000,
            medium_max_tokens: 8000,
            tiny_max_score: 0.3,
            small_max_score: 0.6,
            medium_max_score: 0.85,
        }
    }
}

/// Fallback strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackStrategy {
    /// Enable automatic fallback to higher tiers
    pub enabled: bool,
    
    /// Maximum tier to fallback to
    pub max_tier: ModelTier,
    
    /// Retry count before fallback
    pub retry_count: u32,
    
    /// Timeout before fallback (seconds)
    pub timeout_secs: u64,
}

impl Default for FallbackStrategy {
    fn default() -> Self {
        Self {
            enabled: true,
            max_tier: ModelTier::Large,
            retry_count: 2,
            timeout_secs: 30,
        }
    }
}

/// Provider configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    /// OpenAI configuration
    pub openai: Option<ProviderConfig>,
    
    /// Claude configuration
    pub claude: Option<ProviderConfig>,
    
    /// Groq configuration
    pub groq: Option<ProviderConfig>,
    
    /// Lambda Labs configuration
    pub lambda: Option<ProviderConfig>,
    
    /// Together AI configuration
    pub together: Option<ProviderConfig>,
    
    /// Ollama configuration
    pub ollama: Option<OllamaConfig>,
    
    /// CLI tools configuration
    pub cli_tools: CliToolsConfig,
    
    /// Custom providers
    pub custom: Vec<CustomProviderConfig>,
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self {
            openai: None,
            claude: None,
            groq: None,
            lambda: None,
            together: None,
            ollama: Some(OllamaConfig::default()),
            cli_tools: CliToolsConfig::default(),
            custom: Vec::new(),
        }
    }
}

/// Individual provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub enabled: bool,
    pub api_key_env: String,
    pub models: Vec<String>,
    pub default_model: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub timeout_secs: u64,
}

/// Ollama-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub enabled: bool,
    pub host: String,
    pub models: Vec<String>,
    pub default_model: String,
    pub pull_on_start: bool,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            host: "http://localhost:11434".to_string(),
            models: vec!["qwen2.5-coder:0.5b".to_string()],
            default_model: "qwen2.5-coder:0.5b".to_string(),
            pull_on_start: true,
        }
    }
}

/// CLI tools configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliToolsConfig {
    pub claude_cli: bool,
    pub fabric: bool,
    pub fabric_patterns: Vec<String>,
    pub custom_tools: Vec<CustomCliTool>,
}

impl Default for CliToolsConfig {
    fn default() -> Self {
        Self {
            claude_cli: true,
            fabric: true,
            fabric_patterns: vec![
                "write_code".to_string(),
                "review_code".to_string(),
                "explain_code".to_string(),
            ],
            custom_tools: Vec::new(),
        }
    }
}

/// Custom CLI tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCliTool {
    pub name: String,
    pub command: String,
    pub args_template: Vec<String>,
    pub tier: ModelTier,
}

/// Custom provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProviderConfig {
    pub name: String,
    pub base_url: String,
    pub api_key_env: String,
    pub auth_header: String,
    pub models: Vec<String>,
    pub tier_mapping: HashMap<String, ModelTier>,
}

/// Task-specific configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksConfig {
    /// Classification task config
    pub classification: TaskConfig,
    
    /// Pattern detection config
    pub pattern_detection: TaskConfig,
    
    /// Code generation config
    pub code_generation: TaskConfig,
    
    /// Code review config
    pub code_review: TaskConfig,
    
    /// Custom task configs
    pub custom: HashMap<String, TaskConfig>,
}

impl Default for TasksConfig {
    fn default() -> Self {
        Self {
            classification: TaskConfig {
                preferred_tier: ModelTier::Tiny,
                max_input_tokens: 500,
                timeout_secs: 5,
                cache_ttl_secs: 3600,
            },
            pattern_detection: TaskConfig {
                preferred_tier: ModelTier::Tiny,
                max_input_tokens: 500,
                timeout_secs: 5,
                cache_ttl_secs: 3600,
            },
            code_generation: TaskConfig {
                preferred_tier: ModelTier::Medium,
                max_input_tokens: 4000,
                timeout_secs: 60,
                cache_ttl_secs: 1800,
            },
            code_review: TaskConfig {
                preferred_tier: ModelTier::Small,
                max_input_tokens: 2000,
                timeout_secs: 30,
                cache_ttl_secs: 1800,
            },
            custom: HashMap::new(),
        }
    }
}

/// Individual task configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    pub preferred_tier: ModelTier,
    pub max_input_tokens: usize,
    pub timeout_secs: u64,
    pub cache_ttl_secs: u64,
}

/// Qwen-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QwenConfig {
    /// Enable Qwen models
    pub enabled: bool,
    
    /// Model file path (for GGUF)
    pub model_path: Option<PathBuf>,
    
    /// Use Ollama for Qwen
    pub use_ollama: bool,
    
    /// Ollama model name
    pub ollama_model: String,
    
    /// Primary tasks for Qwen
    pub primary_tasks: Vec<String>,
    
    /// Confidence threshold for Qwen responses
    pub confidence_threshold: f32,
    
    /// Maximum context size
    pub max_context_tokens: usize,
    
    /// Temperature for generation
    pub temperature: f32,
    
    /// Prompt optimization
    pub optimize_prompts: bool,
}

impl Default for QwenConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model_path: None,
            use_ollama: true,
            ollama_model: "qwen2.5-coder:0.5b".to_string(),
            primary_tasks: vec![
                "classification".to_string(),
                "pattern_detection".to_string(),
                "yes_no".to_string(),
                "requirement_check".to_string(),
            ],
            confidence_threshold: 0.7,
            max_context_tokens: 2048,
            temperature: 0.1,
            optimize_prompts: true,
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable response caching
    pub enabled: bool,
    
    /// Maximum cache size (entries)
    pub max_entries: usize,
    
    /// Default TTL (seconds)
    pub default_ttl_secs: u64,
    
    /// Cache directory
    pub cache_dir: Option<PathBuf>,
    
    /// Enable persistent cache
    pub persistent: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entries: 1000,
            default_ttl_secs: 3600,
            cache_dir: None,
            persistent: false,
        }
    }
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable metrics collection
    pub metrics_enabled: bool,
    
    /// Enable detailed logging
    pub detailed_logging: bool,
    
    /// Track token usage
    pub track_token_usage: bool,
    
    /// Track costs
    pub track_costs: bool,
    
    /// Metrics export path
    pub metrics_path: Option<PathBuf>,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            metrics_enabled: true,
            detailed_logging: false,
            track_token_usage: true,
            track_costs: true,
            metrics_path: None,
        }
    }
}

/// Example configuration TOML
pub const EXAMPLE_CONFIG: &str = r#"
# Auto-Dev LLM Configuration

[routing]
prefer_local = true
intelligent_routing = true
cost_optimization_level = 7
performance_priority = 5

[routing.complexity_thresholds]
tiny_max_tokens = 500
small_max_tokens = 2000
medium_max_tokens = 8000

[routing.fallback_strategy]
enabled = true
max_tier = "Large"
retry_count = 2
timeout_secs = 30

[qwen]
enabled = true
use_ollama = true
ollama_model = "qwen2.5-coder:0.5b"
primary_tasks = [
    "classification",
    "pattern_detection",
    "yes_no",
    "requirement_check"
]
confidence_threshold = 0.7
max_context_tokens = 2048
temperature = 0.1
optimize_prompts = true

[providers.ollama]
enabled = true
host = "http://localhost:11434"
models = ["qwen2.5-coder:0.5b", "codellama:7b"]
default_model = "qwen2.5-coder:0.5b"
pull_on_start = true

[cache]
enabled = true
max_entries = 1000
default_ttl_secs = 3600
persistent = false

[monitoring]
metrics_enabled = true
detailed_logging = false
track_token_usage = true
track_costs = true
"#;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = LLMConfig::default();
        assert!(config.routing.prefer_local);
        assert!(config.qwen.enabled);
    }
    
    #[test]
    fn test_qwen_optimized_config() {
        let config = LLMConfig::qwen_optimized();
        assert!(config.qwen.enabled);
        assert!(config.qwen.primary_tasks.contains(&"classification".to_string()));
        assert!(config.routing.task_routing.contains_key("classification"));
    }
    
    #[test]
    fn test_parse_example_config() {
        let config: Result<LLMConfig, _> = toml::from_str(EXAMPLE_CONFIG);
        assert!(config.is_ok());
        
        let config = config.unwrap();
        assert_eq!(config.qwen.ollama_model, "qwen2.5-coder:0.5b");
    }
}