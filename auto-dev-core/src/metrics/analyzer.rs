//! Trend analysis for metrics

use super::storage::TimeSeriesStore;
use super::{MetricPoint, MetricsError, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Direction of a trend
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TrendDirection {
    Improving,
    Declining,
    Stable,
    Unknown,
}

/// Trend analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub direction: TrendDirection,
    pub slope: f64,
    pub confidence: f64,
    pub change_percent: f64,
    pub prediction: Option<f64>,
}

/// Analyzes trends in metrics
pub struct TrendAnalyzer {
    store: TimeSeriesStore,
    min_data_points: usize,
}

impl TrendAnalyzer {
    pub fn new(store: TimeSeriesStore) -> Self {
        Self { store, min_data_points: 5 }
    }

    /// Analyze trend for a metric series
    pub fn analyze_series(&self, series_name: &str, window: Duration) -> Result<TrendAnalysis> {
        let end = Utc::now();
        let start = end - window;

        let points = self.store.query_range(series_name, start, end);

        if points.len() < self.min_data_points {
            return Ok(TrendAnalysis {
                direction: TrendDirection::Unknown,
                slope: 0.0,
                confidence: 0.0,
                change_percent: 0.0,
                prediction: None,
            });
        }

        self.calculate_trend(&points)
    }

    /// Calculate trend from data points
    fn calculate_trend(&self, points: &[MetricPoint]) -> Result<TrendAnalysis> {
        let n = points.len() as f64;

        // Convert timestamps to numeric values (seconds since first point)
        let first_timestamp = points[0].timestamp;
        let x_values: Vec<f64> =
            points.iter().map(|p| (p.timestamp - first_timestamp).num_seconds() as f64).collect();

        let y_values: Vec<f64> = points.iter().map(|p| p.value).collect();

        // Calculate linear regression
        let (slope, intercept) = self.linear_regression(&x_values, &y_values);

        // Calculate R-squared for confidence
        let r_squared = self.calculate_r_squared(&x_values, &y_values, slope, intercept);

        // Determine trend direction
        let direction = if r_squared < 0.3 {
            TrendDirection::Stable // Low confidence means stable/noisy
        } else if slope > 0.01 {
            TrendDirection::Improving
        } else if slope < -0.01 {
            TrendDirection::Declining
        } else {
            TrendDirection::Stable
        };

        // Calculate percent change
        let first_value = y_values.first().unwrap_or(&0.0);
        let last_value = y_values.last().unwrap_or(&0.0);
        let change_percent = if *first_value != 0.0 {
            ((last_value - first_value) / first_value) * 100.0
        } else {
            0.0
        };

        // Make a simple prediction for the next point
        let next_x = x_values.last().unwrap_or(&0.0) + 3600.0; // 1 hour ahead
        let prediction = Some(slope * next_x + intercept);

        Ok(TrendAnalysis { direction, slope, confidence: r_squared, change_percent, prediction })
    }

    /// Simple linear regression
    fn linear_regression(&self, x: &[f64], y: &[f64]) -> (f64, f64) {
        let n = x.len() as f64;

        let sum_x: f64 = x.iter().sum();
        let sum_y: f64 = y.iter().sum();
        let sum_xy: f64 = x.iter().zip(y.iter()).map(|(a, b)| a * b).sum();
        let sum_x2: f64 = x.iter().map(|a| a * a).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;

        (slope, intercept)
    }

    /// Calculate R-squared value
    fn calculate_r_squared(&self, x: &[f64], y: &[f64], slope: f64, intercept: f64) -> f64 {
        let y_mean = y.iter().sum::<f64>() / y.len() as f64;

        let ss_tot: f64 = y.iter().map(|yi| (yi - y_mean).powi(2)).sum();
        let ss_res: f64 =
            x.iter().zip(y.iter()).map(|(xi, yi)| (yi - (slope * xi + intercept)).powi(2)).sum();

        if ss_tot == 0.0 { 0.0 } else { 1.0 - (ss_res / ss_tot) }
    }

    /// Detect anomalies in recent data
    pub fn detect_anomalies(
        &self,
        series_name: &str,
        window: Duration,
        threshold_std: f64,
    ) -> Vec<MetricPoint> {
        let end = Utc::now();
        let start = end - window;

        let points = self.store.query_range(series_name, start, end);

        if points.len() < 3 {
            return Vec::new();
        }

        let values: Vec<f64> = points.iter().map(|p| p.value).collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();

        points.into_iter().filter(|p| (p.value - mean).abs() > threshold_std * std_dev).collect()
    }

    /// Compare two time periods
    pub fn compare_periods(
        &self,
        series_name: &str,
        period1_start: DateTime<Utc>,
        period1_end: DateTime<Utc>,
        period2_start: DateTime<Utc>,
        period2_end: DateTime<Utc>,
    ) -> Result<PeriodComparison> {
        let period1_points = self.store.query_range(series_name, period1_start, period1_end);
        let period2_points = self.store.query_range(series_name, period2_start, period2_end);

        if period1_points.is_empty() || period2_points.is_empty() {
            return Err(MetricsError::Analysis("Insufficient data for comparison".to_string()));
        }

        let period1_avg =
            period1_points.iter().map(|p| p.value).sum::<f64>() / period1_points.len() as f64;
        let period2_avg =
            period2_points.iter().map(|p| p.value).sum::<f64>() / period2_points.len() as f64;

        let change = period2_avg - period1_avg;
        let change_percent = if period1_avg != 0.0 { (change / period1_avg) * 100.0 } else { 0.0 };

        Ok(PeriodComparison {
            period1_average: period1_avg,
            period2_average: period2_avg,
            absolute_change: change,
            percent_change: change_percent,
            improvement: change > 0.0,
        })
    }

    /// Find correlations between metrics
    pub fn find_correlation(&self, series1: &str, series2: &str, window: Duration) -> Result<f64> {
        let end = Utc::now();
        let start = end - window;

        let points1 = self.store.query_range(series1, start, end);
        let points2 = self.store.query_range(series2, start, end);

        if points1.len() < 2 || points2.len() < 2 {
            return Err(MetricsError::Analysis("Insufficient data for correlation".to_string()));
        }

        // Align timestamps and extract values
        let mut values1 = Vec::new();
        let mut values2 = Vec::new();

        for p1 in &points1 {
            // Find closest point in series2
            if let Some(p2) =
                points2.iter().min_by_key(|p| (p.timestamp - p1.timestamp).num_seconds().abs())
            {
                // Only use if timestamps are within 1 minute
                if (p2.timestamp - p1.timestamp).num_seconds().abs() < 60 {
                    values1.push(p1.value);
                    values2.push(p2.value);
                }
            }
        }

        if values1.len() < 2 {
            return Err(MetricsError::Analysis(
                "No aligned data points for correlation".to_string(),
            ));
        }

        Ok(self.pearson_correlation(&values1, &values2))
    }

    /// Calculate Pearson correlation coefficient
    fn pearson_correlation(&self, x: &[f64], y: &[f64]) -> f64 {
        let n = x.len() as f64;

        let sum_x: f64 = x.iter().sum();
        let sum_y: f64 = y.iter().sum();
        let sum_xy: f64 = x.iter().zip(y.iter()).map(|(a, b)| a * b).sum();
        let sum_x2: f64 = x.iter().map(|a| a * a).sum();
        let sum_y2: f64 = y.iter().map(|b| b * b).sum();

        let numerator = n * sum_xy - sum_x * sum_y;
        let denominator = ((n * sum_x2 - sum_x * sum_x) * (n * sum_y2 - sum_y * sum_y)).sqrt();

        if denominator == 0.0 { 0.0 } else { numerator / denominator }
    }
}

/// Comparison between two time periods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodComparison {
    pub period1_average: f64,
    pub period2_average: f64,
    pub absolute_change: f64,
    pub percent_change: f64,
    pub improvement: bool,
}
