#![allow(unused)]
//! LLM usage optimization to minimize costs and improve performance

use super::*;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, trace};

/// Optimizes LLM usage through caching, templates, and patterns
pub struct LLMOptimizer {
    config: LLMOptimizationConfig,
    pattern_store: Arc<RwLock<PatternStore>>,
    template_store: Arc<RwLock<TemplateStore>>,
    cache: Arc<RwLock<ResponseCache>>,
    similarity_index: Arc<RwLock<SimilarityIndex>>,
    batch_queue: Arc<RwLock<BatchQueue>>,
}

impl LLMOptimizer {
    pub fn new(config: LLMOptimizationConfig) -> Self {
        Self {
            config,
            pattern_store: Arc::new(RwLock::new(PatternStore::new())),
            template_store: Arc::new(RwLock::new(TemplateStore::new())),
            cache: Arc::new(RwLock::new(ResponseCache::new())),
            similarity_index: Arc::new(RwLock::new(SimilarityIndex::new())),
            batch_queue: Arc::new(RwLock::new(BatchQueue::new())),
        }
    }

    /// Process a requirement and determine optimal approach
    pub async fn process_requirement(&self, request: LLMRequest) -> Result<Decision> {
        trace!("Processing LLM requirement");

        // Tier 1: Check for existing patterns
        if let Some(pattern) = self.find_pattern(&request).await? {
            info!("Using existing pattern, avoiding LLM call");
            return Ok(Decision::UsePattern(pattern));
        }

        // Tier 2: Check for templates
        if let Some(template) = self.find_template(&request).await? {
            info!("Using template, avoiding LLM call");
            return Ok(Decision::UseTemplate(template));
        }

        // Tier 3: Check cache
        if let Some(cached) = self.check_cache(&request).await? {
            info!("Using cached response, avoiding LLM call");
            return Ok(Decision::UseCached(cached));
        }

        // Tier 4: Find similar solutions
        if let Some(similar) = self.find_similar(&request).await? {
            info!(
                "Using similar solution ({}% match), avoiding LLM call",
                (similar.similarity_score * 100.0) as u32
            );
            return Ok(Decision::AdaptSimilar(similar));
        }

        // Tier 5: Batch if possible
        if self.can_batch(&request) {
            debug!("Adding request to batch queue");
            self.add_to_batch(request).await?;
            return Ok(Decision::Skip("Request batched for later processing".to_string()));
        }

        // Last resort: Use LLM
        info!("No optimization available, using LLM");
        Ok(Decision::RequiresLLM(self.optimize_context(request).await?))
    }

    /// Find matching pattern
    async fn find_pattern(&self, request: &LLMRequest) -> Result<Option<String>> {
        let patterns = self.pattern_store.read().await;

        for (id, pattern) in patterns.iter() {
            if pattern.matches(&request.context) {
                return Ok(Some(id.clone()));
            }
        }

        Ok(None)
    }

    /// Find matching template
    async fn find_template(&self, request: &LLMRequest) -> Result<Option<String>> {
        let templates = self.template_store.read().await;

        for (id, template) in templates.iter() {
            if template.can_handle(&request.prompt) {
                return Ok(Some(id.clone()));
            }
        }

        Ok(None)
    }

    /// Check response cache
    async fn check_cache(&self, request: &LLMRequest) -> Result<Option<CachedResponse>> {
        let cache = self.cache.read().await;
        cache.get(request)
    }

    /// Find similar past solutions
    async fn find_similar(&self, request: &LLMRequest) -> Result<Option<SimilarSolution>> {
        let index = self.similarity_index.read().await;

        let similar =
            index.find_similar(&request.context, self.config.similarity_threshold).await?;

        if let Some((id, score, solution)) = similar {
            Ok(Some(SimilarSolution {
                solution_id: id,
                similarity_score: score,
                solution,
                adaptations_needed: vec![], // Would be computed based on differences
            }))
        } else {
            Ok(None)
        }
    }

    /// Check if request can be batched
    fn can_batch(&self, request: &LLMRequest) -> bool {
        request.model_tier != ModelTier::Tier5LLM
            || request.prompt.contains("generate")
            || request.prompt.contains("implement")
    }

    /// Add request to batch queue
    async fn add_to_batch(&self, request: LLMRequest) -> Result<()> {
        let mut queue = self.batch_queue.write().await;
        queue.add(request).await
    }

    /// Optimize context to reduce tokens
    async fn optimize_context(&self, mut request: LLMRequest) -> Result<LLMRequest> {
        // Remove redundant whitespace
        request.context = request
            .context
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        // Truncate if too long
        if request.context.len() > self.config.max_context_tokens * 4 {
            request.context =
                request.context.chars().take(self.config.max_context_tokens * 4).collect();
        }

        Ok(request)
    }

    /// Process batched requests
    pub async fn process_batch(&self) -> Result<Vec<Decision>> {
        let mut queue = self.batch_queue.write().await;
        let batch = queue.take_batch(self.config.batch_size).await?;

        if batch.is_empty() {
            return Ok(vec![]);
        }

        info!("Processing batch of {} requests", batch.len());

        // Combine contexts and prompts
        let combined = self.combine_batch(batch).await?;

        // Single LLM call for entire batch
        // This would call the actual LLM service

        Ok(vec![])
    }

    /// Combine batch into single request
    async fn combine_batch(&self, batch: Vec<LLMRequest>) -> Result<LLMRequest> {
        let mut combined_context = String::new();
        let mut combined_prompt = String::new();

        for (i, request) in batch.iter().enumerate() {
            combined_context.push_str(&format!("\n=== Request {} ===\n", i + 1));
            combined_context.push_str(&request.context);

            combined_prompt.push_str(&format!("\n{}. {}", i + 1, request.prompt));
        }

        Ok(LLMRequest {
            context: combined_context,
            prompt: combined_prompt,
            model_tier: ModelTier::Tier5LLM,
            max_tokens: None,
        })
    }

    /// Learn from successful LLM response
    pub async fn learn_from_response(&self, request: &LLMRequest, response: &str) -> Result<()> {
        // Cache the response
        let mut cache = self.cache.write().await;
        cache.store(request.clone(), response.to_string()).await;

        // Update similarity index
        let mut index = self.similarity_index.write().await;
        index.add_solution(&request.context, response).await?;

        // Extract patterns if possible
        if let Some(pattern) = self.extract_pattern(&request.context, response).await? {
            let mut patterns = self.pattern_store.write().await;
            patterns.add(pattern).await;
        }

        Ok(())
    }

    /// Extract pattern from successful solution
    async fn extract_pattern(&self, context: &str, solution: &str) -> Result<Option<Pattern>> {
        // Simple pattern extraction - would be more sophisticated in practice
        if solution.contains("fn ") && solution.contains("{") {
            Ok(Some(Pattern {
                id: uuid::Uuid::new_v4().to_string(),
                context_pattern: context.lines().take(3).collect::<Vec<_>>().join("\n"),
                solution_template: solution.to_string(),
                usage_count: 0,
            }))
        } else {
            Ok(None)
        }
    }
}

/// Store for patterns
struct PatternStore {
    patterns: HashMap<String, Pattern>,
}

impl PatternStore {
    fn new() -> Self {
        Self { patterns: HashMap::new() }
    }

    fn iter(&self) -> impl Iterator<Item = (&String, &Pattern)> {
        self.patterns.iter()
    }

    async fn add(&mut self, pattern: Pattern) {
        self.patterns.insert(pattern.id.clone(), pattern);
    }
}

/// Pattern definition
struct Pattern {
    id: String,
    context_pattern: String,
    solution_template: String,
    usage_count: usize,
}

impl Pattern {
    fn matches(&self, context: &str) -> bool {
        context.contains(&self.context_pattern)
    }
}

/// Store for templates
struct TemplateStore {
    templates: HashMap<String, Template>,
}

impl TemplateStore {
    fn new() -> Self {
        Self { templates: HashMap::new() }
    }

    fn iter(&self) -> impl Iterator<Item = (&String, &Template)> {
        self.templates.iter()
    }
}

/// Template definition
struct Template {
    id: String,
    prompt_pattern: String,
    template: String,
    placeholders: Vec<String>,
}

impl Template {
    fn can_handle(&self, prompt: &str) -> bool {
        prompt.contains(&self.prompt_pattern)
    }
}

/// Response cache
struct ResponseCache {
    cache: HashMap<String, CachedResponse>,
    ttl: Duration,
}

impl ResponseCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            ttl: Duration::from_secs(3600 * 24), // 24 hours
        }
    }

    fn get(&self, request: &LLMRequest) -> Result<Option<CachedResponse>> {
        let key = self.make_key(request);

        if let Some(cached) = self.cache.get(&key) {
            let age = Utc::now() - cached.timestamp;
            if age.to_std().unwrap() < self.ttl {
                return Ok(Some(cached.clone()));
            }
        }

        Ok(None)
    }

    async fn store(&mut self, request: LLMRequest, response: String) {
        let key = self.make_key(&request);

        self.cache.insert(
            key.clone(),
            CachedResponse { request_hash: key, response, timestamp: Utc::now(), usage_count: 0 },
        );
    }

    fn make_key(&self, request: &LLMRequest) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        request.context.hash(&mut hasher);
        request.prompt.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// Similarity index for finding similar solutions
struct SimilarityIndex {
    solutions: Vec<(String, String, String)>, // (id, context, solution)
}

impl SimilarityIndex {
    fn new() -> Self {
        Self { solutions: Vec::new() }
    }

    async fn find_similar(
        &self,
        context: &str,
        threshold: f32,
    ) -> Result<Option<(String, f32, String)>> {
        let mut best_match = None;
        let mut best_score = 0.0;

        for (id, stored_context, solution) in &self.solutions {
            let score = self.calculate_similarity(context, stored_context);

            if score > best_score && score >= threshold {
                best_score = score;
                best_match = Some((id.clone(), score, solution.clone()));
            }
        }

        Ok(best_match)
    }

    async fn add_solution(&mut self, context: &str, solution: &str) -> Result<()> {
        let id = uuid::Uuid::new_v4().to_string();
        self.solutions.push((id, context.to_string(), solution.to_string()));
        Ok(())
    }

    fn calculate_similarity(&self, text1: &str, text2: &str) -> f32 {
        // Simple Jaccard similarity - would use embeddings in practice
        let words1: std::collections::HashSet<_> = text1.split_whitespace().collect();
        let words2: std::collections::HashSet<_> = text2.split_whitespace().collect();

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        if union == 0 { 0.0 } else { intersection as f32 / union as f32 }
    }
}

/// Batch queue for combining requests
struct BatchQueue {
    queue: Vec<LLMRequest>,
}

impl BatchQueue {
    fn new() -> Self {
        Self { queue: Vec::new() }
    }

    async fn add(&mut self, request: LLMRequest) -> Result<()> {
        self.queue.push(request);
        Ok(())
    }

    async fn take_batch(&mut self, size: usize) -> Result<Vec<LLMRequest>> {
        let batch: Vec<_> = self.queue.drain(..size.min(self.queue.len())).collect();
        Ok(batch)
    }
}

use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_llm_optimizer() {
        let config = LLMOptimizationConfig {
            cache_ttl_hours: 24,
            similarity_threshold: 0.85,
            batch_size: 5,
            max_context_tokens: 2000,
        };

        let optimizer = LLMOptimizer::new(config);

        let request = LLMRequest {
            context: "Test context".to_string(),
            prompt: "Test prompt".to_string(),
            model_tier: ModelTier::Tier5LLM,
            max_tokens: None,
        };

        // First request should require LLM
        let decision = optimizer.process_requirement(request.clone()).await.unwrap();
        match decision {
            Decision::RequiresLLM(_) => {}
            _ => panic!("Expected RequiresLLM decision"),
        }

        // Learn from response
        optimizer.learn_from_response(&request, "Test response").await.unwrap();

        // Second identical request should use cache
        let decision = optimizer.process_requirement(request).await.unwrap();
        match decision {
            Decision::UseCached(_) => {}
            _ => panic!("Expected UseCached decision"),
        }
    }

    #[test]
    fn test_similarity_calculation() {
        let index = SimilarityIndex::new();

        let text1 = "implement function to calculate fibonacci";
        let text2 = "implement function to compute fibonacci";
        let score = index.calculate_similarity(text1, text2);

        assert!(score > 0.7); // Should be similar
    }
}
