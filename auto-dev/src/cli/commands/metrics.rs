#![allow(unused)]
//! Metrics command for viewing self-improvement metrics

use anyhow::Result;
use auto_dev_core::metrics::{
    DashboardConfig, ExportFormat, MetricEvent, MetricsCollector, MetricsDashboard,
    MetricsExporter, TimeSeriesStore, TrendAnalyzer, calculate_improvement, get_snapshot,
    initialize,
};
use chrono::{Duration, Utc};
use clap::{Args, Subcommand};
use std::collections::HashMap;
use std::path::Path;
use tokio::time;
use tracing::{error, info};

/// View and analyze self-improvement metrics
#[derive(Debug, Args)]
pub struct MetricsCommand {
    #[command(subcommand)]
    pub subcommand: Option<MetricsSubcommand>,
}

#[derive(Debug, Subcommand)]
pub enum MetricsSubcommand {
    /// Display metrics dashboard
    Dashboard {
        /// Auto-refresh interval in seconds
        #[arg(long, default_value = "5")]
        refresh: u64,

        /// Show historical events
        #[arg(long)]
        history: bool,
    },

    /// Export metrics data
    Export {
        /// Output format (json, csv, markdown, html)
        #[arg(long, default_value = "json")]
        format: String,

        /// Output file path
        #[arg(long, default_value = "metrics_export")]
        output: String,

        /// Number of days of data to export
        #[arg(long, default_value = "30")]
        days: i64,
    },

    /// Analyze trends
    Trends {
        /// Number of days to analyze
        #[arg(long, default_value = "30")]
        days: i64,

        /// Specific metric to analyze
        #[arg(long)]
        metric: Option<String>,
    },

    /// Show improvement score
    Score {
        /// Show detailed breakdown
        #[arg(long)]
        detailed: bool,
    },

    /// Record a metric event
    Record {
        /// Event type
        #[arg(long)]
        event_type: String,

        /// Module name
        #[arg(long, default_value = "manual")]
        module: String,

        /// Whether the event was successful
        #[arg(long)]
        success: bool,

        /// Duration in milliseconds
        #[arg(long, default_value = "0")]
        duration: u64,
    },
}

pub async fn handle_metrics_command(command: MetricsCommand) -> Result<()> {
    match command.subcommand {
        Some(MetricsSubcommand::Dashboard { refresh, history }) => {
            show_dashboard(refresh, history).await
        }
        Some(MetricsSubcommand::Export { format, output, days }) => {
            export_metrics(&format, &output, days).await
        }
        Some(MetricsSubcommand::Trends { days, metric }) => {
            analyze_trends(days, metric.as_deref()).await
        }
        Some(MetricsSubcommand::Score { detailed }) => show_improvement_score(detailed).await,
        Some(MetricsSubcommand::Record { event_type, module, success, duration }) => {
            record_event(&event_type, &module, success, duration).await
        }
        None => {
            // Default to showing dashboard
            show_dashboard(5, true).await
        }
    }
}

async fn show_dashboard(refresh_seconds: u64, show_history: bool) -> Result<()> {
    info!("Starting metrics dashboard");

    let config = DashboardConfig {
        refresh_interval_seconds: refresh_seconds,
        show_historical: show_history,
        max_history_items: 5,
    };

    let dashboard = MetricsDashboard::new(config.clone());
    let mut interval = time::interval(time::Duration::from_secs(refresh_seconds));

    println!("Loading metrics dashboard...");

    loop {
        interval.tick().await;

        match get_snapshot().await {
            Ok(snapshot) => {
                match calculate_improvement().await {
                    Ok(score) => {
                        // Get recent events (would come from collector in real implementation)
                        let recent_events = vec![
                            "Added metrics module".to_string(),
                            "Optimized performance by 10%".to_string(),
                            "Learned new pattern: Factory".to_string(),
                        ];

                        if let Err(e) = dashboard.render(&snapshot, &score, &recent_events) {
                            error!("Failed to render dashboard: {}", e);
                        }
                    }
                    Err(e) => {
                        println!("Error calculating improvement score: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("Error getting metrics snapshot: {}", e);
                println!("Retrying in {} seconds...", refresh_seconds);
            }
        }

        // Check for Ctrl+C
        if tokio::signal::ctrl_c().await.is_ok() {
            println!("\nExiting dashboard...");
            break;
        }
    }

    Ok(())
}

async fn export_metrics(format: &str, output: &str, days: i64) -> Result<()> {
    info!("Exporting metrics");

    let collector = initialize().await?;
    let end = Utc::now();
    let start = end - Duration::days(days);

    let snapshots = collector.export_range(start, end).await?;

    if snapshots.is_empty() {
        println!("No metrics data found for the specified time range");
        return Ok(());
    }

    let export_format = match format.to_lowercase().as_str() {
        "json" => ExportFormat::Json,
        "csv" => ExportFormat::Csv,
        "markdown" | "md" => ExportFormat::Markdown,
        "html" => ExportFormat::Html,
        _ => {
            println!("Invalid format '{}'. Using JSON.", format);
            ExportFormat::Json
        }
    };

    let extension = match export_format {
        ExportFormat::Json => "json",
        ExportFormat::Csv => "csv",
        ExportFormat::Markdown => "md",
        ExportFormat::Html => "html",
    };

    let output_path =
        if output.contains('.') { output.to_string() } else { format!("{}.{}", output, extension) };

    MetricsExporter::export_to_file(&snapshots, export_format, &output_path).await?;

    println!("✅ Exported {} snapshots to {}", snapshots.len(), output_path);
    Ok(())
}

async fn analyze_trends(days: i64, metric: Option<&str>) -> Result<()> {
    info!("Analyzing trends");

    let storage_path = Path::new(".auto-dev/metrics/timeseries");
    let store = TimeSeriesStore::new(storage_path)?;
    let analyzer = TrendAnalyzer::new(store);

    let window = Duration::days(days);

    let series_to_analyze = if let Some(m) = metric {
        vec![m.to_string()]
    } else {
        // Analyze key metrics
        vec![
            "development/success_rate".to_string(),
            "quality/complexity".to_string(),
            "performance/compilation_time".to_string(),
            "capability/features_added".to_string(),
        ]
    };

    println!("Trend Analysis Report");
    println!("=====================");
    println!("Period: Last {} days\n", days);

    for series in &series_to_analyze {
        match analyzer.analyze_series(&series, window) {
            Ok(analysis) => {
                println!("Metric: {}", series);
                println!("  Direction: {:?}", analysis.direction);
                println!("  Slope: {:.4}", analysis.slope);
                println!("  Change: {:+.1}%", analysis.change_percent);
                println!("  Confidence: {:.1}%", analysis.confidence * 100.0);

                if let Some(pred) = analysis.prediction {
                    println!("  Next predicted: {:.2}", pred);
                }

                println!();
            }
            Err(e) => {
                println!("Metric: {} - Error: {}\n", series, e);
            }
        }
    }

    // Detect anomalies
    println!("Anomaly Detection");
    println!("-----------------");

    for series in &series_to_analyze {
        let anomalies = analyzer.detect_anomalies(series, window, 2.0);
        if !anomalies.is_empty() {
            println!("{}: {} anomalies detected", series, anomalies.len());
            for anomaly in anomalies.iter().take(3) {
                println!(
                    "  - {} : {:.2}",
                    anomaly.timestamp.format("%Y-%m-%d %H:%M"),
                    anomaly.value
                );
            }
        }
    }

    Ok(())
}

async fn show_improvement_score(detailed: bool) -> Result<()> {
    info!("Calculating improvement score");

    match calculate_improvement().await {
        Ok(score) => {
            println!("Self-Improvement Score");
            println!("======================\n");

            let overall_emoji = if score.overall_score > 0.05 {
                "📈"
            } else if score.overall_score < -0.05 {
                "📉"
            } else {
                "➡️"
            };

            println!("Overall Score: {:+.1} {}", score.overall_score * 10.0, overall_emoji);
            println!("Trend: {:?}", score.trend);
            println!("Confidence: {:.0}%\n", score.confidence * 100.0);

            if detailed {
                println!("Component Scores:");
                println!("  Development: {:+.1}", score.development_score * 10.0);
                println!("  Quality: {:+.1}", score.quality_score * 10.0);
                println!("  Performance: {:+.1}", score.performance_score * 10.0);
                println!("  Capability: {:+.1}", score.capability_score * 10.0);

                println!("\nInterpretation:");
                if score.overall_score > 0.1 {
                    println!("  🎉 Excellent improvement! Keep up the great work.");
                } else if score.overall_score > 0.05 {
                    println!("  ✅ Good progress. Steady improvement observed.");
                } else if score.overall_score > -0.05 {
                    println!("  ⚖️ Stable performance. Consider new optimization strategies.");
                } else {
                    println!("  ⚠️ Performance declining. Review recent changes.");
                }
            }
        }
        Err(e) => {
            println!("❌ Error calculating improvement score: {}", e);
            println!("\nThis may happen if there's insufficient historical data.");
            println!("Try recording more metric events first.");
        }
    }

    Ok(())
}

async fn record_event(
    event_type: &str,
    module: &str,
    success: bool,
    duration_ms: u64,
) -> Result<()> {
    info!("Recording metric event");

    let collector = initialize().await?;

    let event = MetricEvent {
        timestamp: Utc::now(),
        event_type: event_type.to_string(),
        module: module.to_string(),
        success,
        duration_ms,
        metadata: HashMap::new(),
    };

    collector.record_event(event).await?;

    println!(
        "✅ Recorded event: {} ({}) - {}",
        event_type,
        module,
        if success { "success" } else { "failure" }
    );

    Ok(())
}
