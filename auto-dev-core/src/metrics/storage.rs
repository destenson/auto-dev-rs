//! Time-series storage for metrics

use super::{MetricsError, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// A single metric data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub labels: BTreeMap<String, String>,
}

/// Time-series storage for metrics
pub struct TimeSeriesStore {
    series: BTreeMap<String, Vec<MetricPoint>>,
    storage_path: PathBuf,
    retention_days: i64,
}

impl TimeSeriesStore {
    pub fn new(storage_path: impl AsRef<Path>) -> Result<Self> {
        let path = storage_path.as_ref().to_path_buf();
        std::fs::create_dir_all(&path)
            .map_err(|e| MetricsError::Storage(format!("Failed to create storage directory: {}", e)))?;
        
        let mut store = Self {
            series: BTreeMap::new(),
            storage_path: path,
            retention_days: 30, // Default 30 days retention
        };
        
        store.load_from_disk()?;
        Ok(store)
    }
    
    /// Add a data point to a time series
    pub fn add_point(&mut self, series_name: &str, point: MetricPoint) -> Result<()> {
        let series = self.series.entry(series_name.to_string()).or_insert_with(Vec::new);
        
        // Insert in chronological order
        match series.binary_search_by_key(&point.timestamp, |p| p.timestamp) {
            Ok(pos) => series[pos] = point, // Update existing point
            Err(pos) => series.insert(pos, point),
        }
        
        // Apply retention policy
        self.apply_retention(series_name)?;
        
        debug!("Added point to series '{}'", series_name);
        Ok(())
    }
    
    /// Get points within a time range
    pub fn query_range(
        &self,
        series_name: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<MetricPoint> {
        self.series
            .get(series_name)
            .map(|series| {
                series
                    .iter()
                    .filter(|p| p.timestamp >= start && p.timestamp <= end)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get the latest point from a series
    pub fn get_latest(&self, series_name: &str) -> Option<MetricPoint> {
        self.series
            .get(series_name)
            .and_then(|series| series.last().cloned())
    }
    
    /// Calculate aggregates over a time range
    pub fn aggregate(
        &self,
        series_name: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        aggregation: AggregationType,
    ) -> Option<f64> {
        let points = self.query_range(series_name, start, end);
        
        if points.is_empty() {
            return None;
        }
        
        let values: Vec<f64> = points.iter().map(|p| p.value).collect();
        
        match aggregation {
            AggregationType::Sum => Some(values.iter().sum()),
            AggregationType::Average => Some(values.iter().sum::<f64>() / values.len() as f64),
            AggregationType::Min => values.iter().cloned().min_by(|a, b| a.partial_cmp(b).unwrap()),
            AggregationType::Max => values.iter().cloned().max_by(|a, b| a.partial_cmp(b).unwrap()),
            AggregationType::Count => Some(values.len() as f64),
            AggregationType::Percentile(p) => self.calculate_percentile(&values, p),
        }
    }
    
    /// Downsample data for visualization
    pub fn downsample(
        &self,
        series_name: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        max_points: usize,
    ) -> Vec<MetricPoint> {
        let points = self.query_range(series_name, start, end);
        
        if points.len() <= max_points {
            return points;
        }
        
        // Simple downsampling by taking every nth point
        let step = points.len() / max_points;
        points.into_iter().step_by(step).collect()
    }
    
    /// Apply retention policy to remove old data
    fn apply_retention(&mut self, series_name: &str) -> Result<()> {
        let cutoff = Utc::now() - Duration::days(self.retention_days);
        
        if let Some(series) = self.series.get_mut(series_name) {
            let before = series.len();
            series.retain(|p| p.timestamp > cutoff);
            let after = series.len();
            
            if before != after {
                debug!("Removed {} old points from series '{}'", before - after, series_name);
            }
        }
        
        Ok(())
    }
    
    /// Persist time series to disk
    pub fn save_to_disk(&self) -> Result<()> {
        for (name, series) in &self.series {
            let filename = format!("{}.json", name.replace('/', "_"));
            let path = self.storage_path.join(filename);
            
            let json = serde_json::to_string_pretty(series)?;
            std::fs::write(&path, json)?;
        }
        
        info!("Saved {} time series to disk", self.series.len());
        Ok(())
    }
    
    /// Load time series from disk
    fn load_from_disk(&mut self) -> Result<()> {
        let entries = std::fs::read_dir(&self.storage_path)
            .map_err(|e| MetricsError::Storage(format!("Failed to read storage directory: {}", e)))?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    let series_name = name.replace('_', "/");
                    
                    match std::fs::read_to_string(&path) {
                        Ok(content) => {
                            match serde_json::from_str::<Vec<MetricPoint>>(&content) {
                                Ok(series) => {
                                    self.series.insert(series_name, series);
                                }
                                Err(e) => {
                                    warn!("Failed to parse series file {:?}: {}", path, e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to read series file {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
        
        info!("Loaded {} time series from disk", self.series.len());
        Ok(())
    }
    
    fn calculate_percentile(&self, values: &[f64], percentile: f64) -> Option<f64> {
        if values.is_empty() || percentile < 0.0 || percentile > 100.0 {
            return None;
        }
        
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let index = (percentile / 100.0 * (sorted.len() - 1) as f64).round() as usize;
        sorted.get(index).cloned()
    }
    
    /// Get all series names
    pub fn list_series(&self) -> Vec<String> {
        self.series.keys().cloned().collect()
    }
    
    /// Delete a time series
    pub fn delete_series(&mut self, series_name: &str) -> Result<()> {
        self.series.remove(series_name);
        
        let filename = format!("{}.json", series_name.replace('/', "_"));
        let path = self.storage_path.join(filename);
        
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        
        info!("Deleted series '{}'", series_name);
        Ok(())
    }
}

/// Types of aggregation for metrics
#[derive(Debug, Clone)]
pub enum AggregationType {
    Sum,
    Average,
    Min,
    Max,
    Count,
    Percentile(f64),
}

impl Drop for TimeSeriesStore {
    fn drop(&mut self) {
        // Save to disk on drop
        if let Err(e) = self.save_to_disk() {
            warn!("Failed to save time series on drop: {}", e);
        }
    }
}
