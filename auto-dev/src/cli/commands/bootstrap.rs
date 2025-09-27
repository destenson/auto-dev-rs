//! Bootstrap command for self-development initialization

use anyhow::Result;
use auto_dev_core::bootstrap::{
    BootstrapConfig, BootstrapStage, BootstrapStatus, bootstrap, bootstrap_status, reset_bootstrap,
    resume_bootstrap,
};
use clap::{Args, Subcommand};
use tracing::{error, info};

/// Bootstrap self-development mode
#[derive(Debug, Args)]
pub struct BootstrapCommand {
    #[command(subcommand)]
    pub subcommand: Option<BootstrapSubcommand>,

    /// Run in dry-run mode (show what would be done without making changes)
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Subcommand)]
pub enum BootstrapSubcommand {
    /// Run pre-flight checks only
    Preflight {
        /// Run in strict mode (includes test suite)
        #[arg(long)]
        strict: bool,
    },

    /// Start bootstrap process
    Start {
        /// Skip pre-flight checks (dangerous)
        #[arg(long)]
        skip_preflight: bool,

        /// Don't require clean git state
        #[arg(long)]
        allow_dirty: bool,
    },

    /// Check bootstrap status
    Status,

    /// Resume interrupted bootstrap
    Resume,

    /// Reset bootstrap state
    Reset {
        /// Confirm reset without prompting
        #[arg(long)]
        force: bool,
    },
}

pub async fn handle_bootstrap_command(command: BootstrapCommand) -> Result<()> {
    match command.subcommand {
        Some(BootstrapSubcommand::Preflight { strict }) => run_preflight_only(strict).await,
        Some(BootstrapSubcommand::Start { skip_preflight, allow_dirty }) => {
            start_bootstrap(command.dry_run, skip_preflight, allow_dirty).await
        }
        Some(BootstrapSubcommand::Status) => show_bootstrap_status().await,
        Some(BootstrapSubcommand::Resume) => resume_bootstrap_process().await,
        Some(BootstrapSubcommand::Reset { force }) => reset_bootstrap_state(force).await,
        None => {
            // Default to start
            start_bootstrap(command.dry_run, false, false).await
        }
    }
}

async fn run_preflight_only(strict: bool) -> Result<()> {
    info!("Running pre-flight checks");

    let mut config = BootstrapConfig::default();
    config.preflight.strict = strict;

    println!("Running pre-flight checks...");
    println!("Mode: {}", if strict { "STRICT" } else { "NORMAL" });

    let checker = auto_dev_core::bootstrap::PreflightChecker::new(config.preflight);

    println!("\nChecks to perform:");
    for check in checker.describe_checks() {
        println!("  • {}", check);
    }
    println!();

    match checker.run_checks().await {
        Ok(_) => {
            println!("✅ All pre-flight checks passed!");
            println!("\nSystem is ready for bootstrap. Run 'auto-dev bootstrap start' to begin.");
        }
        Err(e) => {
            println!("❌ Pre-flight checks failed: {}", e);
            println!("\nPlease resolve the issues above before proceeding with bootstrap.");
            return Err(anyhow::anyhow!("Pre-flight checks failed"));
        }
    }

    Ok(())
}

async fn start_bootstrap(dry_run: bool, skip_preflight: bool, allow_dirty: bool) -> Result<()> {
    info!("Starting bootstrap process");

    let mut config = BootstrapConfig::default();

    if skip_preflight {
        println!("⚠️  WARNING: Skipping pre-flight checks");
        config.preflight.strict = false;
        config.preflight.check_disk_space = false;
        config.preflight.check_git_state = false;
    }

    if allow_dirty {
        config.preflight.require_clean_git = false;
        config.safety.require_clean_git = false;
    }

    if dry_run {
        println!("Running bootstrap in DRY-RUN mode");
    } else {
        println!("Starting bootstrap sequence for self-development mode");
        println!("\nThis will:");
        println!("  1. Validate your environment");
        println!("  2. Create necessary directories and configuration");
        println!("  3. Take a baseline snapshot of the system");
        println!("  4. Initialize self-development in observation mode");
        println!("\nPress Ctrl+C to abort at any time (can resume later)");

        // Give user a moment to read
        std::thread::sleep(std::time::Duration::from_secs(2));
    }

    match bootstrap(config, dry_run).await {
        Ok(_) => {
            if dry_run {
                println!("\n✅ Dry run completed successfully");
            } else {
                println!("\n✅ Bootstrap completed successfully!");
                println!("\nSelf-development mode is now active in observation mode.");
                println!("Use 'auto-dev self-dev status' to check the current state.");
            }
        }
        Err(e) => {
            error!("Bootstrap failed: {}", e);
            println!("\n❌ Bootstrap failed: {}", e);

            if !dry_run {
                println!("\nYou can:");
                println!("  • Run 'auto-dev bootstrap status' to check the current state");
                println!("  • Run 'auto-dev bootstrap resume' to continue from where it failed");
                println!("  • Run 'auto-dev bootstrap reset' to start over");
            }

            return Err(anyhow::anyhow!("Bootstrap failed"));
        }
    }

    Ok(())
}

async fn show_bootstrap_status() -> Result<()> {
    info!("Checking bootstrap status");

    match bootstrap_status().await {
        Ok(status) => {
            display_status(&status);
        }
        Err(e) => {
            println!("Failed to get bootstrap status: {}", e);
            println!("\nBootstrap may not have been started yet.");
            println!("Run 'auto-dev bootstrap start' to begin.");
        }
    }

    Ok(())
}

async fn resume_bootstrap_process() -> Result<()> {
    info!("Resuming bootstrap process");

    println!("Attempting to resume bootstrap...");

    match resume_bootstrap().await {
        Ok(_) => {
            println!("\n✅ Bootstrap resumed and completed successfully!");
            println!("\nSelf-development mode is now active.");
        }
        Err(e) => {
            error!("Failed to resume bootstrap: {}", e);
            println!("\n❌ Failed to resume bootstrap: {}", e);
            println!("\nYou may need to:");
            println!("  • Run 'auto-dev bootstrap reset' to start over");
            println!("  • Fix any issues and try again");
            return Err(anyhow::anyhow!("Resume failed"));
        }
    }

    Ok(())
}

async fn reset_bootstrap_state(force: bool) -> Result<()> {
    info!("Resetting bootstrap state");

    if !force {
        println!("This will reset all bootstrap progress and checkpoints.");
        print!("Continue? [y/N] ");

        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Reset cancelled.");
            return Ok(());
        }
    }

    match reset_bootstrap().await {
        Ok(_) => {
            println!("✅ Bootstrap state has been reset.");
            println!("\nYou can now run 'auto-dev bootstrap start' to begin fresh.");
        }
        Err(e) => {
            println!("❌ Failed to reset bootstrap: {}", e);
            return Err(anyhow::anyhow!("Reset failed"));
        }
    }

    Ok(())
}

fn display_status(status: &BootstrapStatus) {
    println!("Bootstrap Status");
    println!("================");

    println!("\nCurrent Stage: {}", format_stage(&status.current_stage));

    if let Some(started) = status.started_at {
        println!("Started: {}", started.format("%Y-%m-%d %H:%M:%S UTC"));
    }

    if let Some(completed) = status.completed_at {
        println!("Completed: {}", completed.format("%Y-%m-%d %H:%M:%S UTC"));
    }

    if let Some(error) = &status.error {
        println!("\n❌ Last Error: {}", error);
    }

    match status.current_stage {
        BootstrapStage::NotStarted => {
            println!("\nBootstrap has not been started.");
            println!("Run 'auto-dev bootstrap start' to begin.");
        }
        BootstrapStage::Completed => {
            println!("\n✅ Bootstrap completed successfully!");
            println!("Self-development mode should be active.");
        }
        _ => {
            println!("\n⏸️  Bootstrap is partially complete.");
            println!("Run 'auto-dev bootstrap resume' to continue.");

            println!("\nRemaining stages:");
            let remaining = get_remaining_stages(&status.current_stage);
            for stage in remaining {
                println!("  • {}", stage);
            }
        }
    }
}

fn format_stage(stage: &BootstrapStage) -> String {
    match stage {
        BootstrapStage::NotStarted => "Not Started".to_string(),
        BootstrapStage::PreflightChecks => "Pre-flight Checks".to_string(),
        BootstrapStage::EnvironmentSetup => "Environment Setup".to_string(),
        BootstrapStage::BaselineCreation => "Baseline Creation".to_string(),
        BootstrapStage::Activation => "Activation".to_string(),
        BootstrapStage::Completed => "Completed".to_string(),
    }
}

fn get_remaining_stages(current: &BootstrapStage) -> Vec<String> {
    let all_stages = vec![
        BootstrapStage::PreflightChecks,
        BootstrapStage::EnvironmentSetup,
        BootstrapStage::BaselineCreation,
        BootstrapStage::Activation,
    ];

    let current_index = match current {
        BootstrapStage::NotStarted => 0,
        BootstrapStage::PreflightChecks => 1,
        BootstrapStage::EnvironmentSetup => 2,
        BootstrapStage::BaselineCreation => 3,
        BootstrapStage::Activation => 4,
        BootstrapStage::Completed => 5,
    };

    all_stages.into_iter().skip(current_index).map(|s| format_stage(&s)).collect()
}
