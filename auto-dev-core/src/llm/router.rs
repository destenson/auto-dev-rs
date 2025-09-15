#![allow(unused)]
//! Intelligent LLM routing system inspired by OpenRouter
//!
//! Routes tasks to the most appropriate model based on:
//! - Task complexity
//! - Model availability
//! - Cost optimization
//! - Performance requirements

use super::{
    provider::{*, self},
    tiny::OllamaTinyModel,
    candle::SmartTinyModel,
    TinyModelConfig,
    ClassificationResult,
    QuestionType,
    TinyModel,
};
use anyhow::{Result, Context};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tracing::{info, warn, debug};

/// Intelligent router that selects the best model for each task
pub struct LLMRouter {
    providers: HashMap<String, Arc<dyn LLMProvider>>,
    tiers: HashMap<ModelTier, Vec<String>>,
    cache: Arc<RwLock<ResponseCache>>,
    config: RouterConfig,
    stats: Arc<RwLock<RouterStats>>,
}

impl LLMRouter {
    /// Create a new router with configuration
    pub fn new(config: RouterConfig) -> Self {
        Self {
            providers: HashMap::new(),
            tiers: HashMap::new(),
            cache: Arc::new(RwLock::new(ResponseCache::new(config.cache_size))),
            config,
            stats: Arc::new(RwLock::new(RouterStats::default())),
        }
    }
    
    /// Register a provider
    pub fn register_provider(&mut self, provider: Arc<dyn LLMProvider>) {
        let name = provider.name().to_string();
        let tier = provider.tier();
        
        self.providers.insert(name.clone(), provider);
        self.tiers.entry(tier).or_insert_with(Vec::new).push(name);
    }
    
    /// Setup default providers based on configuration
    pub async fn setup_default_providers(&mut self) -> Result<()> {
        // Always register heuristic provider (No LLM)
        self.register_provider(Arc::new(HeuristicProvider::new()));
        
        // Register Qwen tiny model if configured
        if let Some(qwen_config) = &self.config.qwen_config {
            if qwen_config.enabled {
                // Try Candle first for embedded GGUF
                if let Some(model_path) = &qwen_config.model_path {
                    let model = SmartTinyModel::new(Some(model_path.as_ref()));
                    self.register_provider(Arc::new(QwenProvider::new_candle(model)));
                } else if qwen_config.use_ollama {
                    // Fallback to Ollama
                    let config = TinyModelConfig {
                        model: qwen_config.model_name.clone(),
                        host: qwen_config.ollama_host.clone(),
                        max_tokens: 256,
                        temperature: 0.1,
                        timeout_secs: 10,
                    };
                    let model = OllamaTinyModel::new(config)?;
                    self.register_provider(Arc::new(QwenProvider::new_ollama(model)));
                }
            }
        }
        
        // Register other providers based on config
        // TODO: Add Claude, OpenAI, etc.
        
        info!("Registered {} providers across {} tiers", 
              self.providers.len(), self.tiers.len());
        
        Ok(())
    }
    
    /// Route a task to the appropriate model
    pub async fn route_task(&self, task: &Task) -> Result<TaskResult> {
        let start = Instant::now();
        
        // Check cache first
        if let Some(cached) = self.check_cache(&task).await {
            debug!("Cache hit for task: {}", task.description);
            self.update_stats(true, Duration::from_millis(1)).await;
            return Ok(cached);
        }
        
        // Assess complexity to determine tier
        let complexity = self.assess_task_complexity(&task).await?;
        info!("Task complexity: {:?}, using tier: {:?}", 
              complexity.reasoning, complexity.tier);
        
        // Try providers in order of tier
        let result = self.execute_with_fallback(task, complexity.tier).await?;
        
        // Cache the result
        self.cache_result(&task, &result).await;
        
        // Update stats
        self.update_stats(false, start.elapsed()).await;
        
        Ok(result)
    }
    
    /// Execute task with fallback to higher tiers if needed
    async fn execute_with_fallback(
        &self,
        task: &Task,
        starting_tier: ModelTier,
    ) -> Result<TaskResult> {
        let tiers = [
            ModelTier::NoLLM,
            ModelTier::Tiny,
            ModelTier::Small,
            ModelTier::Medium,
            ModelTier::Large,
        ];
        
        let start_index = tiers.iter().position(|&t| t == starting_tier).unwrap_or(0);
        
        for tier in &tiers[start_index..] {
            if let Some(provider_names) = self.tiers.get(tier) {
                for name in provider_names {
                    if let Some(provider) = self.providers.get(name) {
                        if provider.is_available().await {
                            match self.execute_task_with_provider(task, provider.as_ref()).await {
                                Ok(result) => {
                                    info!("Task completed by {} (tier: {:?})", name, tier);
                                    return Ok(result);
                                }
                                Err(e) => {
                                    warn!("Provider {} failed: {}", name, e);
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Err(anyhow::anyhow!("All providers failed for task: {}", task.description))
    }
    
    /// Execute a specific task with a specific provider
    async fn execute_task_with_provider(
        &self,
        task: &Task,
        provider: &dyn LLMProvider,
    ) -> Result<TaskResult> {
        match &task.task_type {
            TaskType::Classification(content) => {
                let result = provider.classify_content(content).await?;
                Ok(TaskResult::Classification(result))
            }
            TaskType::Question(question) => {
                let answer = provider.answer_question(question).await?
                    .ok_or_else(|| anyhow::anyhow!("No answer available"))?;
                Ok(TaskResult::Answer(answer))
            }
            TaskType::CodeGeneration { spec, context, options } => {
                let code = provider.generate_code(spec, context, options).await?;
                Ok(TaskResult::GeneratedCode(code))
            }
            TaskType::CodeReview { code, requirements } => {
                let review = provider.review_code(code, requirements).await?;
                Ok(TaskResult::Review(review))
            }
        }
    }
    
    /// Assess task complexity using heuristics or tiny model
    async fn assess_task_complexity(&self, task: &Task) -> Result<TaskComplexity> {
        // Quick heuristics first
        let tier = match &task.task_type {
            TaskType::Classification(_) => ModelTier::Tiny,
            TaskType::Question(q) if q.len() < 50 => ModelTier::Tiny,
            TaskType::Question(_) => ModelTier::Small,
            TaskType::CodeGeneration { spec, .. } => {
                if spec.content.len() < 500 {
                    ModelTier::Small
                } else if spec.content.len() < 2000 {
                    ModelTier::Medium
                } else {
                    ModelTier::Large
                }
            }
            TaskType::CodeReview { code, .. } => {
                if code.len() < 500 {
                    ModelTier::Tiny
                } else if code.len() < 2000 {
                    ModelTier::Small
                } else {
                    ModelTier::Medium
                }
            }
        };
        
        Ok(TaskComplexity {
            tier,
            reasoning: "Assessed based on task type and size".to_string(),
            estimated_tokens: task.estimate_tokens(),
            confidence: 0.8,
        })
    }
    
    /// Check cache for existing result
    async fn check_cache(&self, task: &Task) -> Option<TaskResult> {
        let cache = self.cache.read().await;
        cache.get(&task.cache_key())
    }
    
    /// Cache a result
    async fn cache_result(&self, task: &Task, result: &TaskResult) {
        let mut cache = self.cache.write().await;
        cache.insert(task.cache_key(), result.clone());
    }
    
    /// Update router statistics
    async fn update_stats(&self, cache_hit: bool, duration: Duration) {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        if cache_hit {
            stats.cache_hits += 1;
        }
        stats.total_duration += duration;
    }
    
    /// Get current statistics
    pub async fn get_stats(&self) -> RouterStats {
        self.stats.read().await.clone()
    }
}

/// Task to be routed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub description: String,
    pub task_type: TaskType,
    pub priority: Priority,
    pub max_latency_ms: Option<u64>,
}

impl Task {
    /// Generate a cache key for this task
    fn cache_key(&self) -> String {
        format!("{:?}", self.task_type).chars().take(100).collect()
    }
    
    /// Estimate tokens needed for this task
    fn estimate_tokens(&self) -> usize {
        match &self.task_type {
            TaskType::Classification(content) => content.len() / 4,
            TaskType::Question(q) => q.len() / 4 + 100,
            TaskType::CodeGeneration { spec, .. } => spec.content.len() / 4 + 500,
            TaskType::CodeReview { code, .. } => code.len() / 4 + 200,
        }
    }
}

/// Types of tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    Classification(String),
    Question(String),
    CodeGeneration {
        spec: Specification,
        context: ProjectContext,
        options: GenerationOptions,
    },
    CodeReview {
        code: String,
        requirements: Vec<Requirement>,
    },
}

/// Task result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskResult {
    Classification(ClassificationResult),
    Answer(String),
    GeneratedCode(GeneratedCode),
    Review(ReviewResult),
}

/// Router configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    pub cache_size: usize,
    pub prefer_local_models: bool,
    pub max_retries: u32,
    pub fallback_enabled: bool,
    pub cost_optimization: bool,
    pub qwen_config: Option<QwenConfig>,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            cache_size: 1000,
            prefer_local_models: true,
            max_retries: 3,
            fallback_enabled: true,
            cost_optimization: true,
            qwen_config: Some(QwenConfig::default()),
        }
    }
}

/// Qwen model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QwenConfig {
    pub enabled: bool,
    pub model_name: String,
    pub model_path: Option<std::path::PathBuf>,
    pub use_ollama: bool,
    pub ollama_host: String,
}

impl Default for QwenConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model_name: "qwen2.5-coder:0.5b".to_string(),
            model_path: None,
            use_ollama: true,
            ollama_host: "http://localhost:11434".to_string(),
        }
    }
}

/// Router statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RouterStats {
    pub total_requests: usize,
    pub cache_hits: usize,
    pub total_duration: Duration,
    pub provider_usage: HashMap<String, usize>,
    pub tier_usage: HashMap<ModelTier, usize>,
}

/// Simple response cache
struct ResponseCache {
    entries: HashMap<String, (TaskResult, Instant)>,
    max_size: usize,
    ttl: Duration,
}

impl ResponseCache {
    fn new(max_size: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_size,
            ttl: Duration::from_secs(300), // 5 minutes
        }
    }
    
    fn get(&self, key: &str) -> Option<TaskResult> {
        self.entries.get(key).and_then(|(result, timestamp)| {
            if timestamp.elapsed() < self.ttl {
                Some(result.clone())
            } else {
                None
            }
        })
    }
    
    fn insert(&mut self, key: String, result: TaskResult) {
        // Simple LRU: if at capacity, remove oldest
        if self.entries.len() >= self.max_size {
            if let Some(oldest_key) = self.entries
                .iter()
                .min_by_key(|(_, (_, ts))| *ts)
                .map(|(k, _)| k.clone())
            {
                self.entries.remove(&oldest_key);
            }
        }
        
        self.entries.insert(key, (result, Instant::now()));
    }
}

/// Heuristic provider for No-LLM tier
struct HeuristicProvider {
    classifier: crate::llm::classifier::HeuristicClassifier,
}

impl HeuristicProvider {
    fn new() -> Self {
        Self {
            classifier: crate::llm::classifier::HeuristicClassifier::new(),
        }
    }
}

#[async_trait]
impl LLMProvider for HeuristicProvider {
    fn name(&self) -> &str {
        "heuristic"
    }
    
    fn tier(&self) -> ModelTier {
        ModelTier::NoLLM
    }
    
    async fn is_available(&self) -> bool {
        true // Always available
    }
    
    fn cost_per_1k_tokens(&self) -> f32 {
        0.0 // Free!
    }
    
    async fn generate_code(
        &self,
        _spec: &Specification,
        _context: &ProjectContext,
        _options: &GenerationOptions,
    ) -> Result<GeneratedCode> {
        Err(anyhow::anyhow!("Heuristics cannot generate code"))
    }
    
    async fn explain_implementation(
        &self,
        _code: &str,
        _spec: &Specification,
    ) -> Result<Explanation> {
        Err(anyhow::anyhow!("Heuristics cannot explain code"))
    }
    
    async fn review_code(
        &self,
        code: &str,
        requirements: &[Requirement],
    ) -> Result<ReviewResult> {
        // Simple heuristic review
        let mut issues = Vec::new();
        
        if code.len() > 1000 {
            issues.push(Issue {
                severity: IssueSeverity::Warning,
                message: "Code is quite long, consider breaking into smaller functions".to_string(),
                line: None,
                suggestion: None,
            });
        }
        
        Ok(ReviewResult {
            issues,
            suggestions: vec![],
            meets_requirements: requirements.is_empty(),
            confidence: 0.3,
        })
    }
    
    async fn answer_question(&self, question: &str) -> Result<Option<String>> {
        // Only answer very simple questions
        if question.to_lowercase().contains("what is") {
            return Ok(Some("This requires a more capable model".to_string()));
        }
        Ok(None)
    }
    
    async fn classify_content(&self, content: &str) -> Result<ClassificationResult> {
        Ok(self.classifier.classify_content(content))
    }
    
    async fn assess_complexity(&self, task: &str) -> Result<TaskComplexity> {
        let tier = if task.len() < 100 {
            ModelTier::Tiny
        } else if task.len() < 500 {
            ModelTier::Small
        } else {
            ModelTier::Medium
        };
        
        Ok(TaskComplexity {
            tier,
            reasoning: "Based on task length".to_string(),
            estimated_tokens: task.len() / 4,
            confidence: 0.5,
        })
    }
}

/// Qwen provider wrapper
struct QwenProvider {
    model: QwenModelType,
}

enum QwenModelType {
    Ollama(OllamaTinyModel),
    Candle(SmartTinyModel),
}

impl QwenProvider {
    fn new_ollama(model: OllamaTinyModel) -> Self {
        Self {
            model: QwenModelType::Ollama(model),
        }
    }
    
    fn new_candle(model: SmartTinyModel) -> Self {
        Self {
            model: QwenModelType::Candle(model),
        }
    }
}

#[async_trait]
impl LLMProvider for QwenProvider {
    fn name(&self) -> &str {
        "qwen-0.5b"
    }
    
    fn tier(&self) -> ModelTier {
        ModelTier::Tiny
    }
    
    async fn is_available(&self) -> bool {
        // Check if model is loaded/reachable
        match &self.model {
            QwenModelType::Ollama(model) => {
                // Try a simple request
                model.is_code("test").await.is_ok()
            }
            QwenModelType::Candle(_) => true, // Always available if loaded
        }
    }
    
    fn cost_per_1k_tokens(&self) -> f32 {
        0.0 // Local model, no cost
    }
    
    async fn generate_code(
        &self,
        _spec: &Specification,
        _context: &ProjectContext,
        _options: &GenerationOptions,
    ) -> Result<GeneratedCode> {
        // Qwen 0.5B is not suitable for code generation
        Err(anyhow::anyhow!("Qwen 0.5B cannot reliably generate complex code"))
    }
    
    async fn explain_implementation(
        &self,
        _code: &str,
        _spec: &Specification,
    ) -> Result<Explanation> {
        Err(anyhow::anyhow!("Qwen 0.5B cannot provide detailed explanations"))
    }
    
    async fn review_code(
        &self,
        code: &str,
        requirements: &[Requirement],
    ) -> Result<ReviewResult> {
        // Qwen can do simple requirement checking
        let mut meets_requirements = true;
        let mut issues = Vec::new();
        
        for req in requirements {
            let satisfied = match &self.model {
                QwenModelType::Ollama(m) => {
                    m.check_requirement(&req.description, code).await?
                }
                QwenModelType::Candle(m) => {
                    m.check_requirement(&req.description, code).await?
                }
            };
            
            if !satisfied {
                meets_requirements = false;
                issues.push(Issue {
                    severity: IssueSeverity::Warning,
                    message: format!("Requirement not satisfied: {}", req.description),
                    line: None,
                    suggestion: None,
                });
            }
        }
        
        Ok(ReviewResult {
            issues,
            suggestions: vec![],
            meets_requirements,
            confidence: 0.7,
        })
    }
    
    async fn answer_question(&self, question: &str) -> Result<Option<String>> {
        match &self.model {
            QwenModelType::Ollama(m) => m.simple_answer(question).await,
            QwenModelType::Candle(m) => m.simple_answer(question).await,
        }
    }
    
    async fn classify_content(&self, content: &str) -> Result<ClassificationResult> {
        match &self.model {
            QwenModelType::Ollama(m) => m.classify_content(content).await,
            QwenModelType::Candle(m) => m.classify_content(content).await,
        }
    }
    
    async fn assess_complexity(&self, task: &str) -> Result<TaskComplexity> {
        match &self.model {
            QwenModelType::Ollama(m) => {
                let q_type = m.classify_question(task).await?;
                let tier = match q_type {
                    QuestionType::Complex => ModelTier::Large,
                    QuestionType::Simple | QuestionType::Definition => ModelTier::Tiny,
                    _ => ModelTier::Small,
                };
                
                Ok(TaskComplexity {
                    tier,
                    reasoning: format!("Question type: {:?}", q_type),
                    estimated_tokens: task.len() / 4,
                    confidence: 0.8,
                })
            }
            QwenModelType::Candle(m) => {
                let q_type = m.classify_question(task).await?;
                let tier = match q_type {
                    QuestionType::Complex => ModelTier::Large,
                    QuestionType::Simple | QuestionType::Definition => ModelTier::Tiny,
                    _ => ModelTier::Small,
                };
                
                Ok(TaskComplexity {
                    tier,
                    reasoning: format!("Question type: {:?}", q_type),
                    estimated_tokens: task.len() / 4,
                    confidence: 0.8,
                })
            }
        }
    }
}
