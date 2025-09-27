//! Bootstrap sequence orchestration

use super::snapshot::SnapshotContext;
use super::{
    BaselineCreator, BootstrapConfig, BootstrapError, BootstrapStage, BootstrapStatus,
    EnvironmentValidator, PreflightChecker, Result, SystemInitializer,
};
use std::fs;
use std::path::Path;
use tracing::{debug, error, info, warn};

pub struct BootstrapSequence {
    config: BootstrapConfig,
    status: BootstrapStatus,
}

impl BootstrapSequence {
    pub fn new(config: BootstrapConfig) -> Self {
        Self { config, status: BootstrapStatus::default() }
    }

    pub async fn resume() -> Result<Self> {
        let status = Self::load_checkpoint().await?;
        let config = Self::load_config().await?;

        info!("Resuming bootstrap from stage: {:?}", status.current_stage);

        Ok(Self { config, status })
    }

    pub async fn execute(&mut self) -> Result<()> {
        info!("Starting bootstrap sequence");

        // Check if bootstrap is already in progress
        if self.is_already_running()? {
            return Err(BootstrapError::AlreadyInProgress);
        }

        self.status.started_at = Some(chrono::Utc::now());
        self.save_checkpoint().await?;

        // Execute stages based on current status
        match self.status.current_stage {
            BootstrapStage::NotStarted => {
                self.run_all_stages().await?;
            }
            BootstrapStage::PreflightChecks => {
                self.resume_from_preflight().await?;
            }
            BootstrapStage::EnvironmentSetup => {
                self.resume_from_environment().await?;
            }
            BootstrapStage::BaselineCreation => {
                self.resume_from_baseline().await?;
            }
            BootstrapStage::Activation => {
                self.resume_from_activation().await?;
            }
            BootstrapStage::Completed => {
                info!("Bootstrap already completed");
                return Ok(());
            }
        }

        self.status.completed_at = Some(chrono::Utc::now());
        self.status.current_stage = BootstrapStage::Completed;
        self.save_checkpoint().await?;

        info!("Bootstrap sequence completed successfully");
        Ok(())
    }

    pub async fn dry_run(&self) -> Result<()> {
        info!("Running bootstrap dry-run");

        println!("\n=== Bootstrap Dry Run ===\n");

        // Stage 1: Pre-flight
        println!("Stage 1: Pre-flight Checks");
        let preflight = PreflightChecker::new(self.config.preflight.clone());
        for check in preflight.describe_checks() {
            println!("  ✓ Would check: {}", check);
        }

        // Stage 2: Environment
        println!("\nStage 2: Environment Setup");
        let validator = EnvironmentValidator::new();
        for validation in validator.describe_validations() {
            println!("  ✓ Would validate: {}", validation);
        }

        // Stage 3: Initialization
        println!("\nStage 3: System Initialization");
        let initializer = SystemInitializer::new(
            self.config.safety.clone(),
            self.config.modules.clone(),
            self.config.monitoring.clone(),
        );
        for step in initializer.describe_initialization_steps() {
            println!("  ✓ Would perform: {}", step);
        }

        // Stage 4: Baseline
        println!("\nStage 4: Baseline Creation");
        let baseline_creator = BaselineCreator::new(self.config.baseline.clone());
        for component in baseline_creator.describe_baseline_components() {
            println!("  ✓ Would create: {}", component);
        }

        // Stage 5: Activation
        println!("\nStage 5: Activation");
        println!("  ✓ Would start self-monitoring");
        println!("  ✓ Would enable specification generation");
        println!("  ✓ Would activate synthesis engine");
        println!("  ✓ Would begin continuous loop");
        println!("  ✓ Would start metric collection");
        println!("  ✓ Would enable hot-reload system");

        println!("\n=== Dry Run Complete ===");
        println!("No changes were made. Run without --dry-run to execute.");

        Ok(())
    }

    async fn run_all_stages(&mut self) -> Result<()> {
        self.run_preflight_stage().await?;
        self.run_environment_stage().await?;
        self.run_baseline_stage().await?;
        self.run_activation_stage().await?;
        Ok(())
    }

    async fn resume_from_preflight(&mut self) -> Result<()> {
        self.run_preflight_stage().await?;
        self.run_environment_stage().await?;
        self.run_baseline_stage().await?;
        self.run_activation_stage().await?;
        Ok(())
    }

    async fn resume_from_environment(&mut self) -> Result<()> {
        self.run_environment_stage().await?;
        self.run_baseline_stage().await?;
        self.run_activation_stage().await?;
        Ok(())
    }

    async fn resume_from_baseline(&mut self) -> Result<()> {
        self.run_baseline_stage().await?;
        self.run_activation_stage().await?;
        Ok(())
    }

    async fn resume_from_activation(&mut self) -> Result<()> {
        self.run_activation_stage().await?;
        Ok(())
    }

    async fn run_preflight_stage(&mut self) -> Result<()> {
        info!("Stage 1/4: Running pre-flight checks");
        self.status.current_stage = BootstrapStage::PreflightChecks;
        self.save_checkpoint().await?;

        let checker = PreflightChecker::new(self.config.preflight.clone());
        checker.run_checks().await?;

        Ok(())
    }

    async fn run_environment_stage(&mut self) -> Result<()> {
        info!("Stage 2/4: Setting up environment");
        self.status.current_stage = BootstrapStage::EnvironmentSetup;
        self.save_checkpoint().await?;

        // Validate environment
        let validator = EnvironmentValidator::new();
        validator.validate().await?;

        // Initialize system
        let initializer = SystemInitializer::new(
            self.config.safety.clone(),
            self.config.modules.clone(),
            self.config.monitoring.clone(),
        );
        initializer.initialize().await?;

        Ok(())
    }

    async fn run_baseline_stage(&mut self) -> Result<()> {
        info!("Stage 3/4: Creating baseline");
        self.status.current_stage = BootstrapStage::BaselineCreation;
        self.save_checkpoint().await?;

        let creator = BaselineCreator::new(self.config.baseline.clone());

        // Get previous context if resuming
        let previous_context = creator.get_previous_context();
        let attempt_number =
            previous_context.as_ref().map(|c| c.bootstrap_attempt + 1).unwrap_or(1);

        let context = SnapshotContext {
            reason: "Bootstrap initialization".to_string(),
            previous_snapshot_id: previous_context
                .as_ref()
                .map(|c| c.previous_snapshot_id.clone())
                .flatten(),
            bootstrap_attempt: attempt_number,
            environment_changes: vec![],
            intent_description: Some("Establishing baseline for self-development mode".to_string()),
        };

        let _baseline = creator.create_baseline(context).await?;

        Ok(())
    }

    async fn run_activation_stage(&mut self) -> Result<()> {
        info!("Stage 4/4: Activating self-development");
        self.status.current_stage = BootstrapStage::Activation;
        self.save_checkpoint().await?;

        // Start monitoring if configured
        if self.config.monitoring.start_immediately {
            info!("Starting file system monitoring");
            // Would integrate with monitor module here
        } else {
            info!(
                "Monitoring will start after {} seconds delay",
                self.config.monitoring.initial_delay_seconds
            );
        }

        // Initialize self-development components
        info!("Initializing self-development components");

        // Load self-dev configuration
        let self_dev_config = crate::self_dev::SelfDevConfig {
            enabled: true,
            mode: crate::self_dev::DevelopmentMode::Observation,
            safety_level: crate::self_dev::SafetyLevel::Strict,
            auto_approve: false,
            max_changes_per_day: 10,
            require_tests: true,
            require_documentation: true,
            components: crate::self_dev::ComponentConfig {
                monitoring: true,
                synthesis: false,
                testing: false,
                deployment: false,
                learning: false,
            },
        };

        // Initialize orchestrator
        let orchestrator = crate::self_dev::initialize(self_dev_config).await.map_err(|e| {
            BootstrapError::ActivationFailed(format!("Failed to initialize self-dev: {}", e))
        })?;

        // Start in observation mode
        orchestrator.start().await.map_err(|e| {
            BootstrapError::ActivationFailed(format!("Failed to start orchestrator: {}", e))
        })?;

        info!("Self-development mode activated in observation mode");

        Ok(())
    }

    fn is_already_running(&self) -> Result<bool> {
        // Check for lock file
        let lock_file = Path::new(".auto-dev/bootstrap.lock");
        if lock_file.exists() {
            // Check if lock is stale (older than 1 hour)
            if let Ok(metadata) = fs::metadata(lock_file) {
                if let Ok(modified) = metadata.modified() {
                    let elapsed =
                        std::time::SystemTime::now().duration_since(modified).unwrap_or_default();

                    if elapsed.as_secs() > 3600 {
                        warn!("Removing stale bootstrap lock file");
                        fs::remove_file(lock_file).ok();
                        return Ok(false);
                    }
                }
            }
            return Ok(true);
        }

        // Create lock file
        fs::write(lock_file, chrono::Utc::now().to_rfc3339()).map_err(|e| BootstrapError::Io(e))?;

        Ok(false)
    }

    async fn save_checkpoint(&self) -> Result<()> {
        let checkpoint_path = &self.status.checkpoint_path;

        // Ensure directory exists
        if let Some(parent) = checkpoint_path.parent() {
            fs::create_dir_all(parent).map_err(|e| BootstrapError::Io(e))?;
        }

        let json = serde_json::to_string_pretty(&self.status).map_err(|e| {
            BootstrapError::Configuration(format!("Failed to serialize checkpoint: {}", e))
        })?;

        fs::write(checkpoint_path, json).map_err(|e| BootstrapError::Io(e))?;

        debug!("Checkpoint saved at stage: {:?}", self.status.current_stage);
        Ok(())
    }

    async fn load_checkpoint() -> Result<BootstrapStatus> {
        let checkpoint_path = Path::new(".auto-dev/bootstrap.checkpoint");

        if !checkpoint_path.exists() {
            return Err(BootstrapError::Configuration(
                "No checkpoint found to resume from".to_string(),
            ));
        }

        let content = fs::read_to_string(checkpoint_path).map_err(|e| BootstrapError::Io(e))?;

        let status = serde_json::from_str(&content).map_err(|e| {
            BootstrapError::Configuration(format!("Failed to parse checkpoint: {}", e))
        })?;

        Ok(status)
    }

    async fn load_config() -> Result<BootstrapConfig> {
        let config_path = Path::new(".auto-dev/bootstrap.toml");

        if config_path.exists() {
            let content = fs::read_to_string(config_path).map_err(|e| BootstrapError::Io(e))?;

            let config = toml::from_str(&content).map_err(|e| {
                BootstrapError::Configuration(format!("Failed to parse config: {}", e))
            })?;

            Ok(config)
        } else {
            Ok(BootstrapConfig::default())
        }
    }

    pub async fn get_status() -> Result<BootstrapStatus> {
        if Path::new(".auto-dev/bootstrap.checkpoint").exists() {
            Self::load_checkpoint().await
        } else {
            Ok(BootstrapStatus::default())
        }
    }

    pub async fn reset() -> Result<()> {
        info!("Resetting bootstrap state");

        // Remove checkpoint
        let checkpoint_path = Path::new(".auto-dev/bootstrap.checkpoint");
        if checkpoint_path.exists() {
            fs::remove_file(checkpoint_path).map_err(|e| BootstrapError::Io(e))?;
        }

        // Remove lock file
        let lock_file = Path::new(".auto-dev/bootstrap.lock");
        if lock_file.exists() {
            fs::remove_file(lock_file).map_err(|e| BootstrapError::Io(e))?;
        }

        info!("Bootstrap state reset");
        Ok(())
    }
}
