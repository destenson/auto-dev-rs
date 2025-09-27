//! Metrics collection implementation

use super::{
    CapabilityMetrics, DevelopmentMetrics, MetricType, MetricsError, MetricsSnapshot,
    PerformanceMetrics, QualityMetrics, Result,
};
use crate::learning::success_tracker::SuccessTracker;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
#[cfg(feature = "tracing")]
use tracing::{debug, info, warn};
#[cfg(feature = "tracing")]
#[cfg(feature = "log")]
use tracing::{debug, info, warn};

/// Event that triggers metric collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub module: String,
    pub success: bool,
    pub duration_ms: u64,
    pub metadata: HashMap<String, String>,
}

/// Collects and aggregates metrics
pub struct MetricsCollector {
    events: Arc<RwLock<Vec<MetricEvent>>>,
    snapshots: Arc<RwLock<Vec<MetricsSnapshot>>>,
    storage_path: PathBuf,
    success_tracker: Option<Arc<SuccessTracker>>,
}

impl MetricsCollector {
    pub async fn new() -> Result<Self> {
        let storage_path = Path::new(".auto-dev/metrics");
        std::fs::create_dir_all(&storage_path).map_err(|e| {
            MetricsError::Storage(format!("Failed to create metrics directory: {}", e))
        })?;

        let mut collector = Self {
            events: Arc::new(RwLock::new(Vec::new())),
            snapshots: Arc::new(RwLock::new(Vec::new())),
            storage_path: storage_path.to_path_buf(),
            success_tracker: None,
        };

        // Load existing metrics
        collector.load_historical_data().await?;

        Ok(collector)
    }

    /// Record a metric event
    pub async fn record_event(&self, event: MetricEvent) -> Result<()> {
        let mut events = self.events.write().await;
        events.push(event.clone());

        // Keep only last 10000 events
        if events.len() > 10000 {
            events.drain(0..1000);
        }

        debug!("Recorded metric event: {}", event.event_type);

        // Trigger aggregation if needed
        if events.len() % 100 == 0 {
            drop(events);
            self.aggregate_metrics().await?;
        }

        Ok(())
    }

    /// Get current metrics snapshot
    pub async fn get_current_snapshot(&self) -> Result<MetricsSnapshot> {
        self.aggregate_metrics().await?;

        let snapshots = self.snapshots.read().await;
        snapshots
            .last()
            .cloned()
            .ok_or_else(|| MetricsError::Collection("No metrics snapshot available".to_string()))
    }

    /// Get previous snapshot for comparison
    pub async fn get_previous_snapshot(&self) -> Result<Option<MetricsSnapshot>> {
        let snapshots = self.snapshots.read().await;
        if snapshots.len() >= 2 {
            Ok(snapshots.get(snapshots.len() - 2).cloned())
        } else {
            Ok(None)
        }
    }

    /// Aggregate events into a metrics snapshot
    async fn aggregate_metrics(&self) -> Result<()> {
        let events = self.events.read().await;

        if events.is_empty() {
            return Ok(());
        }

        let development = self.calculate_development_metrics(&events).await;
        let quality = self.calculate_quality_metrics().await;
        let performance = self.calculate_performance_metrics().await;
        let capability = self.calculate_capability_metrics(&events).await;

        let snapshot = MetricsSnapshot {
            timestamp: Utc::now(),
            development,
            quality,
            performance,
            capability,
        };

        let mut snapshots = self.snapshots.write().await;
        snapshots.push(snapshot.clone());

        // Keep only last 1000 snapshots
        if snapshots.len() > 1000 {
            snapshots.drain(0..100);
        }

        // Persist snapshot
        self.save_snapshot(&snapshot).await?;

        info!("Aggregated metrics snapshot");
        Ok(())
    }

    async fn calculate_development_metrics(&self, events: &[MetricEvent]) -> DevelopmentMetrics {
        let total_events = events.len() as f64;
        let successful_events = events.iter().filter(|e| e.success).count() as f64;
        let success_rate = if total_events > 0.0 { successful_events / total_events } else { 0.0 };

        let avg_duration = if !events.is_empty() {
            events.iter().map(|e| e.duration_ms).sum::<u64>() / events.len() as u64
        } else {
            0
        };

        // Calculate modifications per day based on event frequency
        let modifications_per_day =
            if let (Some(first), Some(last)) = (events.first(), events.last()) {
                let duration_days = (last.timestamp - first.timestamp).num_days().max(1) as f64;
                total_events / duration_days
            } else {
                0.0
            };

        // Calculate rollback frequency
        let rollback_events =
            events.iter().filter(|e| e.event_type.contains("rollback")).count() as f64;
        let rollback_frequency =
            if total_events > 0.0 { rollback_events / total_events } else { 0.0 };

        DevelopmentMetrics {
            modifications_per_day,
            success_rate,
            implementation_time_avg_ms: avg_duration,
            rollback_frequency,
            test_coverage_percent: self.get_test_coverage().await,
        }
    }

    async fn calculate_quality_metrics(&self) -> QualityMetrics {
        // These would be calculated from actual code analysis
        // For now, return placeholder values
        QualityMetrics {
            cyclomatic_complexity: 5.0,
            documentation_coverage: 75.0,
            lint_warnings: 10,
            lint_errors: 0,
            duplicate_code_percent: 2.5,
            technical_debt_score: 3.0,
        }
    }

    async fn calculate_performance_metrics(&self) -> PerformanceMetrics {
        // These would be collected from actual measurements
        // For now, return placeholder values
        PerformanceMetrics {
            compilation_time_ms: 5000,
            test_execution_time_ms: 2000,
            binary_size_bytes: 10_000_000,
            memory_usage_mb: 50.0,
            module_load_time_ms: 100,
        }
    }

    async fn calculate_capability_metrics(&self, events: &[MetricEvent]) -> CapabilityMetrics {
        let features_added =
            events.iter().filter(|e| e.event_type.contains("feature_add")).count() as u32;

        let apis_created =
            events.iter().filter(|e| e.event_type.contains("api_create")).count() as u32;

        let modules_loaded =
            events.iter().filter(|e| e.event_type.contains("module_load")).count() as u32;

        let patterns_learned =
            events.iter().filter(|e| e.event_type.contains("pattern_learn")).count() as u32;

        let llm_calls_saved =
            events.iter().filter(|e| e.event_type.contains("llm_cache_hit")).count() as u32;

        CapabilityMetrics {
            features_added,
            apis_created,
            modules_loaded,
            patterns_learned,
            llm_calls_saved,
        }
    }

    async fn get_test_coverage(&self) -> f64 {
        // Would integrate with actual test coverage tools
        // For now return a placeholder
        75.0
    }

    async fn save_snapshot(&self, snapshot: &MetricsSnapshot) -> Result<()> {
        let filename = format!("snapshot_{}.json", snapshot.timestamp.timestamp());
        let path = self.storage_path.join(filename);

        let json = serde_json::to_string_pretty(snapshot)?;
        tokio::fs::write(&path, json).await?;

        debug!("Saved metrics snapshot to {:?}", path);
        Ok(())
    }

    async fn load_historical_data(&mut self) -> Result<()> {
        let mut snapshots = Vec::new();

        if let Ok(mut entries) = tokio::fs::read_dir(&self.storage_path).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = tokio::fs::read_to_string(&path).await {
                        if let Ok(snapshot) = serde_json::from_str::<MetricsSnapshot>(&content) {
                            snapshots.push(snapshot);
                        }
                    }
                }
            }
        }

        snapshots.sort_by_key(|s| s.timestamp);

        let mut stored_snapshots = self.snapshots.write().await;
        *stored_snapshots = snapshots;

        info!("Loaded {} historical snapshots", stored_snapshots.len());
        Ok(())
    }

    /// Export metrics for a given time range
    pub async fn export_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<MetricsSnapshot>> {
        let snapshots = self.snapshots.read().await;
        Ok(snapshots
            .iter()
            .filter(|s| s.timestamp >= start && s.timestamp <= end)
            .cloned()
            .collect())
    }
}
