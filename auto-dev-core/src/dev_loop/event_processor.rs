//! Event processing for the development loop

use super::*;
use anyhow::Result;
use std::path::Path;
use tracing::{debug, trace};

/// Processes events from various sources
pub struct EventProcessor {
    event_filters: Vec<Box<dyn EventFilter>>,
    event_transformers: Vec<Box<dyn EventTransformer>>,
}

impl EventProcessor {
    pub fn new() -> Self {
        Self {
            event_filters: vec![
                Box::new(DeduplicationFilter::new()),
                Box::new(RateLimitFilter::new()),
            ],
            event_transformers: vec![
                Box::new(PriorityAssigner::new()),
                Box::new(MetadataEnricher::new()),
            ],
        }
    }

    /// Process a raw event
    pub async fn process(&self, mut event: Event) -> Result<Option<Event>> {
        trace!("Processing event: {:?}", event.id);
        
        // Apply filters
        for filter in &self.event_filters {
            if !filter.should_process(&event).await? {
                debug!("Event {} filtered out", event.id);
                return Ok(None);
            }
        }
        
        // Apply transformations
        for transformer in &self.event_transformers {
            event = transformer.transform(event).await?;
        }
        
        Ok(Some(event))
    }

    /// Handle filesystem event
    pub async fn handle_fs_event(&self, path: &Path, change_type: ChangeType) -> Result<Option<Event>> {
        let event_type = match change_type {
            ChangeType::Created | ChangeType::Modified => {
                if path.extension().map_or(false, |ext| ext == "md") {
                    EventType::SpecificationChanged
                } else if path.to_str().map_or(false, |s| s.contains("test")) {
                    EventType::TestAdded
                } else {
                    EventType::CodeModified
                }
            },
            ChangeType::Deleted => {
                return Ok(None); // Ignore deletions for now
            },
        };
        
        let event = Event::new(event_type, path.to_path_buf());
        self.process(event).await
    }
}

/// Change type for filesystem events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
}

/// Trait for event filters
#[async_trait::async_trait]
trait EventFilter: Send + Sync {
    async fn should_process(&self, event: &Event) -> Result<bool>;
}

/// Trait for event transformers
#[async_trait::async_trait]
trait EventTransformer: Send + Sync {
    async fn transform(&self, event: Event) -> Result<Event>;
}

/// Deduplication filter
struct DeduplicationFilter {
    recent_events: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
}

impl DeduplicationFilter {
    fn new() -> Self {
        Self {
            recent_events: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl EventFilter for DeduplicationFilter {
    async fn should_process(&self, event: &Event) -> Result<bool> {
        let mut recent = self.recent_events.lock().await;
        
        let key = format!("{:?}:{}", event.event_type, event.source.display());
        
        if let Some(last_seen) = recent.get(&key) {
            let elapsed = event.timestamp - *last_seen;
            if elapsed.num_milliseconds() < 500 {
                return Ok(false); // Debounce
            }
        }
        
        recent.insert(key, event.timestamp);
        Ok(true)
    }
}

/// Rate limiting filter
struct RateLimitFilter {
    rate_limits: Arc<Mutex<HashMap<EventType, RateLimit>>>,
}

impl RateLimitFilter {
    fn new() -> Self {
        Self {
            rate_limits: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl EventFilter for RateLimitFilter {
    async fn should_process(&self, event: &Event) -> Result<bool> {
        let mut limits = self.rate_limits.lock().await;
        
        let limit = limits
            .entry(event.event_type.clone())
            .or_insert_with(|| RateLimit::new(10, Duration::from_secs(60)));
        
        Ok(limit.check())
    }
}

/// Rate limit tracker
struct RateLimit {
    max_events: usize,
    window: Duration,
    events: Vec<DateTime<Utc>>,
}

impl RateLimit {
    fn new(max_events: usize, window: Duration) -> Self {
        Self {
            max_events,
            window,
            events: Vec::new(),
        }
    }
    
    fn check(&mut self) -> bool {
        let now = Utc::now();
        let cutoff = now - chrono::Duration::from_std(self.window).unwrap();
        
        // Remove old events
        self.events.retain(|&e| e > cutoff);
        
        if self.events.len() < self.max_events {
            self.events.push(now);
            true
        } else {
            false
        }
    }
}

/// Priority assigner
struct PriorityAssigner;

impl PriorityAssigner {
    fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl EventTransformer for PriorityAssigner {
    async fn transform(&self, mut event: Event) -> Result<Event> {
        // Assign priority based on event type
        event.priority = match event.event_type {
            EventType::TestFailed => Priority::Critical,
            EventType::SpecificationChanged => Priority::High,
            EventType::TestAdded => Priority::High,
            EventType::CodeModified => Priority::Medium,
            EventType::DependencyUpdated => Priority::Medium,
            EventType::ConfigurationChanged => Priority::Low,
            EventType::HealthCheck => Priority::Background,
            EventType::UserCommand(_) => Priority::High,
        };
        
        Ok(event)
    }
}

/// Metadata enricher
struct MetadataEnricher;

impl MetadataEnricher {
    fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl EventTransformer for MetadataEnricher {
    async fn transform(&self, mut event: Event) -> Result<Event> {
        // Add file size if file exists
        if event.source.exists() {
            if let Ok(metadata) = tokio::fs::metadata(&event.source).await {
                event.metadata.insert(
                    "file_size".to_string(),
                    serde_json::Value::Number(metadata.len().into()),
                );
            }
        }
        
        // Add file extension
        if let Some(ext) = event.source.extension() {
            event.metadata.insert(
                "extension".to_string(),
                serde_json::Value::String(ext.to_string_lossy().to_string()),
            );
        }
        
        Ok(event)
    }
}

use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_processor() {
        let processor = EventProcessor::new();
        
        let event = Event::new(
            EventType::SpecificationChanged,
            PathBuf::from("test.md"),
        );
        
        let processed = processor.process(event).await.unwrap();
        assert!(processed.is_some());
        
        if let Some(event) = processed {
            // Priority should be assigned
            assert_eq!(event.priority, Priority::High);
        }
    }

    #[tokio::test]
    async fn test_deduplication() {
        let filter = DeduplicationFilter::new();
        
        let event = Event::new(
            EventType::CodeModified,
            PathBuf::from("test.rs"),
        );
        
        // First event should pass
        assert!(filter.should_process(&event).await.unwrap());
        
        // Immediate duplicate should be filtered
        assert!(!filter.should_process(&event).await.unwrap());
        
        // After delay, should pass again
        tokio::time::sleep(Duration::from_millis(600)).await;
        assert!(filter.should_process(&event).await.unwrap());
    }
}