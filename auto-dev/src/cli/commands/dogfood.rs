//! Dogfood command for self-development

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Dogfood command for auto-dev-rs self-development
#[deprecated(note = "The dogfood command has been merged into self-dev")]
#[derive(Parser, Debug)]
pub struct DogfoodCommand {
    #[command(subcommand)]
    pub subcommand: DogfoodSubcommand,

    /// Use dry-run mode (show changes without applying)
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Skip confirmation prompts
    #[arg(long, global = true)]
    pub no_confirm: bool,
}

#[deprecated(note = "The dogfood command has been merged into self-dev")]
#[derive(Subcommand, Debug)]
pub enum DogfoodSubcommand {
    /// Validate dogfood configuration
    Validate,

    /// Monitor auto-dev-rs source tree
    Monitor {
        /// Watch for changes continuously
        #[arg(long)]
        watch: bool,
    },

    /// Check safety boundaries
    CheckSafety,

    /// Run dogfood mode with a specific task
    Run {
        /// Task description
        task: String,

        /// Skip pre-validation checks
        #[arg(long)]
        skip_validation: bool,
    },

    /// Initialize dogfood configuration
    Init,

    /// Show dogfood status and metrics
    Status,
}

#[deprecated(note = "The dogfood command has been merged into self-dev")]
/// Dogfood configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DogfoodConfig {
    pub project: ProjectConfig,
    pub monitoring: MonitoringConfig,
    pub synthesis: SynthesisConfig,
    pub parser: ParserConfig,
    pub validation: ValidationConfig,
    pub rollback: RollbackConfig,
    pub safety: SafetyConfig,
}

#[deprecated(note = "The dogfood command has been merged into self-dev")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub project_name: String,
    pub mode: String,
    pub description: String,
}

#[deprecated(note = "The dogfood command has been merged into self-dev")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub watch_patterns: Vec<String>,
    pub exclude: Vec<String>,
    pub debounce_ms: u64,
}

#[deprecated(note = "The dogfood command has been merged into self-dev")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisConfig {
    pub target_dir: String,
    pub safety_mode: String,
    pub allow_paths: Vec<String>,
    pub deny_paths: Vec<String>,
    pub dry_run_default: bool,
    pub max_file_size: usize,
}

#[deprecated(note = "The dogfood command has been merged into self-dev")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserConfig {
    pub include_todos: bool,
    pub todo_patterns: Vec<String>,
    pub todo_file_types: Vec<String>,
}

#[deprecated(note = "The dogfood command has been merged into self-dev")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub run_tests: bool,
    pub run_linting: bool,
    pub security_scanning: bool,
    pub min_coverage: u8,
    pub pre_validation: Vec<String>,
    pub post_validation: Vec<String>,
}

#[deprecated(note = "The dogfood command has been merged into self-dev")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackConfig {
    pub enabled: bool,
    pub backup_dir: String,
    pub max_backups: usize,
    pub rollback_on_test_failure: bool,
    pub rollback_on_build_failure: bool,
    pub rollback_on_lint_failure: bool,
}

#[deprecated(note = "The dogfood command has been merged into self-dev")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    pub require_confirmation: bool,
    pub allow_breaking_changes: bool,
    pub preserve_formatting: bool,
    pub maintain_backwards_compatibility: bool,
    pub max_files_per_operation: usize,
    pub max_lines_per_file: usize,
    pub max_operations_per_session: usize,
}

#[deprecated(note = "The dogfood command has been merged into self-dev")]
/// Execute the dogfood command - DEPRECATED: Redirects to self-dev
pub async fn execute(cmd: DogfoodCommand) -> Result<()> {
    println!("DEPRECATED: The 'dogfood' command has been merged into 'self-dev'");
    println!("Please use the following equivalent commands:\n");
    
    match cmd.subcommand {
        DogfoodSubcommand::Validate => {
            println!("  auto-dev self-dev validate");
            println!("\nRedirecting to self-dev validate...\n");
            super::self_dev::handle_self_dev_command(
                super::self_dev::SelfDevCommand::Validate
            ).await
        }
        DogfoodSubcommand::Monitor { watch } => {
            println!("  auto-dev self-dev monitor{}", if watch { " --watch" } else { "" });
            println!("\nRedirecting to self-dev monitor...\n");
            super::self_dev::handle_self_dev_command(
                super::self_dev::SelfDevCommand::Monitor { watch }
            ).await
        }
        DogfoodSubcommand::CheckSafety => {
            println!("  auto-dev self-dev check-safety");
            println!("\nRedirecting to self-dev check-safety...\n");
            super::self_dev::handle_self_dev_command(
                super::self_dev::SelfDevCommand::CheckSafety
            ).await
        }
        DogfoodSubcommand::Run { task, skip_validation } => {
            let dry_run = cmd.dry_run;
            let no_confirm = cmd.no_confirm;
            println!("  auto-dev self-dev run \"{}\"{}{}{}",
                task,
                if dry_run { " --dry-run" } else { "" },
                if skip_validation { " --skip-validation" } else { "" },
                if no_confirm { " --no-confirm" } else { "" }
            );
            println!("\nRedirecting to self-dev run...\n");
            super::self_dev::handle_self_dev_command(
                super::self_dev::SelfDevCommand::Run { 
                    task, 
                    dry_run, 
                    skip_validation, 
                    no_confirm 
                }
            ).await
        }
        DogfoodSubcommand::Init => {
            println!("  auto-dev self-dev init");
            println!("\nRedirecting to self-dev init...\n");
            super::self_dev::handle_self_dev_command(
                super::self_dev::SelfDevCommand::Init { force: false }
            ).await
        }
        DogfoodSubcommand::Status => {
            println!("  auto-dev self-dev status");
            println!("\nRedirecting to self-dev status...\n");
            super::self_dev::handle_self_dev_command(
                super::self_dev::SelfDevCommand::Status
            ).await
        }
    }
}

#[deprecated(note = "The dogfood command has been merged into self-dev")]
/// Validate the dogfood configuration
async fn validate_config() -> Result<()> {
    info!("Validating dogfood configuration...");

    let config_path = PathBuf::from("auto-dev.dogfood.toml");
    if !config_path.exists() {
        return Err(anyhow::anyhow!(
            "Dogfood configuration not found. Run 'auto-dev dogfood init' to create it."
        ));
    }

    let config_content =
        fs::read_to_string(&config_path).context("Failed to read dogfood configuration")?;

    let config: DogfoodConfig =
        toml::from_str(&config_content).context("Failed to parse dogfood configuration")?;

    // Validate paths exist
    for path in &config.synthesis.allow_paths {
        let p = Path::new(path);
        if !p.exists() && !path.contains('*') {
            warn!("Allow path does not exist: {}", path);
        }
    }

    // Check for conflicting paths
    for allow in &config.synthesis.allow_paths {
        for deny in &config.synthesis.deny_paths {
            if allow == deny || allow.starts_with(deny) {
                return Err(anyhow::anyhow!(
                    "Conflicting paths: '{}' is both allowed and denied",
                    allow
                ));
            }
        }
    }

    // Validate safety settings
    if config.safety.max_files_per_operation == 0 {
        return Err(anyhow::anyhow!("max_files_per_operation must be greater than 0"));
    }

    println!(" Configuration is valid!");
    println!("  Project: {}", config.project.project_name);
    println!("  Mode: {}", config.project.mode);
    println!("  Safety mode: {}", config.synthesis.safety_mode);
    println!("  Dry-run default: {}", config.synthesis.dry_run_default);

    Ok(())
}

#[deprecated(note = "The dogfood command has been merged into self-dev")]
/// Monitor the source tree
async fn monitor_source(watch: bool) -> Result<()> {
    info!("Monitoring auto-dev-rs source tree...");

    let config = load_config()?;

    println!(" Monitoring Configuration:");
    println!("  Watch patterns: {:?}", config.monitoring.watch_patterns);
    println!("  Exclude: {:?}", config.monitoring.exclude);

    if watch {
        println!("\n Watching for changes (press Ctrl+C to stop)...");
        // TODO: Implement actual file watching using the monitor module
        println!("  File watching not yet implemented in dogfood mode");
    } else {
        // Just scan once and report
        use auto_dev_core::parser::SpecParser;
        use auto_dev_core::parser::todo_extractor::TodoConfig;

        let mut todo_config = TodoConfig::default();
        todo_config.include_todos = config.parser.include_todos;
        todo_config.todo_patterns = config.parser.todo_patterns.clone();
        todo_config.file_types = config.parser.todo_file_types.clone();

        let parser = SpecParser::with_todo_config(todo_config);
        let specs = parser.parse_directory_with_todos(Path::new("src")).await?;

        let total_reqs: usize = specs.iter().map(|s| s.requirements.len()).sum();

        println!("\n Source Analysis:");
        println!("  Found {} specification files", specs.len());
        println!("  Total requirements: {}", total_reqs);

        // Show TODOs found
        let todo_reqs: Vec<_> = specs
            .iter()
            .flat_map(|s| &s.requirements)
            .filter(|r| r.tags.contains(&"todo".to_string()))
            .collect();

        if !todo_reqs.is_empty() {
            println!("\n  Found {} TODO items:", todo_reqs.len());
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

#[deprecated(note = "The dogfood command has been merged into self-dev")]
/// Check safety boundaries
async fn check_safety() -> Result<()> {
    info!("Checking safety boundaries...");

    let config = load_config()?;

    println!(" Safety Configuration:");
    println!("  Require confirmation: {}", config.safety.require_confirmation);
    println!("  Allow breaking changes: {}", config.safety.allow_breaking_changes);
    println!("  Max files per operation: {}", config.safety.max_files_per_operation);
    println!("  Max operations per session: {}", config.safety.max_operations_per_session);

    println!("\n Protected Paths:");
    for path in &config.synthesis.deny_paths {
        let exists = Path::new(path).exists() || path.contains('*');
        let status = if exists { "" } else { " (not found)" };
        println!("  - {}{}", path, status);
    }

    println!("\n Modifiable Paths:");
    for path in &config.synthesis.allow_paths {
        let exists = Path::new(path).exists() || path.contains('*');
        let status = if exists { "" } else { " (not found)" };
        println!("  + {}{}", path, status);
    }

    // Check for potential issues
    let cargo_lock = Path::new("Cargo.lock");
    if cargo_lock.exists() && !config.synthesis.deny_paths.contains(&"Cargo.lock".to_string()) {
        warn!("Warning: Cargo.lock is not in deny_paths!");
    }

    let git_dir = Path::new(".git");
    if git_dir.exists() && !config.synthesis.deny_paths.iter().any(|p| p.starts_with(".git")) {
        warn!("Warning: .git directory is not protected!");
    }

    println!("\n Safety boundaries verified.");

    Ok(())
}

#[deprecated(note = "The dogfood command has been merged into self-dev")]
/// Run dogfood mode with a specific task
async fn run_dogfood(task: String, dry_run: bool, skip_validation: bool) -> Result<()> {
    info!("Running dogfood mode: {}", task);

    let config = load_config()?;

    if config.safety.require_confirmation && !dry_run {
        println!(" Task: {}", task);
        println!(" This will modify auto-dev-rs's own code.");
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

    // Run pre-validation if not skipped
    if !skip_validation && !config.validation.pre_validation.is_empty() {
        println!("\n Running pre-validation checks...");
        for cmd in &config.validation.pre_validation {
            println!("  Running: {}", cmd);
            let output = std::process::Command::new("sh").arg("-c").arg(cmd).output()?;

            if !output.status.success() {
                return Err(anyhow::anyhow!(
                    "Pre-validation failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }
        println!("  Pre-validation passed!");
    }

    if dry_run || config.synthesis.dry_run_default {
        println!("\n DRY-RUN MODE - No changes will be made");
        println!(" Would execute task: {}", task);

        // TODO: Integrate with synthesis engine to show planned changes
        println!(" (Integration with synthesis engine pending)");
    } else {
        println!("\n Executing task: {}", task);

        // TODO: Integrate with synthesis engine to execute task
        println!(" (Integration with synthesis engine pending)");

        // Run post-validation
        if !config.validation.post_validation.is_empty() {
            println!("\n Running post-validation checks...");
            for cmd in &config.validation.post_validation {
                println!("  Running: {}", cmd);
                let output = std::process::Command::new("sh").arg("-c").arg(cmd).output()?;

                if !output.status.success() {
                    warn!("Post-validation failed: {}", String::from_utf8_lossy(&output.stderr));

                    if config.rollback.enabled && config.rollback.rollback_on_test_failure {
                        println!(" Triggering rollback...");
                        // TODO: Implement rollback
                    }
                }
            }
        }
    }

    Ok(())
}

#[deprecated(note = "The dogfood command has been merged into self-dev")]
/// Initialize dogfood configuration
async fn init_dogfood() -> Result<()> {
    info!("Initializing dogfood configuration...");

    let config_path = PathBuf::from("auto-dev.dogfood.toml");

    if config_path.exists() {
        println!(" Dogfood configuration already exists at: {}", config_path.display());
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

    // Create default configuration
    let default_config = include_str!("../../../../.auto-dev/config.toml");
    fs::write(&config_path, default_config)?;

    println!(" Created dogfood configuration at: {}", config_path.display());

    // Create .auto-dev directory if it doesn't exist
    let auto_dev_dir = PathBuf::from(".auto-dev");
    if !auto_dev_dir.exists() {
        fs::create_dir(&auto_dev_dir)?;
        println!(" Created .auto-dev directory");
    }

    // Create backup directory
    let backup_dir = auto_dev_dir.join("backups");
    if !backup_dir.exists() {
        fs::create_dir(&backup_dir)?;
        println!(" Created backup directory");
    }

    println!("\n Dogfood mode initialized!");
    println!(" Run 'auto-dev dogfood validate' to verify configuration");
    println!(" Run 'auto-dev dogfood monitor' to analyze the codebase");

    Ok(())
}

#[deprecated(note = "The dogfood command has been merged into self-dev")]
/// Show dogfood status and metrics
async fn show_status() -> Result<()> {
    info!("Showing dogfood status...");

    let config_path = PathBuf::from("auto-dev.dogfood.toml");
    if !config_path.exists() {
        println!(" Dogfood mode not initialized");
        println!(" Run 'auto-dev dogfood init' to get started");
        return Ok(());
    }

    let config = load_config()?;

    println!(" Dogfood Status");
    println!("  Project: {}", config.project.project_name);
    println!("  Mode: {}", config.project.mode);

    // Check metrics file
    let metrics_path = PathBuf::from(".auto-dev/metrics.json");
    if metrics_path.exists() {
        let metrics_content = fs::read_to_string(&metrics_path)?;
        if let Ok(metrics) = serde_json::from_str::<serde_json::Value>(&metrics_content) {
            println!("\n Metrics:");
            if let Some(mods) = metrics.get("total_modifications") {
                println!("  Total modifications: {}", mods);
            }
            if let Some(tests) = metrics.get("test_runs") {
                println!("  Test runs: {}", tests);
            }
            if let Some(rollbacks) = metrics.get("rollbacks") {
                println!("  Rollbacks: {}", rollbacks);
            }
        }
    } else {
        println!("\n No metrics available yet");
    }

    // Check backup directory
    let backup_dir = PathBuf::from(&config.rollback.backup_dir);
    if backup_dir.exists() {
        let backup_count = fs::read_dir(&backup_dir)?.count();
        println!("\n Backups: {} files", backup_count);
    }

    Ok(())
}

#[deprecated(note = "The dogfood command has been merged into self-dev")]
/// Load the dogfood configuration
fn load_config() -> Result<DogfoodConfig> {
    let config_path = PathBuf::from("auto-dev.dogfood.toml");

    if !config_path.exists() {
        return Err(anyhow::anyhow!(
            "Dogfood configuration not found. Run 'auto-dev dogfood init' to create it."
        ));
    }

    let config_content =
        fs::read_to_string(&config_path).context("Failed to read dogfood configuration")?;

    toml::from_str(&config_content).context("Failed to parse dogfood configuration")
}
