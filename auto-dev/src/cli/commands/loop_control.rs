//! Loop control commands for managing the autonomous development loop

use anyhow::Result;
use auto_dev_core::dev_loop::control_server::{ControlClient, ControlServer};
use auto_dev_core::dev_loop::{Event, EventType, LoopConfig, Orchestrator};
use clap::{Args, Subcommand};
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Loop control commands
#[derive(Debug, Args)]
pub struct LoopCommand {
    #[command(subcommand)]
    pub subcommand: LoopSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum LoopSubcommand {
    /// Start the autonomous development loop
    Start {
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Run in background
        #[arg(short, long)]
        background: bool,
    },

    /// Stop the running loop
    Stop,

    /// Show loop status
    Status,

    /// Show loop metrics
    Metrics,

    /// Trigger a manual event
    Trigger {
        /// Event type to trigger
        #[arg(value_enum)]
        event_type: EventTypeArg,

        /// Source file for the event
        #[arg(short, long)]
        source: PathBuf,
    },

    /// Run a stress test
    StressTest {
        /// Number of events to generate
        #[arg(short, long, default_value = "100")]
        events: usize,

        /// Duration in seconds
        #[arg(short, long, default_value = "60")]
        duration: u64,
    },
}

/// Event type argument for CLI
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum EventTypeArg {
    SpecChange,
    TestAdded,
    TestFailed,
    CodeModified,
    DependencyUpdated,
    ConfigChanged,
}

impl From<EventTypeArg> for EventType {
    fn from(arg: EventTypeArg) -> Self {
        match arg {
            EventTypeArg::SpecChange => EventType::SpecificationChanged,
            EventTypeArg::TestAdded => EventType::TestAdded,
            EventTypeArg::TestFailed => EventType::TestFailed,
            EventTypeArg::CodeModified => EventType::CodeModified,
            EventTypeArg::DependencyUpdated => EventType::DependencyUpdated,
            EventTypeArg::ConfigChanged => EventType::ConfigurationChanged,
        }
    }
}

/// Execute loop command
pub async fn execute(command: LoopCommand, target_self: bool) -> Result<()> {
    match command.subcommand {
        LoopSubcommand::Start { config, background } => {
            start_loop(config, background, target_self, 9090).await
        }
        LoopSubcommand::Stop => stop_loop().await,
        LoopSubcommand::Status => show_status().await,
        LoopSubcommand::Metrics => show_metrics().await,
        LoopSubcommand::Trigger { event_type, source } => trigger_event(event_type, source).await,
        LoopSubcommand::StressTest { events, duration } => run_stress_test(events, duration).await,
    }
}

/// Start the development loop
async fn start_loop(
    config_path: Option<PathBuf>,
    background: bool,
    target_self: bool,
    port: u16,
) -> Result<()> {
    info!("Starting autonomous development loop on port {}", port);

    // Check if already running (use temp directory for port file if no .auto-dev)
    let mut client = ControlClient::new();
    if client.ping().await.unwrap_or(false) {
        warn!("Loop is already running");
        return Ok(());
    }

    // Load configuration - use defaults if no config provided
    let mut config =
        if let Some(path) = config_path { load_config(path).await? } else { LoopConfig::default() };

    // Apply self-targeting if requested
    if target_self {
        info!("Configuring for self-targeting mode");
        config.self_targeting = Some(true);

        // Load self-targeting configuration
        let self_config = auto_dev_core::self_target::SelfTargetConfig::load_or_create()?;
        info!("Targeting project: {} v{}", self_config.project.name, self_config.project.version);
    }

    if background {
        // Start in background
        tokio::spawn(async move {
            if let Err(e) = run_loop_internal(config, port).await {
                error!("Loop error: {}", e);
            }
        });

        info!("Loop started in background");
    } else {
        // Run in foreground
        run_loop_internal(config, port).await?;
    }

    Ok(())
}

/// Internal loop runner with control server
async fn run_loop_internal(config: LoopConfig, port: u16) -> Result<()> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let (command_tx, _command_rx) = mpsc::channel(100);

    // Start control server
    let control_shutdown_tx = shutdown_tx.clone();
    let control_server = ControlServer::new(port, control_shutdown_tx, command_tx);

    tokio::spawn(async move {
        if let Err(e) = control_server.start().await {
            error!("Control server error: {}", e);
        }
    });

    // Set up signal handler for graceful shutdown
    let signal_shutdown_tx = shutdown_tx.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        signal_shutdown_tx.send(()).await.ok();
    });

    // Start orchestrator
    let orchestrator = Orchestrator::new(config, shutdown_rx);
    orchestrator.run().await
}

/// Stop the running loop via IPC
async fn stop_loop() -> Result<()> {
    info!("Sending shutdown command to development loop");

    let mut client = ControlClient::new();

    // Check if running
    if !client.ping().await.unwrap_or(false) {
        warn!("Loop is not running");
        return Ok(());
    }

    // Send shutdown command
    client.shutdown().await?;

    info!("Shutdown command sent successfully");

    // Clean up port file
    let port_file = PathBuf::from(".auto-dev/loop/control.port");
    if port_file.exists() {
        tokio::fs::remove_file(port_file).await.ok();
    }

    Ok(())
}

/// Show loop status
async fn show_status() -> Result<()> {
    // Try to load state from persistent storage if available
    let state_path = PathBuf::from(".auto-dev/loop/state.json");

    if state_path.exists() {
        let content = tokio::fs::read_to_string(state_path).await?;
        let state: serde_json::Value = serde_json::from_str(&content)?;

        println!("Loop Status:");
        println!("{:#}", state);
    } else {
        println!("No loop state found. Loop may not be running.");
    }

    Ok(())
}

/// Show loop metrics
async fn show_metrics() -> Result<()> {
    // Load metrics from .auto-dev/loop/metrics.json
    let metrics_path = PathBuf::from(".auto-dev/loop/metrics.json");

    if metrics_path.exists() {
        let content = tokio::fs::read_to_string(metrics_path).await?;
        let metrics: serde_json::Value = serde_json::from_str(&content)?;

        println!("Loop Metrics:");
        println!("═══════════════════════════════════════");

        if let Some(obj) = metrics.as_object() {
            for (key, value) in obj {
                let formatted_key = key
                    .replace('_', " ")
                    .split_whitespace()
                    .map(|word| {
                        let mut chars = word.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(first) => {
                                first.to_uppercase().collect::<String>() + chars.as_str()
                            }
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");

                println!("{:<30} {}", formatted_key + ":", value);
            }
        }

        println!("═══════════════════════════════════════");
    } else {
        println!("No metrics found. Loop may not have been run yet.");
    }

    Ok(())
}

/// Trigger a manual event
async fn trigger_event(event_type: EventTypeArg, source: PathBuf) -> Result<()> {
    info!("Triggering event: {:?} for {:?}", event_type, source);

    let event = Event::new(event_type.into(), source);

    // This would send the event to the running loop
    // For now, we'll save it to a queue file
    let queue_path = PathBuf::from(".auto-dev/loop/event_queue.json");

    // Create directory if it doesn't exist
    if let Some(parent) = queue_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Load existing queue or create new
    let mut events: Vec<Event> = if queue_path.exists() {
        let content = tokio::fs::read_to_string(&queue_path).await?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    events.push(event);

    // Save updated queue
    let content = serde_json::to_string_pretty(&events)?;
    tokio::fs::write(queue_path, content).await?;

    info!("Event queued successfully");
    Ok(())
}

/// Run a stress test
async fn run_stress_test(num_events: usize, duration: u64) -> Result<()> {
    info!("Running stress test: {} events over {} seconds", num_events, duration);

    let config = LoopConfig::default();
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

    // Start orchestrator
    let orchestrator = Orchestrator::new(config, shutdown_rx);

    // Generate test events
    let event_types = vec![
        EventType::SpecificationChanged,
        EventType::TestAdded,
        EventType::TestFailed,
        EventType::CodeModified,
    ];

    let interval = std::time::Duration::from_millis((duration * 1000) / num_events as u64);

    for i in 0..num_events {
        let event_type = event_types[i % event_types.len()].clone();
        let source = PathBuf::from(format!("test_file_{}.rs", i));
        let event = Event::new(event_type, source);

        orchestrator.queue_event(event).await?;
        tokio::time::sleep(interval).await;
    }

    // Wait for processing
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // Get metrics
    let metrics = orchestrator.get_metrics().await;

    println!("\nStress Test Results:");
    println!("═══════════════════════════════════════");
    println!("Events Queued:         {}", num_events);
    println!("Events Processed:      {}", metrics.events_processed);
    println!("LLM Calls Made:        {}", metrics.llm_calls_made);
    println!("LLM Calls Avoided:     {}", metrics.llm_calls_avoided);
    println!("Implementations:       {}", metrics.implementations_completed);
    println!("Tests Generated:       {}", metrics.tests_generated);
    println!("Errors Encountered:    {}", metrics.errors_encountered);
    println!("═══════════════════════════════════════");

    // Shutdown
    shutdown_tx.send(()).await.ok();

    Ok(())
}

/// Load configuration from file
async fn load_config(path: PathBuf) -> Result<LoopConfig> {
    let content = tokio::fs::read_to_string(path).await?;
    let config: LoopConfig = toml::from_str(&content)?;
    Ok(config)
}

use serde_json;
use toml;

/// Get or create a path under .auto-dev if it exists, otherwise use temp directory
pub async fn get_storage_path(subdir: &str) -> PathBuf {
    let auto_dev_dir = PathBuf::from(".auto-dev");

    // If .auto-dev exists, use it (and create subdir if needed)
    if auto_dev_dir.exists() {
        let path = auto_dev_dir.join(subdir);
        if !path.exists() {
            if let Ok(_) = tokio::fs::create_dir_all(&path).await {
                info!("Created .auto-dev/{} for persistent storage", subdir);
            }
        }
        return path;
    }

    // Otherwise use system temp directory (no persistence)
    std::env::temp_dir().join("auto-dev").join(subdir)
}

/// Run with default configuration (alias for 'loop start --background')
pub async fn run_default(target_self: bool) -> Result<()> {
    // Check for optional config file (but don't require it)
    let config_path = PathBuf::from(".auto-dev/config.toml");
    let config = if config_path.exists() {
        info!("Using configuration from .auto-dev/config.toml");
        Some(config_path)
    } else {
        info!("Running with default configuration (no .auto-dev directory needed)");
        None
    };

    // Always start in background with default port
    start_loop(config, true, target_self, 9090).await
}

/// Initialize auto-dev project structure (completely optional - creates config template)
pub async fn init_project() -> Result<()> {
    info!("Creating optional .auto-dev configuration directory");

    let auto_dev_dir = PathBuf::from(".auto-dev");

    // Create the directory structure for users who want persistent config
    if !auto_dev_dir.exists() {
        tokio::fs::create_dir_all(&auto_dev_dir).await?;
        println!("Created .auto-dev directory for persistent configuration");
    }

    // Create subdirectories that will be used if they exist
    let dirs = ["cache", "history", "metrics", "patterns", "templates"];

    for subdir in &dirs {
        let path = auto_dev_dir.join(subdir);
        if !path.exists() {
            tokio::fs::create_dir_all(&path).await?;
            println!(
                "Created .auto-dev/{} for {}",
                subdir,
                match subdir {
                    &"cache" => "caching analysis results",
                    &"history" => "persisting command history",
                    &"metrics" => "tracking performance metrics",
                    &"patterns" => "custom code patterns",
                    &"templates" => "project templates",
                    _ => "data storage",
                }
            );
        }
    }

    // Create config file with helpful defaults
    let config_path = auto_dev_dir.join("config.toml");
    if !config_path.exists() {
        // Create a minimal default config
        let default_config = r#"# Auto-dev Configuration
# This file is optional - auto-dev works with sane defaults

[monitoring]
# Patterns to watch
include = ["src/**/*", "*.toml", "*.md"]
exclude = ["target/**", ".git/**", "node_modules/**"]

[behavior]
# Autonomous behavior settings
auto_fix_tests = false
auto_generate_tests = false
auto_update_docs = false

[llm]
# LLM provider settings (optional)
# provider = "local"  # or "openai", "claude", etc.
"#;
        tokio::fs::write(&config_path, default_config).await?;
        println!("Created .auto-dev/config.toml with example configuration");
    } else {
        println!("Config file already exists at .auto-dev/config.toml");
    }

    println!("\n✅ Auto-dev initialized with optional configuration!");
    println!("\nNote: This step is optional. Auto-dev works with sane defaults.");
    println!("\nYou can now:");
    println!("  • Run 'auto-dev run' or 'auto-dev start' to begin");
    println!("  • Customize .auto-dev/config.toml if desired");
    println!("  • Use 'auto-dev loop status' to check if running");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_conversion() {
        let arg = EventTypeArg::SpecChange;
        let event_type: EventType = arg.into();
        assert_eq!(event_type, EventType::SpecificationChanged);
    }
}
