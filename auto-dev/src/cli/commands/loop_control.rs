//! Loop control commands for managing the autonomous development loop

use anyhow::Result;
use auto_dev_core::dev_loop::{Event, EventType, LoopConfig, Orchestrator};
use auto_dev_core::dev_loop::control_server::{ControlClient, ControlServer};
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
pub async fn execute(command: LoopCommand) -> Result<()> {
    match command.subcommand {
        LoopSubcommand::Start { config, background } => {
            start_loop(config, background, 9090).await
        },
        LoopSubcommand::Stop => {
            stop_loop().await
        },
        LoopSubcommand::Status => {
            show_status().await
        },
        LoopSubcommand::Metrics => {
            show_metrics().await
        },
        LoopSubcommand::Trigger { event_type, source } => {
            trigger_event(event_type, source).await
        },
        LoopSubcommand::StressTest { events, duration } => {
            run_stress_test(events, duration).await
        },
    }
}

/// Start the development loop
async fn start_loop(config_path: Option<PathBuf>, background: bool, port: u16) -> Result<()> {
    info!("Starting autonomous development loop on port {}", port);
    
    // Check if already running
    let mut client = ControlClient::new();
    if client.ping().await.unwrap_or(false) {
        warn!("Loop is already running");
        return Ok(());
    }
    
    // Load configuration
    let config = if let Some(path) = config_path {
        load_config(path).await?
    } else {
        LoopConfig::default()
    };
    
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
    // Load state from .auto-dev/loop/state.json
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
                let formatted_key = key.replace('_', " ")
                    .split_whitespace()
                    .map(|word| {
                        let mut chars = word.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
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

/// Run with default configuration (alias for 'loop start --background')
pub async fn run_default() -> Result<()> {
    // Check for config file
    let config_path = PathBuf::from(".auto-dev/config.toml");
    let config = if config_path.exists() {
        info!("Using configuration from .auto-dev/config.toml");
        Some(config_path)
    } else {
        info!("No config file found, using defaults");
        None
    };
    
    // Always start in background with default port
    start_loop(config, true, 9090).await
}

/// Initialize auto-dev project structure
pub async fn init_project() -> Result<()> {
    info!("Initializing auto-dev project structure");
    
    // Create .auto-dev directory
    let auto_dev_dir = PathBuf::from(".auto-dev");
    if !auto_dev_dir.exists() {
        tokio::fs::create_dir_all(&auto_dev_dir).await?;
        println!("Created .auto-dev directory");
    }
    
    // Create subdirectories
    let subdirs = [
        "loop",
        "patterns",
        "templates",
        "cache",
        "history",
        "metrics",
    ];
    
    for subdir in &subdirs {
        let path = auto_dev_dir.join(subdir);
        if !path.exists() {
            tokio::fs::create_dir_all(&path).await?;
            println!("Created .auto-dev/{}", subdir);
        }
    }
    
    // Create config file if it doesn't exist
    let config_path = auto_dev_dir.join("config.toml");
    if !config_path.exists() {
        // Embed the default configuration at compile time
        let default_config = include_str!("../../../../.auto-dev/config.toml.example");
        tokio::fs::write(&config_path, default_config).await?;
        println!("Created .auto-dev/config.toml with default configuration");
    } else {
        println!("Config file already exists at .auto-dev/config.toml");
    }
    
    // Create a .gitignore for .auto-dev if it doesn't exist
    let gitignore_path = auto_dev_dir.join(".gitignore");
    if !gitignore_path.exists() {
        let gitignore_content = "# Auto-generated files\n\
                                cache/\n\
                                *.tmp\n\
                                *.log\n\
                                control.port\n\
                                state.json\n";
        tokio::fs::write(&gitignore_path, gitignore_content).await?;
        println!("Created .auto-dev/.gitignore");
    }
    
    println!("\n✅ Auto-dev initialized successfully!");
    println!("\nNext steps:");
    println!("  1. Review and customize .auto-dev/config.toml");
    println!("  2. Run 'auto-dev start' to begin autonomous development");
    println!("  3. Use 'auto-dev status' to check the loop status");
    println!("  4. Use 'auto-dev stop' to stop the loop");
    
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