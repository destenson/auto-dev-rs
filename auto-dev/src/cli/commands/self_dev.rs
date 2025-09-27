//! CLI commands for self-development control

use anyhow::Result;
use auto_dev_core::self_dev::{
    ControlCommand, DevelopmentMode, SafetyLevel, SelfDevConfig, SelfDevOrchestrator,
};
use clap::{Args, Subcommand};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

/// Self-development control commands
#[derive(Debug, Args)]
pub struct SelfDevCommand {
    #[command(subcommand)]
    pub subcommand: SelfDevSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum SelfDevSubcommand {
    /// Start self-development mode
    Start {
        /// Development mode (observation, assisted, semi-autonomous, fully-autonomous)
        #[arg(long, default_value = "observation")]
        mode: String,

        /// Safety level (permissive, standard, strict)
        #[arg(long, default_value = "strict")]
        safety: String,
    },

    /// Stop self-development mode
    Stop,

    /// Pause self-development
    Pause,

    /// Resume self-development
    Resume,

    /// Emergency stop with rollback
    EmergencyStop,

    /// Get current self-development status
    Status,

    /// Review pending changes
    Review,

    /// Approve a specific change
    Approve {
        /// Change ID to approve
        change_id: String,
    },

    /// Reject a specific change
    Reject {
        /// Change ID to reject
        change_id: String,
    },

    /// Set maximum changes per day
    SetLimit {
        /// Maximum number of changes allowed per day
        limit: usize,
    },

    /// Run a specific task manually (dogfood mode)
    Run {
        /// Task description or PRP number
        task: String,

        /// Use dry-run mode (show changes without applying)
        #[arg(long)]
        dry_run: bool,

        /// Skip pre-validation checks
        #[arg(long)]
        skip_validation: bool,

        /// Skip confirmation prompts
        #[arg(long)]
        no_confirm: bool,
    },

    /// Monitor source tree for changes and TODOs
    Monitor {
        /// Watch for changes continuously
        #[arg(long)]
        watch: bool,
    },

    /// Validate configuration and safety boundaries
    Validate,

    /// Initialize self-development configuration
    Init {
        /// Force overwrite existing configuration
        #[arg(long)]
        force: bool,
    },

    /// Check safety boundaries
    CheckSafety,
}

static ORCHESTRATOR: once_cell::sync::Lazy<Arc<RwLock<Option<SelfDevOrchestrator>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(None)));

pub async fn handle_self_dev_command(command: SelfDevCommand) -> Result<()> {
    match command.subcommand {
        SelfDevSubcommand::Start { mode, safety } => {
            start_self_dev(parse_mode(&mode)?, parse_safety(&safety)?).await
        }
        SelfDevSubcommand::Stop => stop_self_dev().await,
        SelfDevSubcommand::Pause => pause_self_dev().await,
        SelfDevSubcommand::Resume => resume_self_dev().await,
        SelfDevSubcommand::EmergencyStop => emergency_stop().await,
        SelfDevSubcommand::Status => show_status().await,
        SelfDevSubcommand::Review => review_changes().await,
        SelfDevSubcommand::Approve { change_id } => approve_change(change_id).await,
        SelfDevSubcommand::Reject { change_id } => reject_change(change_id).await,
        SelfDevSubcommand::SetLimit { limit } => set_change_limit(limit).await,
        SelfDevSubcommand::Run { task, dry_run, skip_validation, no_confirm } => {
            run_manual_task(task, dry_run, skip_validation, no_confirm).await
        }
        SelfDevSubcommand::Monitor { watch } => monitor_source(watch).await,
        SelfDevSubcommand::Validate => validate_configuration().await,
        SelfDevSubcommand::Init { force } => init_self_dev(force).await,
        SelfDevSubcommand::CheckSafety => check_safety_boundaries().await,
    }
}

async fn start_self_dev(mode: DevelopmentMode, safety: SafetyLevel) -> Result<()> {
    info!("Starting self-development mode");

    let config = SelfDevConfig {
        enabled: true,
        mode: mode.clone(),
        safety_level: safety.clone(),
        auto_approve: matches!(mode, DevelopmentMode::FullyAutonomous),
        max_changes_per_day: 10,
        require_tests: true,
        require_documentation: true,
        components: auto_dev_core::self_dev::ComponentConfig {
            monitoring: true,
            synthesis: matches!(
                mode,
                DevelopmentMode::Assisted
                    | DevelopmentMode::SemiAutonomous
                    | DevelopmentMode::FullyAutonomous
            ),
            testing: matches!(
                mode,
                DevelopmentMode::SemiAutonomous | DevelopmentMode::FullyAutonomous
            ),
            deployment: matches!(mode, DevelopmentMode::FullyAutonomous),
            learning: true,
        },
    };

    let orchestrator = auto_dev_core::self_dev::initialize(config).await?;
    orchestrator.start().await?;

    *ORCHESTRATOR.write().await = Some(orchestrator);

    println!("Self-development started in {:?} mode with {:?} safety", mode, safety);
    Ok(())
}

async fn stop_self_dev() -> Result<()> {
    info!("Stopping self-development mode");

    let mut orchestrator_lock = ORCHESTRATOR.write().await;
    if let Some(orchestrator) = orchestrator_lock.as_ref() {
        orchestrator.stop().await?;
        *orchestrator_lock = None;
        println!("Self-development stopped");
    } else {
        println!("Self-development is not running");
    }

    Ok(())
}

async fn pause_self_dev() -> Result<()> {
    let orchestrator_lock = ORCHESTRATOR.read().await;
    if let Some(orchestrator) = orchestrator_lock.as_ref() {
        orchestrator.pause().await?;
        println!("Self-development paused");
    } else {
        println!("Self-development is not running");
    }
    Ok(())
}

async fn resume_self_dev() -> Result<()> {
    let orchestrator_lock = ORCHESTRATOR.read().await;
    if let Some(orchestrator) = orchestrator_lock.as_ref() {
        orchestrator.resume().await?;
        println!("Self-development resumed");
    } else {
        println!("Self-development is not running");
    }
    Ok(())
}

async fn emergency_stop() -> Result<()> {
    error!("EMERGENCY STOP TRIGGERED");

    let mut orchestrator_lock = ORCHESTRATOR.write().await;
    if let Some(orchestrator) = orchestrator_lock.as_ref() {
        orchestrator.emergency_stop().await?;
        *orchestrator_lock = None;
        println!("Emergency stop completed - all changes rolled back");
    } else {
        println!("Self-development is not running");
    }

    Ok(())
}

async fn show_status() -> Result<()> {
    let orchestrator_lock = ORCHESTRATOR.read().await;
    if let Some(orchestrator) = orchestrator_lock.as_ref() {
        let status = orchestrator.get_status().await?;

        println!("Self-Development Status");
        println!("=======================");
        println!("State: {}", status.current_state);
        println!("Mode: {:?}", status.mode);
        println!("Paused: {}", status.is_paused);
        println!("Today's changes: {}", status.today_changes);
        println!("Pending changes: {}", status.pending_changes.len());
        println!("Active components: {}", status.active_components.join(", "));

        if !status.pending_changes.is_empty() {
            println!("\nPending Changes:");
            for change in &status.pending_changes {
                println!("  - {} ({:?}): {}", change.id, change.change_type, change.description);
            }
        }
    } else {
        println!("Self-development is not running");
    }

    Ok(())
}

async fn review_changes() -> Result<()> {
    let orchestrator_lock = ORCHESTRATOR.read().await;
    if let Some(orchestrator) = orchestrator_lock.as_ref() {
        let changes = orchestrator.review_changes().await?;

        if changes.is_empty() {
            println!("No pending changes to review");
        } else {
            println!("Pending Changes for Review");
            println!("===========================");
            for change in changes {
                println!("\nChange ID: {}", change.id);
                println!("Description: {}", change.description);
                println!("File: {}", change.file_path);
                println!("Type: {:?}", change.change_type);
                println!("Risk: {:?}", change.risk_level);
                println!("---");
            }
            println!(
                "\nUse 'auto-dev self-dev approve <id>' or 'auto-dev self-dev reject <id>' to handle changes"
            );
        }
    } else {
        println!("Self-development is not running");
    }

    Ok(())
}

async fn approve_change(change_id: String) -> Result<()> {
    let orchestrator_lock = ORCHESTRATOR.read().await;
    if let Some(orchestrator) = orchestrator_lock.as_ref() {
        orchestrator.approve_change(change_id.clone()).await?;
        println!("Change {} approved", change_id);
    } else {
        println!("Self-development is not running");
    }
    Ok(())
}

async fn reject_change(change_id: String) -> Result<()> {
    let orchestrator_lock = ORCHESTRATOR.read().await;
    if let Some(orchestrator) = orchestrator_lock.as_ref() {
        orchestrator.reject_change(change_id.clone()).await?;
        println!("Change {} rejected", change_id);
    } else {
        println!("Self-development is not running");
    }
    Ok(())
}

async fn set_change_limit(limit: usize) -> Result<()> {
    if limit == 0 || limit > 1000 {
        println!("Invalid limit. Must be between 1 and 1000");
        return Ok(());
    }

    let orchestrator_lock = ORCHESTRATOR.read().await;
    if let Some(orchestrator) = orchestrator_lock.as_ref() {
        orchestrator.handle_control_command(ControlCommand::SetMaxChangesPerDay(limit)).await?;
        println!("Maximum changes per day set to {}", limit);
    } else {
        println!("Self-development is not running");
    }

    Ok(())
}

fn parse_mode(mode: &str) -> Result<DevelopmentMode> {
    match mode.to_lowercase().as_str() {
        "observation" => Ok(DevelopmentMode::Observation),
        "assisted" => Ok(DevelopmentMode::Assisted),
        "semi-autonomous" => Ok(DevelopmentMode::SemiAutonomous),
        "fully-autonomous" => Ok(DevelopmentMode::FullyAutonomous),
        _ => Err(anyhow::anyhow!(
            "Invalid mode. Use: observation, assisted, semi-autonomous, or fully-autonomous"
        )),
    }
}

fn parse_safety(safety: &str) -> Result<SafetyLevel> {
    match safety.to_lowercase().as_str() {
        "permissive" => Ok(SafetyLevel::Permissive),
        "standard" => Ok(SafetyLevel::Standard),
        "strict" => Ok(SafetyLevel::Strict),
        _ => Err(anyhow::anyhow!("Invalid safety level. Use: permissive, standard, or strict")),
    }
}

async fn run_manual_task(
    task: String,
    dry_run: bool,
    skip_validation: bool,
    no_confirm: bool,
) -> Result<()> {
    info!("Running manual task: {}", task);

    // Load self-target configuration for safety checks
    let self_target_config = auto_dev_core::self_target::SelfTargetConfig::load_or_create()?;

    if !no_confirm && self_target_config.safety.require_confirmation && !dry_run {
        println!("Task: {}", task);
        println!("This will modify auto-dev-rs's own code.");
        print!("Continue? [y/N] ");

        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    if !skip_validation {
        println!("Running pre-validation checks...");
        // Run cargo check
        let output = std::process::Command::new("cargo").arg("check").output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Pre-validation failed: cargo check failed\n{}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        println!("Pre-validation passed");
    }

    if dry_run {
        println!("\nDRY-RUN MODE - No changes will be made");
        println!("Would execute task: {}", task);

        // Check if task is a PRP number
        if task.starts_with("PRP-") || task.starts_with("prp-") || task.parse::<u32>().is_ok() {
            let prp_num = task.replace("PRP-", "").replace("prp-", "");
            println!("Would implement PRP-{}", prp_num);
        }
    } else {
        println!("\nExecuting task: {}", task);

        // Create a temporary orchestrator for manual task execution
        let config = SelfDevConfig {
            enabled: true,
            mode: DevelopmentMode::Assisted,
            safety_level: SafetyLevel::Standard,
            auto_approve: false,
            max_changes_per_day: 50,
            require_tests: true,
            require_documentation: true,
            components: auto_dev_core::self_dev::ComponentConfig {
                monitoring: true,
                synthesis: true,
                testing: true,
                deployment: false,
                learning: true,
            },
        };

        let orchestrator = auto_dev_core::self_dev::initialize(config).await?;

        // Execute the task through the orchestrator
        println!("Processing task: {}", task);
        orchestrator.start().await?;
        orchestrator.execute_task(&task).await?;
        orchestrator.stop().await?;

        if !skip_validation {
            println!("\nRunning post-validation checks...");
            let output = std::process::Command::new("cargo").arg("test").output()?;

            if !output.status.success() {
                println!("Post-validation warning: some tests failed");
            } else {
                println!("Post-validation passed");
            }
        }
    }

    Ok(())
}

async fn monitor_source(watch: bool) -> Result<()> {
    info!("Monitoring auto-dev-rs source tree");

    let self_target_config = auto_dev_core::self_target::SelfTargetConfig::load_or_create()?;

    println!("Monitoring Configuration:");
    println!("  Watch patterns: {:?}", self_target_config.monitor.watch);
    println!("  Exclude: {:?}", self_target_config.monitor.exclude);

    if watch {
        println!("\nWatching for changes (press Ctrl+C to stop)...");
        // TODO: Integrate with FileSystemMonitor for continuous watching
        println!("  Continuous watching pending monitor integration");
    } else {
        use auto_dev_core::parser::SpecParser;
        use auto_dev_core::parser::todo_extractor::TodoConfig;
        use std::path::Path;

        let mut todo_config = TodoConfig::default();
        todo_config.include_todos = true;
        todo_config.todo_patterns =
            vec!["TODO".to_string(), "FIXME".to_string(), "HACK".to_string(), "NOTE".to_string()];

        let parser = SpecParser::with_todo_config(todo_config);
        let specs = parser.parse_directory_with_todos(Path::new("src")).await?;

        let total_reqs: usize = specs.iter().map(|s| s.requirements.len()).sum();

        println!("\nSource Analysis:");
        println!("  Found {} specification files", specs.len());
        println!("  Total requirements: {}", total_reqs);

        let todo_reqs: Vec<_> = specs
            .iter()
            .flat_map(|s| &s.requirements)
            .filter(|r| r.tags.contains(&"todo".to_string()))
            .collect();

        if !todo_reqs.is_empty() {
            println!("\nFound {} TODO items:", todo_reqs.len());
            for (i, req) in todo_reqs.iter().take(5).enumerate() {
                println!("    {}. [{}] {}", i + 1, req.priority, req.description);
            }
            if todo_reqs.len() > 5 {
                println!("    ... and {} more", todo_reqs.len() - 5);
            }
        }
    }

    Ok(())
}

async fn validate_configuration() -> Result<()> {
    info!("Validating self-development configuration");

    let self_target_config = auto_dev_core::self_target::SelfTargetConfig::load_or_create()?;

    println!("Configuration loaded successfully");
    println!(
        "📦 Project: {} v{}",
        self_target_config.project.name, self_target_config.project.version
    );
    println!("📁 Path: {}", self_target_config.project.path.display());

    // Validate safety settings
    if self_target_config.safety.forbidden_paths.is_empty() {
        println!("Warning: No forbidden paths configured");
    }

    // Check for path conflicts
    for watch_path in &self_target_config.monitor.watch {
        for exclude_path in &self_target_config.monitor.exclude {
            if watch_path.starts_with(exclude_path) {
                println!(
                    "Conflict: '{}' is watched but excluded by '{}'",
                    watch_path, exclude_path
                );
            }
        }
    }

    println!("\nSafety Configuration:");
    println!("  Safety mode: {:?}", self_target_config.synthesis.safety_mode);
    println!("  Require confirmation: {}", self_target_config.safety.require_confirmation);
    println!("  Create backups: {}", self_target_config.safety.create_backups);
    println!("  Max file size: {} bytes", self_target_config.safety.max_file_size);

    Ok(())
}

async fn init_self_dev(force: bool) -> Result<()> {
    info!("Initializing self-development configuration");

    let config_path = auto_dev_core::self_target::SelfTargetConfig::default_config_path();

    if config_path.exists() && !force {
        println!("Configuration already exists at: {}", config_path.display());
        print!("Overwrite? [y/N] ");

        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Initialization cancelled.");
            return Ok(());
        }
    }

    // Initialize using self-target configuration
    auto_dev_core::self_target::init_self_targeting().await?;

    println!("\nSelf-development initialized!");
    println!("Configuration saved to: {}", config_path.display());
    println!("\nNext steps:");
    println!("  1. Run 'auto-dev self-dev validate' to verify configuration");
    println!("  2. Run 'auto-dev self-dev monitor' to analyze the codebase");
    println!("  3. Run 'auto-dev self-dev start' to begin autonomous development");

    Ok(())
}

async fn check_safety_boundaries() -> Result<()> {
    info!("Checking safety boundaries");

    let self_target_config = auto_dev_core::self_target::SelfTargetConfig::load_or_create()?;

    println!("Safety Boundaries Check");
    println!("========================");

    println!("\nForbidden Paths:");
    for path in &self_target_config.safety.forbidden_paths {
        let exists = std::path::Path::new(path).exists();
        let status = if exists { "[exists]" } else { "[missing]" };
        println!("  {} {}", status, path);
    }

    println!("\nMonitored Paths:");
    for path in &self_target_config.monitor.watch {
        println!("  + {}", path);
    }

    println!("\nExcluded Paths:");
    for path in &self_target_config.monitor.exclude {
        println!("  - {}", path);
    }

    // Check for critical paths
    let critical_paths = vec![
        (".git", "Git repository"),
        ("Cargo.lock", "Dependency lock file"),
        ("target", "Build artifacts"),
    ];

    println!("\nCritical Path Protection:");
    for (path, desc) in critical_paths {
        let protected =
            self_target_config.safety.forbidden_paths.iter().any(|p| p.starts_with(path));

        let status = if protected { "Protected" } else { "NOT PROTECTED" };
        println!("  {} - {} ({})", path, desc, status);
    }

    println!("\nSafety Settings:");
    println!("  Max file size: {} KB", self_target_config.safety.max_file_size / 1024);
    println!("  Require confirmation: {}", self_target_config.safety.require_confirmation);
    println!("  Create backups: {}", self_target_config.safety.create_backups);

    Ok(())
}
