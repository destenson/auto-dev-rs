//! Terminal dashboard for metrics visualization

use super::{
    MetricsSnapshot, ImprovementScore, TrendDirection,
    MetricsError, Result
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use tracing::{debug, info};

/// Dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub refresh_interval_seconds: u64,
    pub show_historical: bool,
    pub max_history_items: usize,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            refresh_interval_seconds: 5,
            show_historical: true,
            max_history_items: 5,
        }
    }
}

/// Terminal metrics dashboard
pub struct MetricsDashboard {
    config: DashboardConfig,
}

impl MetricsDashboard {
    pub fn new(config: DashboardConfig) -> Self {
        Self { config }
    }
    
    /// Render the dashboard to terminal
    pub fn render(
        &self,
        snapshot: &MetricsSnapshot,
        score: &ImprovementScore,
        recent_events: &[String],
    ) -> Result<()> {
        // Clear screen (platform-specific)
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd")
                .args(&["/C", "cls"])
                .status();
        }
        #[cfg(not(target_os = "windows"))]
        {
            print!("\x1B[2J\x1B[1;1H");
        }
        
        self.render_header()?;
        self.render_improvement_score(score)?;
        self.render_metrics(snapshot)?;
        
        if self.config.show_historical {
            self.render_recent_activity(recent_events)?;
        }
        
        self.render_footer()?;
        
        io::stdout().flush()?;
        Ok(())
    }
    
    fn render_header(&self) -> Result<()> {
        println!("╭─ Self-Improvement Metrics ─────────────────────────╮");
        Ok(())
    }
    
    fn render_improvement_score(&self, score: &ImprovementScore) -> Result<()> {
        let trend_symbol = match score.trend {
            TrendDirection::Improving => "↑",
            TrendDirection::Declining => "↓",
            TrendDirection::Stable => "→",
            TrendDirection::Unknown => "?",
        };
        
        let overall_color = if score.overall_score > 0.05 {
            "\x1b[32m" // Green
        } else if score.overall_score < -0.05 {
            "\x1b[31m" // Red
        } else {
            "\x1b[33m" // Yellow
        };
        
        println!("│                                                     │");
        println!("│ Overall Score: {}{:+.1} {} \x1b[0m                           │",
                 overall_color, score.overall_score * 10.0, trend_symbol);
        println!("│ Confidence: {:.0}%                                   │",
                 score.confidence * 100.0);
        println!("│                                                     │");
        println!("├─────────────────────────────────────────────────────┤");
        
        Ok(())
    }
    
    fn render_metrics(&self, snapshot: &MetricsSnapshot) -> Result<()> {
        // Development metrics
        println!("│ Development                                         │");
        println!("│   Success Rate: {:.1}%                              │",
                 snapshot.development.success_rate * 100.0);
        println!("│   Velocity: {:.1} mods/day                          │",
                 snapshot.development.modifications_per_day);
        println!("│   Test Coverage: {:.1}%                             │",
                 snapshot.development.test_coverage_percent);
        
        println!("│                                                     │");
        
        // Quality metrics
        println!("│ Quality                                             │");
        println!("│   Complexity: {:.1}/10                              │",
                 snapshot.quality.cyclomatic_complexity);
        println!("│   Documentation: {:.1}%                             │",
                 snapshot.quality.documentation_coverage);
        println!("│   Warnings: {} | Errors: {}                        │",
                 snapshot.quality.lint_warnings, snapshot.quality.lint_errors);
        
        println!("│                                                     │");
        
        // Performance metrics
        println!("│ Performance                                         │");
        println!("│   Compile Time: {}ms                               │",
                 snapshot.performance.compilation_time_ms);
        println!("│   Memory Usage: {:.1} MB                           │",
                 snapshot.performance.memory_usage_mb);
        println!("│   Module Load: {}ms                                │",
                 snapshot.performance.module_load_time_ms);
        
        println!("│                                                     │");
        
        // Capability metrics
        println!("│ Capabilities                                        │");
        println!("│   Features: {} | APIs: {} | Modules: {}           │",
                 snapshot.capability.features_added,
                 snapshot.capability.apis_created,
                 snapshot.capability.modules_loaded);
        println!("│   Patterns Learned: {}                             │",
                 snapshot.capability.patterns_learned);
        
        Ok(())
    }
    
    fn render_recent_activity(&self, events: &[String]) -> Result<()> {
        println!("├─────────────────────────────────────────────────────┤");
        println!("│ Recent Activity                                     │");
        
        let display_events = events.iter()
            .rev()
            .take(self.config.max_history_items)
            .collect::<Vec<_>>();
        
        if display_events.is_empty() {
            println!("│   No recent activity                               │");
        } else {
            for event in display_events {
                // Truncate event to fit
                let truncated = if event.len() > 48 {
                    format!("{}...", &event[..45])
                } else {
                    format!("{:<48}", event)
                };
                println!("│   • {}│", truncated);
            }
        }
        
        Ok(())
    }
    
    fn render_footer(&self) -> Result<()> {
        println!("╰─────────────────────────────────────────────────────╯");
        println!("\nPress Ctrl+C to exit | Refreshes every {}s", 
                 self.config.refresh_interval_seconds);
        Ok(())
    }
    
    /// Create a compact single-line summary
    pub fn render_summary(snapshot: &MetricsSnapshot, score: &ImprovementScore) -> String {
        let trend = match score.trend {
            TrendDirection::Improving => "↑",
            TrendDirection::Declining => "↓",
            TrendDirection::Stable => "→",
            TrendDirection::Unknown => "?",
        };
        
        format!(
            "Score: {:+.1}{} | Success: {:.0}% | Quality: {:.1}/10 | Features: {}",
            score.overall_score * 10.0,
            trend,
            snapshot.development.success_rate * 100.0,
            10.0 - snapshot.quality.cyclomatic_complexity.min(10.0),
            snapshot.capability.features_added
        )
    }
    
    /// Create a sparkline chart for a series of values
    pub fn sparkline(values: &[f64], width: usize) -> String {
        if values.is_empty() {
            return String::new();
        }
        
        let chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max - min;
        
        let step = values.len() / width.min(values.len());
        
        values.iter()
            .step_by(step.max(1))
            .take(width)
            .map(|v| {
                if range > 0.0 {
                    let normalized = (v - min) / range;
                    let index = ((normalized * 7.0).round() as usize).min(7);
                    chars[index]
                } else {
                    chars[4]
                }
            })
            .collect()
    }
}
