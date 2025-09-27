//! Export metrics in various formats

use super::{MetricsError, MetricsSnapshot, Result};
use chrono::{DateTime, Utc};
use serde_json;
use std::path::Path;
use tracing::{debug, info};

/// Supported export formats
#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    Csv,
    Markdown,
    Html,
}

/// Exports metrics data
pub struct MetricsExporter;

impl MetricsExporter {
    /// Export snapshots to a file
    pub async fn export_to_file(
        snapshots: &[MetricsSnapshot],
        format: ExportFormat,
        output_path: impl AsRef<Path>,
    ) -> Result<()> {
        let content = match format {
            ExportFormat::Json => Self::export_json(snapshots)?,
            ExportFormat::Csv => Self::export_csv(snapshots)?,
            ExportFormat::Markdown => Self::export_markdown(snapshots)?,
            ExportFormat::Html => Self::export_html(snapshots)?,
        };

        tokio::fs::write(output_path.as_ref(), content).await?;

        info!("Exported {} snapshots to {:?}", snapshots.len(), output_path.as_ref());
        Ok(())
    }

    /// Export as JSON
    fn export_json(snapshots: &[MetricsSnapshot]) -> Result<String> {
        serde_json::to_string_pretty(snapshots)
            .map_err(|e| MetricsError::Export(format!("JSON export failed: {}", e)))
    }

    /// Export as CSV
    fn export_csv(snapshots: &[MetricsSnapshot]) -> Result<String> {
        let mut csv = String::new();

        // Header
        csv.push_str("timestamp,modifications_per_day,success_rate,implementation_time_ms,");
        csv.push_str("test_coverage,complexity,doc_coverage,lint_warnings,lint_errors,");
        csv.push_str("compile_time_ms,test_time_ms,binary_size,memory_mb,");
        csv.push_str("features_added,apis_created,modules_loaded,patterns_learned\n");

        // Data rows
        for snapshot in snapshots {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                snapshot.timestamp.to_rfc3339(),
                snapshot.development.modifications_per_day,
                snapshot.development.success_rate,
                snapshot.development.implementation_time_avg_ms,
                snapshot.development.test_coverage_percent,
                snapshot.quality.cyclomatic_complexity,
                snapshot.quality.documentation_coverage,
                snapshot.quality.lint_warnings,
                snapshot.quality.lint_errors,
                snapshot.performance.compilation_time_ms,
                snapshot.performance.test_execution_time_ms,
                snapshot.performance.binary_size_bytes,
                snapshot.performance.memory_usage_mb,
                snapshot.capability.features_added,
                snapshot.capability.apis_created,
                snapshot.capability.modules_loaded,
                snapshot.capability.patterns_learned,
            ));
        }

        Ok(csv)
    }

    /// Export as Markdown
    fn export_markdown(snapshots: &[MetricsSnapshot]) -> Result<String> {
        let mut md = String::new();

        md.push_str("# Self-Improvement Metrics Report\n\n");
        md.push_str(&format!("Generated: {}\n\n", Utc::now().to_rfc3339()));

        if let Some(latest) = snapshots.last() {
            md.push_str("## Latest Metrics\n\n");
            md.push_str(&format!("**Timestamp**: {}\n\n", latest.timestamp.to_rfc3339()));

            md.push_str("### Development Metrics\n");
            md.push_str(&format!(
                "- Modifications per day: {:.2}\n",
                latest.development.modifications_per_day
            ));
            md.push_str(&format!(
                "- Success rate: {:.1}%\n",
                latest.development.success_rate * 100.0
            ));
            md.push_str(&format!(
                "- Avg implementation time: {}ms\n",
                latest.development.implementation_time_avg_ms
            ));
            md.push_str(&format!(
                "- Test coverage: {:.1}%\n",
                latest.development.test_coverage_percent
            ));
            md.push_str(&format!(
                "- Rollback frequency: {:.1}%\n\n",
                latest.development.rollback_frequency * 100.0
            ));

            md.push_str("### Quality Metrics\n");
            md.push_str(&format!(
                "- Cyclomatic complexity: {:.1}\n",
                latest.quality.cyclomatic_complexity
            ));
            md.push_str(&format!(
                "- Documentation coverage: {:.1}%\n",
                latest.quality.documentation_coverage
            ));
            md.push_str(&format!("- Lint warnings: {}\n", latest.quality.lint_warnings));
            md.push_str(&format!("- Lint errors: {}\n", latest.quality.lint_errors));
            md.push_str(&format!(
                "- Duplicate code: {:.1}%\n",
                latest.quality.duplicate_code_percent
            ));
            md.push_str(&format!(
                "- Technical debt score: {:.1}\n\n",
                latest.quality.technical_debt_score
            ));

            md.push_str("### Performance Metrics\n");
            md.push_str(&format!(
                "- Compilation time: {}ms\n",
                latest.performance.compilation_time_ms
            ));
            md.push_str(&format!(
                "- Test execution time: {}ms\n",
                latest.performance.test_execution_time_ms
            ));
            md.push_str(&format!(
                "- Binary size: {} MB\n",
                latest.performance.binary_size_bytes / 1_000_000
            ));
            md.push_str(&format!("- Memory usage: {:.1} MB\n", latest.performance.memory_usage_mb));
            md.push_str(&format!(
                "- Module load time: {}ms\n\n",
                latest.performance.module_load_time_ms
            ));

            md.push_str("### Capability Metrics\n");
            md.push_str(&format!("- Features added: {}\n", latest.capability.features_added));
            md.push_str(&format!("- APIs created: {}\n", latest.capability.apis_created));
            md.push_str(&format!("- Modules loaded: {}\n", latest.capability.modules_loaded));
            md.push_str(&format!("- Patterns learned: {}\n", latest.capability.patterns_learned));
            md.push_str(&format!("- LLM calls saved: {}\n\n", latest.capability.llm_calls_saved));
        }

        // Summary table
        if snapshots.len() > 1 {
            md.push_str("## Historical Summary\n\n");
            md.push_str("| Date | Success Rate | Complexity | Test Coverage | Features |\n");
            md.push_str("|------|-------------|------------|---------------|----------|\n");

            for snapshot in snapshots.iter().rev().take(10) {
                md.push_str(&format!(
                    "| {} | {:.1}% | {:.1} | {:.1}% | {} |\n",
                    snapshot.timestamp.format("%Y-%m-%d"),
                    snapshot.development.success_rate * 100.0,
                    snapshot.quality.cyclomatic_complexity,
                    snapshot.development.test_coverage_percent,
                    snapshot.capability.features_added,
                ));
            }
        }

        Ok(md)
    }

    /// Export as HTML
    fn export_html(snapshots: &[MetricsSnapshot]) -> Result<String> {
        let mut html = String::new();

        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str("<title>Self-Improvement Metrics</title>\n");
        html.push_str("<style>\n");
        html.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
        html.push_str("h1, h2 { color: #333; }\n");
        html.push_str(".metric { margin: 10px 0; }\n");
        html.push_str(".metric-label { font-weight: bold; }\n");
        html.push_str(".metric-value { color: #007bff; }\n");
        html.push_str("table { border-collapse: collapse; width: 100%; }\n");
        html.push_str("th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n");
        html.push_str("th { background-color: #f2f2f2; }\n");
        html.push_str("</style>\n</head>\n<body>\n");

        html.push_str("<h1>Self-Improvement Metrics Report</h1>\n");
        html.push_str(&format!("<p>Generated: {}</p>\n", Utc::now().to_rfc3339()));

        if let Some(latest) = snapshots.last() {
            html.push_str("<h2>Latest Metrics</h2>\n");
            html.push_str(&format!(
                "<p><strong>Timestamp:</strong> {}</p>\n",
                latest.timestamp.to_rfc3339()
            ));

            // Development metrics
            html.push_str("<h3>Development Metrics</h3>\n");
            html.push_str("<div class='metric'><span class='metric-label'>Success Rate:</span> ");
            html.push_str(&format!(
                "<span class='metric-value'>{:.1}%</span></div>\n",
                latest.development.success_rate * 100.0
            ));
            html.push_str(
                "<div class='metric'><span class='metric-label'>Modifications/Day:</span> ",
            );
            html.push_str(&format!(
                "<span class='metric-value'>{:.2}</span></div>\n",
                latest.development.modifications_per_day
            ));

            // Quality metrics
            html.push_str("<h3>Quality Metrics</h3>\n");
            html.push_str("<div class='metric'><span class='metric-label'>Complexity:</span> ");
            html.push_str(&format!(
                "<span class='metric-value'>{:.1}</span></div>\n",
                latest.quality.cyclomatic_complexity
            ));
            html.push_str("<div class='metric'><span class='metric-label'>Documentation:</span> ");
            html.push_str(&format!(
                "<span class='metric-value'>{:.1}%</span></div>\n",
                latest.quality.documentation_coverage
            ));
        }

        // Historical table
        if snapshots.len() > 1 {
            html.push_str("<h2>Historical Data</h2>\n");
            html.push_str("<table>\n<tr><th>Date</th><th>Success Rate</th><th>Complexity</th><th>Features Added</th></tr>\n");

            for snapshot in snapshots.iter().rev().take(10) {
                html.push_str(&format!(
                    "<tr><td>{}</td><td>{:.1}%</td><td>{:.1}</td><td>{}</td></tr>\n",
                    snapshot.timestamp.format("%Y-%m-%d %H:%M"),
                    snapshot.development.success_rate * 100.0,
                    snapshot.quality.cyclomatic_complexity,
                    snapshot.capability.features_added,
                ));
            }

            html.push_str("</table>\n");
        }

        html.push_str("</body>\n</html>");

        Ok(html)
    }
}
