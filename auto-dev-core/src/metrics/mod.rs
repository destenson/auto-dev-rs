#![allow(unused)]
//! Self-improvement metrics collection and monitoring
//! 
//! This module provides comprehensive metrics tracking for auto-dev-rs's 
//! self-development activities, enabling data-driven improvement decisions.

pub mod collector;
pub mod storage;
pub mod analyzer;
pub mod exporter;
pub mod dashboard;

pub use collector::{MetricsCollector, MetricEvent};
pub use storage::{TimeSeriesStore, MetricPoint};
pub use analyzer::{TrendAnalyzer, TrendDirection};
pub use exporter::{MetricsExporter, ExportFormat};
pub use dashboard::{MetricsDashboard, DashboardConfig};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MetricsError {
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Collection error: {0}")]
    Collection(String),
    
    #[error("Analysis error: {0}")]
    Analysis(String),
    
    #[error("Export error: {0}")]
    Export(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, MetricsError>;

/// Types of metrics we track
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MetricType {
    Development,
    Quality,
    Performance,
    Capability,
}

/// Development metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentMetrics {
    pub modifications_per_day: f64,
    pub success_rate: f64,
    pub implementation_time_avg_ms: u64,
    pub rollback_frequency: f64,
    pub test_coverage_percent: f64,
}

/// Code quality metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub cyclomatic_complexity: f64,
    pub documentation_coverage: f64,
    pub lint_warnings: u32,
    pub lint_errors: u32,
    pub duplicate_code_percent: f64,
    pub technical_debt_score: f64,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub compilation_time_ms: u64,
    pub test_execution_time_ms: u64,
    pub binary_size_bytes: u64,
    pub memory_usage_mb: f64,
    pub module_load_time_ms: u64,
}

/// Capability metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMetrics {
    pub features_added: u32,
    pub apis_created: u32,
    pub modules_loaded: u32,
    pub patterns_learned: u32,
    pub llm_calls_saved: u32,
}

/// Combined metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: DateTime<Utc>,
    pub development: DevelopmentMetrics,
    pub quality: QualityMetrics,
    pub performance: PerformanceMetrics,
    pub capability: CapabilityMetrics,
}

/// Improvement score calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementScore {
    pub overall_score: f64,
    pub development_score: f64,
    pub quality_score: f64,
    pub performance_score: f64,
    pub capability_score: f64,
    pub trend: TrendDirection,
    pub confidence: f64,
}

impl ImprovementScore {
    pub fn calculate(current: &MetricsSnapshot, previous: Option<&MetricsSnapshot>) -> Self {
        if let Some(prev) = previous {
            let dev_score = Self::calc_development_score(&current.development, &prev.development);
            let qual_score = Self::calc_quality_score(&current.quality, &prev.quality);
            let perf_score = Self::calc_performance_score(&current.performance, &prev.performance);
            let cap_score = Self::calc_capability_score(&current.capability, &prev.capability);
            
            let overall = (dev_score + qual_score + perf_score + cap_score) / 4.0;
            
            let trend = if overall > 0.05 {
                TrendDirection::Improving
            } else if overall < -0.05 {
                TrendDirection::Declining
            } else {
                TrendDirection::Stable
            };
            
            Self {
                overall_score: overall,
                development_score: dev_score,
                quality_score: qual_score,
                performance_score: perf_score,
                capability_score: cap_score,
                trend,
                confidence: 0.8, // TODO: Calculate based on data points
            }
        } else {
            Self {
                overall_score: 0.0,
                development_score: 0.0,
                quality_score: 0.0,
                performance_score: 0.0,
                capability_score: 0.0,
                trend: TrendDirection::Unknown,
                confidence: 0.0,
            }
        }
    }
    
    fn calc_development_score(current: &DevelopmentMetrics, previous: &DevelopmentMetrics) -> f64 {
        let mut score = 0.0;
        
        // Higher success rate is better
        score += (current.success_rate - previous.success_rate) * 0.3;
        
        // More modifications is better (productivity)
        score += ((current.modifications_per_day - previous.modifications_per_day) / previous.modifications_per_day.max(1.0)) * 0.2;
        
        // Faster implementation is better
        if previous.implementation_time_avg_ms > 0 {
            score += ((previous.implementation_time_avg_ms as f64 - current.implementation_time_avg_ms as f64) / previous.implementation_time_avg_ms as f64) * 0.2;
        }
        
        // Lower rollback frequency is better
        score += (previous.rollback_frequency - current.rollback_frequency) * 0.15;
        
        // Higher test coverage is better
        score += ((current.test_coverage_percent - previous.test_coverage_percent) / 100.0) * 0.15;
        
        score
    }
    
    fn calc_quality_score(current: &QualityMetrics, previous: &QualityMetrics) -> f64 {
        let mut score = 0.0;
        
        // Lower complexity is better
        if previous.cyclomatic_complexity > 0.0 {
            score += ((previous.cyclomatic_complexity - current.cyclomatic_complexity) / previous.cyclomatic_complexity) * 0.25;
        }
        
        // Higher documentation coverage is better
        score += ((current.documentation_coverage - previous.documentation_coverage) / 100.0) * 0.2;
        
        // Fewer warnings/errors is better
        score += ((previous.lint_warnings as f64 - current.lint_warnings as f64) / previous.lint_warnings.max(1) as f64) * 0.15;
        score += ((previous.lint_errors as f64 - current.lint_errors as f64) / previous.lint_errors.max(1) as f64) * 0.2;
        
        // Less duplicate code is better
        score += ((previous.duplicate_code_percent - current.duplicate_code_percent) / 100.0) * 0.1;
        
        // Lower technical debt is better
        if previous.technical_debt_score > 0.0 {
            score += ((previous.technical_debt_score - current.technical_debt_score) / previous.technical_debt_score) * 0.1;
        }
        
        score
    }
    
    fn calc_performance_score(current: &PerformanceMetrics, previous: &PerformanceMetrics) -> f64 {
        let mut score = 0.0;
        
        // Faster compilation is better
        if previous.compilation_time_ms > 0 {
            score += ((previous.compilation_time_ms as f64 - current.compilation_time_ms as f64) / previous.compilation_time_ms as f64) * 0.25;
        }
        
        // Faster tests are better
        if previous.test_execution_time_ms > 0 {
            score += ((previous.test_execution_time_ms as f64 - current.test_execution_time_ms as f64) / previous.test_execution_time_ms as f64) * 0.2;
        }
        
        // Smaller binary is better (within reason)
        if previous.binary_size_bytes > 0 {
            score += ((previous.binary_size_bytes as f64 - current.binary_size_bytes as f64) / previous.binary_size_bytes as f64) * 0.15;
        }
        
        // Lower memory usage is better
        if previous.memory_usage_mb > 0.0 {
            score += ((previous.memory_usage_mb - current.memory_usage_mb) / previous.memory_usage_mb) * 0.2;
        }
        
        // Faster module loading is better
        if previous.module_load_time_ms > 0 {
            score += ((previous.module_load_time_ms as f64 - current.module_load_time_ms as f64) / previous.module_load_time_ms as f64) * 0.2;
        }
        
        score
    }
    
    fn calc_capability_score(current: &CapabilityMetrics, previous: &CapabilityMetrics) -> f64 {
        let mut score = 0.0;
        
        // More features is better
        score += ((current.features_added - previous.features_added) as f64) * 0.01;
        
        // More APIs is better
        score += ((current.apis_created - previous.apis_created) as f64) * 0.01;
        
        // More modules is better
        score += ((current.modules_loaded - previous.modules_loaded) as f64) * 0.005;
        
        // More patterns learned is better
        score += ((current.patterns_learned - previous.patterns_learned) as f64) * 0.02;
        
        // More LLM calls saved is better (efficiency)
        score += ((current.llm_calls_saved - previous.llm_calls_saved) as f64) * 0.001;
        
        score.min(1.0) // Cap at 1.0
    }
}

/// Initialize the metrics system
pub async fn initialize() -> Result<MetricsCollector> {
    MetricsCollector::new().await
}

/// Get current metrics snapshot
pub async fn get_snapshot() -> Result<MetricsSnapshot> {
    let collector = MetricsCollector::new().await?;
    collector.get_current_snapshot().await
}

/// Calculate improvement score
pub async fn calculate_improvement() -> Result<ImprovementScore> {
    let collector = MetricsCollector::new().await?;
    let current = collector.get_current_snapshot().await?;
    let previous = collector.get_previous_snapshot().await?;
    Ok(ImprovementScore::calculate(&current, previous.as_ref()))
}
