#![allow(unused)]
//! Central orchestrator for self-development activities

use super::{
    ComponentCoordinator, ControlCommand, DevelopmentMode, DevelopmentState,
    DevelopmentStateMachine, OperatorInterface, Result, SafetyMonitor, SelfDevConfig, SelfDevError,
};
use crate::parser::SpecParser;
use crate::parser::model::{Priority, Requirement, RequirementType, SourceLocation, Specification};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{debug, error, info, warn};

/// Lifecycle status for a pending change tracked by the orchestrator
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangeStatus {
    Discovered,
    Analyzed,
    Planned,
    Generating,
    ReadyForTesting,
    Testing,
    ReadyForReview,
    Approved,
    Deploying,
    Deployed,
    RolledBack,
}

impl fmt::Display for ChangeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discovered => write!(f, "Discovered"),
            Self::Analyzed => write!(f, "Analyzed"),
            Self::Planned => write!(f, "Planned"),
            Self::Generating => write!(f, "Generating"),
            Self::ReadyForTesting => write!(f, "Ready for Testing"),
            Self::Testing => write!(f, "Testing"),
            Self::ReadyForReview => write!(f, "Ready for Review"),
            Self::Approved => write!(f, "Approved"),
            Self::Deploying => write!(f, "Deploying"),
            Self::Deployed => write!(f, "Deployed"),
            Self::RolledBack => write!(f, "Rolled Back"),
        }
    }
}

/// Per-change telemetry captured across the self-development lifecycle
#[derive(Debug, Clone, Default)]
pub struct ChangeMetrics {
    pub test_runs: Vec<TestRunSummary>,
    pub safety_failures: u32,
    pub deployments: u32,
}

/// One step within the planned implementation for a change
#[derive(Debug, Clone)]
pub struct PlanStep {
    pub id: String,
    pub description: String,
    pub depends_on: Vec<String>,
    pub tests: Vec<String>,
}

/// Digest of the implementation plan used during development
#[derive(Debug, Clone)]
pub struct PlanDigest {
    pub steps: Vec<PlanStep>,
    pub estimated_duration: Duration,
    pub critical_path: Vec<String>,
}

/// Summary of a single test run executed for a change
#[derive(Debug, Clone)]
pub struct TestRunSummary {
    pub command: String,
    pub duration: Duration,
    pub passed: bool,
    pub details: Option<String>,
}

/// Aggregated test results for the current development cycle
#[derive(Debug, Clone)]
pub struct TestResults {
    total: usize,
    failed: usize,
    skipped: usize,
    runs: Vec<TestRunSummary>,
}

impl TestResults {
    pub fn new(passed: usize, failed: usize, skipped: usize) -> Self {
        Self { total: passed + failed + skipped, failed, skipped, runs: Vec::new() }
    }

    pub fn from_runs(runs: Vec<TestRunSummary>) -> Self {
        let mut summary = Self::new(0, 0, 0);
        for run in runs {
            summary.record_run(run);
        }
        summary
    }

    pub fn record_run(&mut self, run: TestRunSummary) {
        if run.passed {
            self.total += 1;
        } else {
            self.total += 1;
            self.failed += 1;
        }
        self.runs.push(run);
    }

    pub fn runs(&self) -> &[TestRunSummary] {
        &self.runs
    }

    pub fn all_passed(&self) -> bool {
        self.failed == 0 && !self.runs.is_empty()
    }

    pub fn failed(&self) -> usize {
        self.failed
    }
}

impl Default for TestResults {
    fn default() -> Self {
        Self { total: 0, failed: 0, skipped: 0, runs: Vec::new() }
    }
}

/// Representation of a change under consideration for self-development
#[derive(Debug, Clone)]
pub struct PendingChange {
    pub id: String,
    pub description: String,
    pub summary: Option<String>,
    pub file_path: String,
    pub change_type: ChangeType,
    pub risk_level: RiskLevel,
    pub status: ChangeStatus,
    pub plan: Option<PlanDigest>,
    pub target_files: Vec<PathBuf>,
    pub required_components: Vec<String>,
    pub last_updated: SystemTime,
    pub metrics: ChangeMetrics,
}

impl PendingChange {
    pub fn touch(&mut self) {
        self.last_updated = SystemTime::now();
    }
}

#[derive(Debug, Clone)]
pub enum ChangeType {
    Create,
    Modify,
    Delete,
    Refactor,
}

#[derive(Debug, Clone)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

pub struct SelfDevOrchestrator {
    config: Arc<RwLock<SelfDevConfig>>,
    state_machine: Arc<Mutex<DevelopmentStateMachine>>,
    coordinator: Arc<ComponentCoordinator>,
    safety_monitor: Arc<SafetyMonitor>,
    operator_interface: Arc<OperatorInterface>,
    shutdown_tx: mpsc::Sender<()>,
    shutdown_rx: Arc<Mutex<mpsc::Receiver<()>>>,
    project_root: PathBuf,
}

impl SelfDevOrchestrator {
    pub async fn new(config: SelfDevConfig) -> Result<Self> {
        let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        let config = Arc::new(RwLock::new(config));
        let state_machine = DevelopmentStateMachine::new(DevelopmentState::Idle);
        let coordinator = ComponentCoordinator::new(config.clone(), project_root.clone()).await;
        let safety_level = { config.read().await.safety_level.clone() };
        let safety_monitor = SafetyMonitor::new(safety_level);
        let operator_interface = OperatorInterface::new();

        Ok(Self {
            config,
            state_machine: Arc::new(Mutex::new(state_machine)),
            coordinator: Arc::new(coordinator),
            safety_monitor: Arc::new(safety_monitor),
            operator_interface: Arc::new(operator_interface),
            shutdown_tx,
            shutdown_rx: Arc::new(Mutex::new(shutdown_rx)),
            project_root,
        })
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting self-development orchestrator");

        let config = self.config.read().await;
        if !config.enabled {
            return Err(SelfDevError::Configuration("Self-development is not enabled".to_string()));
        }

        drop(config);
        self.transition_state(DevelopmentState::Analyzing).await?;

        let orchestrator = self.clone_internal().await;
        tokio::spawn(async move {
            if let Err(e) = orchestrator.run_development_loop().await {
                error!("Self-development loop error: {}", e);
            }
        });

        info!("Self-development orchestrator started");
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        info!("Stopping self-development orchestrator");

        self.transition_state(DevelopmentState::Idle).await?;

        if self.shutdown_tx.send(()).await.is_err() {
            warn!("Shutdown channel already closed");
        }

        info!("Self-development orchestrator stopped");
        Ok(())
    }

    pub async fn pause(&self) -> Result<()> {
        info!("Pausing self-development");

        let mut state_machine = self.state_machine.lock().await;
        state_machine.pause()?;

        Ok(())
    }

    pub async fn resume(&self) -> Result<()> {
        info!("Resuming self-development");

        let mut state_machine = self.state_machine.lock().await;
        state_machine.resume()?;

        Ok(())
    }

    pub async fn emergency_stop(&self) -> Result<()> {
        error!("Emergency stop initiated");

        self.transition_state(DevelopmentState::Idle).await?;

        if self.shutdown_tx.send(()).await.is_err() {
            warn!("Shutdown channel already closed");
        }

        self.coordinator.rollback_all().await?;

        error!("Emergency stop completed");
        Ok(())
    }

    pub async fn get_status(&self) -> Result<SelfDevStatus> {
        let state_machine = self.state_machine.lock().await;
        let config = self.config.read().await;

        Ok(SelfDevStatus {
            current_state: state_machine.current_state(),
            is_paused: state_machine.is_paused(),
            mode: config.mode.clone(),
            safety_level: config.safety_level.clone(),
            active_components: self.coordinator.get_active_components().await,
            pending_changes: self.coordinator.get_pending_changes().await?,
            today_changes: self.coordinator.get_today_changes_count().await,
        })
    }

    pub async fn review_changes(&self) -> Result<Vec<PendingChange>> {
        self.coordinator.get_pending_changes().await
    }

    pub async fn approve_change(&self, change_id: String) -> Result<()> {
        if let Some(change) = self.coordinator.lookup_change(&change_id).await? {
            if !self.safety_monitor.validate_change(&change).await? {
                return Err(SelfDevError::SafetyViolation(format!(
                    "Change {} failed safety validation",
                    change_id
                )));
            }

            self.coordinator.approve_change(change_id).await
        } else {
            Err(SelfDevError::Coordination(format!("Change {} not found", change_id)))
        }
    }

    pub async fn reject_change(&self, change_id: String) -> Result<()> {
        self.coordinator.reject_change(change_id).await
    }

    pub async fn handle_control_command(&self, command: ControlCommand) -> Result<()> {
        let ticket = self.operator_interface.handle_command(command.clone()).await?;
        let result = self.apply_control_command(command).await;

        match &result {
            Ok(_) => {
                self.operator_interface
                    .finalize_command(ticket, super::control::CommandResult::Success)
                    .await;
            }
            Err(err) => {
                self.operator_interface
                    .finalize_command(
                        ticket,
                        super::control::CommandResult::Failure(err.to_string()),
                    )
                    .await;
            }
        }

        result
    }

    async fn apply_control_command(&self, command: ControlCommand) -> Result<()> {
        match command {
            ControlCommand::Start => self.start().await,
            ControlCommand::Stop => self.stop().await,
            ControlCommand::Pause => self.pause().await,
            ControlCommand::Resume => self.resume().await,
            ControlCommand::EmergencyStop => self.emergency_stop().await,
            ControlCommand::GetStatus => {
                self.get_status().await?;
                Ok(())
            }
            ControlCommand::ReviewChanges => {
                self.review_changes().await?;
                Ok(())
            }
            ControlCommand::ApproveChange(change_id) => self.approve_change(change_id).await,
            ControlCommand::RejectChange(change_id) => self.reject_change(change_id).await,
            ControlCommand::SetMode(mode) => self.set_mode(mode).await,
            ControlCommand::SetSafetyLevel(level) => self.set_safety_level(level).await,
            ControlCommand::EnableComponent(component) => {
                self.coordinator.enable_component(component).await
            }
            ControlCommand::DisableComponent(component) => {
                self.coordinator.disable_component(component).await
            }
            ControlCommand::SetMaxChangesPerDay(limit) => self.set_max_changes_per_day(limit).await,
        }
    }

    async fn set_mode(&self, mode: DevelopmentMode) -> Result<()> {
        let mut config = self.config.write().await;
        config.mode = mode.clone();
        config.auto_approve = matches!(mode, DevelopmentMode::FullyAutonomous);
        config.components.synthesis = matches!(
            mode,
            DevelopmentMode::Assisted
                | DevelopmentMode::SemiAutonomous
                | DevelopmentMode::FullyAutonomous
        );
        config.components.testing =
            matches!(mode, DevelopmentMode::SemiAutonomous | DevelopmentMode::FullyAutonomous);
        config.components.deployment = matches!(mode, DevelopmentMode::FullyAutonomous);
        drop(config);

        let config_clone = self.config.read().await.clone();
        self.coordinator.update_configuration(config_clone).await;
        info!("Development mode set to {:?}", mode);
        Ok(())
    }

    async fn set_safety_level(&self, level: super::SafetyLevel) -> Result<()> {
        {
            let mut config = self.config.write().await;
            config.safety_level = level.clone();
        }

        self.safety_monitor.set_safety_level(level).await;
        info!("Safety level updated");
        Ok(())
    }

    async fn set_max_changes_per_day(&self, limit: usize) -> Result<()> {
        if limit == 0 {
            return Err(SelfDevError::Configuration(
                "Max changes per day must be greater than zero".to_string(),
            ));
        }

        {
            let mut config = self.config.write().await;
            config.max_changes_per_day = limit;
        }

        self.coordinator.set_max_changes_per_day(limit).await;
        info!("Max changes per day updated to {}", limit);
        Ok(())
    }

    async fn run_development_loop(&self) -> Result<()> {
        info!("Development loop started");

        let mut shutdown_rx = self.shutdown_rx.lock().await;

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("Shutdown signal received");
                    break;
                }
                command = self.operator_interface.receive_command() => {
                    if let Some((ticket, command)) = command {
                        let result = self.apply_control_command(command.clone()).await;
                        match result {
                            Ok(_) => {
                                self.operator_interface.finalize_command(ticket, super::control::CommandResult::Success).await;
                            }
                            Err(err) => {
                                self.operator_interface.finalize_command(ticket, super::control::CommandResult::Failure(err.to_string())).await;
                            }
                        }
                    }
                }
                _ = self.execute_development_cycle() => {
                    debug!("Development cycle completed");
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }

        info!("Development loop ended");
        Ok(())
    }

    async fn execute_development_cycle(&self) -> Result<()> {
        let state_machine = self.state_machine.lock().await;

        if state_machine.is_paused() {
            debug!("Development paused, skipping cycle");
            return Ok(());
        }

        let current_state = state_machine.current_state();
        drop(state_machine);

        match current_state {
            DevelopmentState::Idle => {
                self.check_for_work().await?;
            }
            DevelopmentState::Analyzing => {
                self.analyze_requirements().await?;
            }
            DevelopmentState::Planning => {
                self.create_plan().await?;
            }
            DevelopmentState::Developing => {
                self.generate_solution().await?;
            }
            DevelopmentState::Testing => {
                self.test_changes().await?;
            }
            DevelopmentState::Reviewing => {
                self.review_safety().await?;
            }
            DevelopmentState::Deploying => {
                self.deploy_changes().await?;
            }
            DevelopmentState::Monitoring => {
                self.monitor_effects().await?;
            }
            DevelopmentState::Learning => {
                self.extract_patterns().await?;
            }
        }

        Ok(())
    }

    async fn check_for_work(&self) -> Result<()> {
        if self.coordinator.has_pending_work().await {
            self.transition_state(DevelopmentState::Analyzing).await?;
        }
        Ok(())
    }

    async fn analyze_requirements(&self) -> Result<()> {
        self.coordinator.analyze_requirements().await?;
        self.transition_state(DevelopmentState::Planning).await
    }

    async fn create_plan(&self) -> Result<()> {
        self.coordinator.create_implementation_plan().await?;
        self.transition_state(DevelopmentState::Developing).await
    }

    async fn generate_solution(&self) -> Result<()> {
        self.coordinator.generate_solution().await?;
        self.transition_state(DevelopmentState::Testing).await
    }

    async fn test_changes(&self) -> Result<()> {
        let test_results = self.coordinator.test_solution().await?;

        if test_results.all_passed() {
            self.transition_state(DevelopmentState::Reviewing).await
        } else {
            warn!("Tests failed, returning to development ({} failures)", test_results.failed());
            self.transition_state(DevelopmentState::Developing).await
        }
    }

    async fn review_safety(&self) -> Result<()> {
        let changes = self.coordinator.get_pending_changes().await?;

        let mut all_safe = true;
        for change in changes.iter().filter(|c| c.status >= ChangeStatus::ReadyForReview) {
            if !self.safety_monitor.validate_change(change).await? {
                warn!("Change {} failed safety validation", change.id);
                all_safe = false;
                self.coordinator.flag_safety_failure(&change.id).await;
            }
        }

        if all_safe {
            let config = self.config.read().await;
            if config.auto_approve {
                for change in changes.into_iter().filter(|c| {
                    c.status >= ChangeStatus::ReadyForReview && c.status < ChangeStatus::Approved
                }) {
                    self.coordinator.approve_change(change.id).await?;
                }
                self.transition_state(DevelopmentState::Deploying).await
            } else {
                info!("Changes require manual approval");
                self.transition_state(DevelopmentState::Idle).await
            }
        } else {
            warn!("Safety validation failed, returning to planning");
            self.transition_state(DevelopmentState::Planning).await
        }
    }

    async fn deploy_changes(&self) -> Result<()> {
        self.coordinator.deploy_approved_changes().await?;
        self.transition_state(DevelopmentState::Monitoring).await
    }

    async fn monitor_effects(&self) -> Result<()> {
        self.coordinator.monitor_deployment().await?;
        self.transition_state(DevelopmentState::Learning).await
    }

    async fn extract_patterns(&self) -> Result<()> {
        self.coordinator.extract_learning_patterns().await?;
        self.transition_state(DevelopmentState::Idle).await
    }

    async fn transition_state(&self, new_state: DevelopmentState) -> Result<()> {
        let mut state_machine = self.state_machine.lock().await;
        state_machine.transition_to(new_state)?;
        Ok(())
    }

    async fn clone_internal(&self) -> Self {
        Self {
            config: self.config.clone(),
            state_machine: self.state_machine.clone(),
            coordinator: self.coordinator.clone(),
            safety_monitor: self.safety_monitor.clone(),
            operator_interface: self.operator_interface.clone(),
            shutdown_tx: self.shutdown_tx.clone(),
            shutdown_rx: self.shutdown_rx.clone(),
            project_root: self.project_root.clone(),
        }
    }

    pub async fn execute_task(&self, task_description: &str) -> Result<()> {
        info!("Executing task: {}", task_description);

        self.transition_state(DevelopmentState::Analyzing).await?;

        let mut spec = Specification {
            source: PathBuf::from("manual_task"),
            requirements: vec![],
            apis: vec![],
            data_models: vec![],
            behaviors: vec![],
            examples: vec![],
            constraints: vec![],
        };

        if let Some(path_str) = extract_file_reference(task_description) {
            let path = Path::new(&path_str);
            if path.exists() {
                info!("Found referenced file: {}", path.display());
                let parser = SpecParser::new();
                spec = parser
                    .parse_file(path)
                    .await
                    .map_err(|e| SelfDevError::Orchestration(e.to_string()))?;
            }
        }

        if spec.requirements.is_empty() {
            let requirement = Requirement {
                id: format!("task_{}", chrono::Utc::now().timestamp()),
                description: task_description.to_string(),
                category: RequirementType::Functional,
                priority: Priority::High,
                acceptance_criteria: vec![],
                source_location: SourceLocation::default(),
                related: vec![],
                tags: vec!["manual".to_string()],
            };
            spec.requirements.push(requirement);
        }

        self.transition_state(DevelopmentState::Planning).await?;

        use crate::synthesis::{SynthesisConfig, SynthesisEngine};
        let synthesis_config = SynthesisConfig::default();
        let mut engine = SynthesisEngine::new(synthesis_config)
            .map_err(|e| SelfDevError::Orchestration(e.to_string()))?;

        info!("Synthesizing implementation for {} requirements...", spec.requirements.len());

        self.transition_state(DevelopmentState::Developing).await?;

        let result = engine
            .synthesize(&spec)
            .await
            .map_err(|e| SelfDevError::Orchestration(e.to_string()))?;

        self.transition_state(DevelopmentState::Testing).await?;

        use crate::safety::{CodeModification, ModificationType, SafetyGatekeeper};
        let safety_config = crate::safety::SafetyConfig::default();
        let safety_gate = SafetyGatekeeper::new(safety_config)
            .map_err(|e| SelfDevError::Orchestration(e.to_string()))?;

        let mut applied_count = 0;
        for generated_file in &result.files_generated {
            let modification = CodeModification {
                file_path: generated_file.clone(),
                original: String::new(),
                modified: String::new(),
                modification_type: if generated_file.exists() {
                    ModificationType::Update
                } else {
                    ModificationType::Create
                },
                reason: "Generated by synthesis".to_string(),
                prp_reference: None,
            };

            self.transition_state(DevelopmentState::Reviewing).await?;

            if safety_gate
                .validate(&modification)
                .await
                .map_err(|e| SelfDevError::SafetyViolation(e.to_string()))?
                .passed
            {
                self.transition_state(DevelopmentState::Deploying).await?;

                info!("Writing generated file: {}", generated_file.display());

                if let Some(parent) = generated_file.parent() {
                    tokio::fs::create_dir_all(parent).await.ok();
                }

                tokio::fs::write(&generated_file, "")
                    .await
                    .map_err(|e| SelfDevError::Orchestration(e.to_string()))?;
                applied_count += 1;
            } else {
                warn!("Safety validation failed for: {}", generated_file.display());
            }
        }

        info!("Applied {} of {} generated files", applied_count, result.files_generated.len());

        self.transition_state(DevelopmentState::Idle).await?;

        info!("Task execution completed");
        Ok(())
    }
}

/// Extract file reference from task description
fn extract_file_reference(task_description: &str) -> Option<String> {
    let patterns = [
        r"(?i)(?:implement|parse|process|read|analyze)\s+([^\s]+\.[a-z]+)",
        r"(?i)from\s+([^\s]+\.[a-z]+)",
        r"([^\s]+\.[a-z]+)",
    ];

    for pattern in &patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(captures) = re.captures(task_description) {
                if let Some(path) = captures.get(1) {
                    return Some(path.as_str().to_string());
                }
            }
        }
    }

    None
}

#[derive(Debug, Clone)]
pub struct SelfDevStatus {
    pub current_state: DevelopmentState,
    pub is_paused: bool,
    pub mode: super::DevelopmentMode,
    pub safety_level: super::SafetyLevel,
    pub active_components: Vec<String>,
    pub pending_changes: Vec<PendingChange>,
    pub today_changes: usize,
}
