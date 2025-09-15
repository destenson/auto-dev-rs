//! Decision engine for determining actions without LLM when possible

use super::*;
use anyhow::Result;
use std::path::Path;
use tracing::{debug, trace};

/// Makes decisions based on events and context
pub struct DecisionEngine {
    rules: Vec<Box<dyn DecisionRule>>,
    patterns: PatternMatcher,
    cache: DecisionCache,
}

impl DecisionEngine {
    pub fn new() -> Self {
        Self {
            rules: Self::default_rules(),
            patterns: PatternMatcher::new(),
            cache: DecisionCache::new(),
        }
    }

    /// Make a decision based on an event
    pub async fn decide(&self, event: &Event) -> Result<Decision> {
        trace!("Making decision for event: {:?}", event.event_type);
        
        // Check cache first
        if let Some(cached) = self.cache.get(event).await {
            debug!("Using cached decision for event {}", event.id);
            return Ok(cached);
        }
        
        // Apply rules in order
        for rule in &self.rules {
            if let Some(decision) = rule.evaluate(event).await? {
                debug!("Rule {} matched for event {}", rule.name(), event.id);
                self.cache.store(event, decision.clone()).await;
                return Ok(decision);
            }
        }
        
        // Check for patterns
        if let Some(pattern) = self.patterns.find_match(event).await? {
            debug!("Pattern {} matched for event {}", pattern, event.id);
            return Ok(Decision::UsePattern(pattern));
        }
        
        // Default: requires analysis
        debug!("Event {} requires LLM analysis", event.id);
        Ok(Decision::RequiresLLM(LLMRequest {
            context: self.build_context(event).await?,
            prompt: self.build_prompt(event),
            model_tier: ModelTier::Tier5LLM,
            max_tokens: None,
        }))
    }

    /// Build context for LLM request
    async fn build_context(&self, event: &Event) -> Result<String> {
        let mut context = String::new();
        
        context.push_str(&format!("Event Type: {:?}\n", event.event_type));
        context.push_str(&format!("Source: {}\n", event.source.display()));
        context.push_str(&format!("Priority: {:?}\n", event.priority));
        
        // Add file content if available
        if event.source.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&event.source).await {
                context.push_str("\nFile Content:\n");
                context.push_str(&content);
            }
        }
        
        Ok(context)
    }

    /// Build prompt for LLM
    fn build_prompt(&self, event: &Event) -> String {
        match event.event_type {
            EventType::SpecificationChanged => {
                "Analyze this specification change and determine what implementation is needed.".to_string()
            },
            EventType::TestAdded => {
                "Analyze this new test and implement code to make it pass.".to_string()
            },
            EventType::TestFailed => {
                "Analyze this test failure and fix the implementation.".to_string()
            },
            _ => {
                "Analyze this event and determine the appropriate action.".to_string()
            },
        }
    }

    /// Get default rules
    fn default_rules() -> Vec<Box<dyn DecisionRule>> {
        vec![
            Box::new(SpecificationChangeRule),
            Box::new(TestAddedRule),
            Box::new(TestFailedRule),
            Box::new(DependencyUpdateRule),
            Box::new(ConfigChangeRule),
        ]
    }
}

/// Trait for decision rules
#[async_trait::async_trait]
trait DecisionRule: Send + Sync {
    fn name(&self) -> &str;
    async fn evaluate(&self, event: &Event) -> Result<Option<Decision>>;
}

/// Rule for specification changes
struct SpecificationChangeRule;

#[async_trait::async_trait]
impl DecisionRule for SpecificationChangeRule {
    fn name(&self) -> &str {
        "SpecificationChange"
    }
    
    async fn evaluate(&self, event: &Event) -> Result<Option<Decision>> {
        if event.event_type != EventType::SpecificationChanged {
            return Ok(None);
        }
        
        // Check if implementation exists
        let impl_path = event.source.with_extension("rs");
        
        if impl_path.exists() {
            // Update existing implementation
            Ok(Some(Decision::Implement(ImplementationTask {
                spec_path: event.source.clone(),
                target_path: impl_path,
                requirements: vec![],
                incremental: true,
            })))
        } else {
            // New implementation needed
            Ok(Some(Decision::Implement(ImplementationTask {
                spec_path: event.source.clone(),
                target_path: impl_path,
                requirements: vec![],
                incremental: false,
            })))
        }
    }
}

/// Rule for new tests
struct TestAddedRule;

#[async_trait::async_trait]
impl DecisionRule for TestAddedRule {
    fn name(&self) -> &str {
        "TestAdded"
    }
    
    async fn evaluate(&self, event: &Event) -> Result<Option<Decision>> {
        if event.event_type != EventType::TestAdded {
            return Ok(None);
        }
        
        Ok(Some(Decision::UpdateTests(vec![
            TestUpdate {
                test_path: event.source.clone(),
                update_type: TestUpdateType::AddTest("Generated from specification".to_string()),
            }
        ])))
    }
}

/// Rule for test failures
struct TestFailedRule;

#[async_trait::async_trait]
impl DecisionRule for TestFailedRule {
    fn name(&self) -> &str {
        "TestFailed"
    }
    
    async fn evaluate(&self, event: &Event) -> Result<Option<Decision>> {
        if event.event_type != EventType::TestFailed {
            return Ok(None);
        }
        
        Ok(Some(Decision::UpdateTests(vec![
            TestUpdate {
                test_path: event.source.clone(),
                update_type: TestUpdateType::FixFailure("Fix test failure".to_string()),
            }
        ])))
    }
}

/// Rule for dependency updates
struct DependencyUpdateRule;

#[async_trait::async_trait]
impl DecisionRule for DependencyUpdateRule {
    fn name(&self) -> &str {
        "DependencyUpdate"
    }
    
    async fn evaluate(&self, event: &Event) -> Result<Option<Decision>> {
        if event.event_type != EventType::DependencyUpdated {
            return Ok(None);
        }
        
        // Usually skip dependency updates unless critical
        Ok(Some(Decision::Skip("Dependency update - no action needed".to_string())))
    }
}

/// Rule for configuration changes
struct ConfigChangeRule;

#[async_trait::async_trait]
impl DecisionRule for ConfigChangeRule {
    fn name(&self) -> &str {
        "ConfigChange"
    }
    
    async fn evaluate(&self, event: &Event) -> Result<Option<Decision>> {
        if event.event_type != EventType::ConfigurationChanged {
            return Ok(None);
        }
        
        // Usually requires restart or reload
        Ok(Some(Decision::Skip("Configuration change - manual restart may be needed".to_string())))
    }
}

/// Pattern matcher for known patterns
struct PatternMatcher {
    patterns: Arc<RwLock<HashMap<String, Pattern>>>,
}

impl PatternMatcher {
    fn new() -> Self {
        Self {
            patterns: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    async fn find_match(&self, event: &Event) -> Result<Option<String>> {
        let patterns = self.patterns.read().await;
        
        for (id, pattern) in patterns.iter() {
            if pattern.matches(event) {
                return Ok(Some(id.clone()));
            }
        }
        
        Ok(None)
    }
    
    pub async fn add_pattern(&self, id: String, pattern: Pattern) {
        let mut patterns = self.patterns.write().await;
        patterns.insert(id, pattern);
    }
}

/// Pattern definition
#[derive(Debug, Clone)]
struct Pattern {
    event_type: EventType,
    path_pattern: Option<String>,
    metadata_match: HashMap<String, serde_json::Value>,
}

impl Pattern {
    fn matches(&self, event: &Event) -> bool {
        if self.event_type != event.event_type {
            return false;
        }
        
        if let Some(pattern) = &self.path_pattern {
            let path_str = event.source.to_string_lossy();
            if !path_str.contains(pattern) {
                return false;
            }
        }
        
        for (key, value) in &self.metadata_match {
            if event.metadata.get(key) != Some(value) {
                return false;
            }
        }
        
        true
    }
}

/// Decision cache
struct DecisionCache {
    cache: Arc<Mutex<HashMap<String, (Decision, DateTime<Utc>)>>>,
    ttl: Duration,
}

impl DecisionCache {
    fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            ttl: Duration::from_secs(300), // 5 minutes
        }
    }
    
    async fn get(&self, event: &Event) -> Option<Decision> {
        let mut cache = self.cache.lock().await;
        let key = self.make_key(event);
        
        if let Some((decision, timestamp)) = cache.get(&key) {
            let age = Utc::now() - *timestamp;
            if age.to_std().unwrap() < self.ttl {
                return Some(decision.clone());
            } else {
                cache.remove(&key);
            }
        }
        
        None
    }
    
    async fn store(&self, event: &Event, decision: Decision) {
        let mut cache = self.cache.lock().await;
        let key = self.make_key(event);
        cache.insert(key, (decision, Utc::now()));
    }
    
    fn make_key(&self, event: &Event) -> String {
        format!("{:?}:{}", event.event_type, event.source.display())
    }
}

use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_decision_engine() {
        let engine = DecisionEngine::new();
        
        let event = Event::new(
            EventType::SpecificationChanged,
            PathBuf::from("test.md"),
        );
        
        let decision = engine.decide(&event).await.unwrap();
        
        match decision {
            Decision::Implement(task) => {
                assert_eq!(task.spec_path, PathBuf::from("test.md"));
                assert_eq!(task.target_path, PathBuf::from("test.rs"));
            },
            _ => panic!("Expected Implement decision"),
        }
    }

    #[tokio::test]
    async fn test_pattern_matching() {
        let matcher = PatternMatcher::new();
        
        let pattern = Pattern {
            event_type: EventType::TestAdded,
            path_pattern: Some("test".to_string()),
            metadata_match: HashMap::new(),
        };
        
        matcher.add_pattern("test_pattern".to_string(), pattern).await;
        
        let event = Event::new(
            EventType::TestAdded,
            PathBuf::from("test_file.rs"),
        );
        
        let result = matcher.find_match(&event).await.unwrap();
        assert_eq!(result, Some("test_pattern".to_string()));
    }
}