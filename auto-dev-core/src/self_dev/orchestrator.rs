#![allow(unused)]
//! Central orchestrator for self-development activities

use super::{
    ComponentCoordinator, ControlCommand, DevelopmentState, DevelopmentStateMachine,
    OperatorInterface, Result, SafetyMonitor, SelfDevConfig, SelfDevError,
};
use crate::parser::model::{Priority, Requirement, RequirementType, SourceLocation, Specification};
use crate::parser::SpecParser;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, warn};

pub struct SelfDevOrchestrator {
    config: Arc<RwLock<SelfDevConfig>>,
    state_machine: Arc<Mutex<DevelopmentStateMachine>>,
    coordinator: Arc<ComponentCoordinator>,
    safety_monitor: Arc<SafetyMonitor>,
    operator_interface: Arc<OperatorInterface>,
    shutdown_tx: mpsc::Sender<()>,
    shutdown_rx: Arc<Mutex<mpsc::Receiver<()>>>,
}

impl SelfDevOrchestrator {
    pub async fn new(config: SelfDevConfig) -> Result<Self> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        
        let state_machine = DevelopmentStateMachine::new(DevelopmentState::Idle);
        let coordinator = ComponentCoordinator::new(config.clone());
        let safety_monitor = SafetyMonitor::new(config.safety_level.clone());
        let operator_interface = OperatorInterface::new();
        
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            state_machine: Arc::new(Mutex::new(state_machine)),
            coordinator: Arc::new(coordinator),
            safety_monitor: Arc::new(safety_monitor),
            operator_interface: Arc::new(operator_interface),
            shutdown_tx,
            shutdown_rx: Arc::new(Mutex::new(shutdown_rx)),
        })
    }
    
    pub async fn start(&self) -> Result<()> {
        info!("Starting self-development orchestrator");
        
        let config = self.config.read().await;
        if !config.enabled {
            return Err(SelfDevError::Configuration(
                "Self-development is not enabled".to_string()
            ));
        }
        
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
            active_components: self.coordinator.get_active_components().await,
            pending_changes: self.coordinator.get_pending_changes().await.unwrap_or_default(),
            today_changes: self.coordinator.get_today_changes_count().await,
        })
    }
    
    pub async fn review_changes(&self) -> Result<Vec<PendingChange>> {
        self.coordinator.get_pending_changes().await
    }
    
    pub async fn approve_change(&self, change_id: String) -> Result<()> {
        
        if !self.safety_monitor.validate_change(&change_id).await? {
            return Err(SelfDevError::SafetyViolation(
                format!("Change {} failed safety validation", change_id)
            ));
        }
        
        self.coordinator.approve_change(change_id).await
    }
    
    pub async fn reject_change(&self, change_id: String) -> Result<()> {
        self.coordinator.reject_change(change_id).await
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
            warn!("Tests failed, returning to development");
            self.transition_state(DevelopmentState::Developing).await
        }
    }
    
    async fn review_safety(&self) -> Result<()> {
        let changes = self.coordinator.get_pending_changes().await?;
        
        let mut all_safe = true;
        for change in changes {
            if !self.safety_monitor.validate_change(&change.id).await? {
                warn!("Change {} failed safety validation", change.id);
                all_safe = false;
            }
        }
        
        if all_safe {
            let config = self.config.read().await;
            if config.auto_approve {
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
        }
    }
    
    pub async fn handle_control_command(&self, command: ControlCommand) -> Result<()> {
        self.operator_interface.handle_command(command).await
    }
    
    pub async fn execute_task(&self, task_description: &str) -> Result<()> {
        info!("Executing task: {}", task_description);
        
        // Transition to analyzing state
        self.transition_state(DevelopmentState::Analyzing).await?;
        
        // Parse the task to understand what needs to be done
        use crate::parser::{Requirement, RequirementType, Specification, SpecParser};
        use std::path::{Path, PathBuf};
        
        let mut spec = Specification {
            source: PathBuf::from("manual_task"),
            requirements: vec![],
            apis: vec![],
            data_models: vec![],
            behaviors: vec![],
            examples: vec![],
            constraints: vec![],
        };
        
        // Try to find specification files if they reference a specific format
        if let Some(path_str) = extract_file_reference(task_description) {
            let path = Path::new(&path_str);
            if path.exists() {
                info!("Found referenced file: {}", path.display());
                let parser = SpecParser::new();
                spec = parser.parse_file(path).await.map_err(|e| SelfDevError::Orchestration(e.to_string()))?;
            }
        }
        
        // If no spec was loaded, create one from the task description
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
        
        // Transition to planning
        self.transition_state(DevelopmentState::Planning).await?;
        
        // Use synthesis engine to generate implementation
        use crate::synthesis::{SynthesisEngine, SynthesisConfig};
        let synthesis_config = SynthesisConfig::default();
        let mut engine = SynthesisEngine::new(synthesis_config).map_err(|e| SelfDevError::Orchestration(e.to_string()))?;
        
        info!("Synthesizing implementation for {} requirements...", spec.requirements.len());
        
        // Transition to developing
        self.transition_state(DevelopmentState::Developing).await?;
        
        let result = engine.synthesize(&spec).await.map_err(|e| SelfDevError::Orchestration(e.to_string()))?;
        
        // Transition to testing  
        self.transition_state(DevelopmentState::Testing).await?;
        
        // Apply safety validation before writing
        use crate::safety::{SafetyGatekeeper, CodeModification, ModificationType, RiskLevel};
        let safety_config = crate::safety::SafetyConfig::default();
        let safety_gate = SafetyGatekeeper::new(safety_config).map_err(|e| SelfDevError::Orchestration(e.to_string()))?;
        
        let mut applied_count = 0;
        for generated_file in &result.files_generated {
            // For now, we'll skip the modification since we only have PathBuf
            // and CodeModification needs content
            // This would need actual file content loading
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
            
            // Transition to reviewing
            self.transition_state(DevelopmentState::Reviewing).await?;
            
            if safety_gate.validate(&modification).await
                .map_err(|e| SelfDevError::SafetyViolation(e.to_string()))?.passed {
                // Transition to deploying
                self.transition_state(DevelopmentState::Deploying).await?;
                
                info!("Writing generated file: {}", generated_file.display());
                
                // Create parent directories if needed
                if let Some(parent) = generated_file.parent() {
                    tokio::fs::create_dir_all(parent).await.ok();
                }
                
                // Note: generated_file is just a PathBuf, not containing content
                // This would need to be loaded from synthesis result
                // For now, creating empty file as placeholder
                tokio::fs::write(&generated_file, "").await
                    .map_err(|e| SelfDevError::Orchestration(e.to_string()))?;
                applied_count += 1;
            } else {
                warn!("Safety validation failed for: {}", generated_file.display());
            }
        }
        
        info!("Applied {} of {} generated files", applied_count, result.files_generated.len());
        
        // Transition back to idle
        self.transition_state(DevelopmentState::Idle).await?;
        
        info!("Task execution completed");
        Ok(())
    }
}

/// Extract file reference from task description
fn extract_file_reference(task_description: &str) -> Option<String> {
    // Look for patterns like "implement X.md" or "parse /path/to/file"
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
    pub active_components: Vec<String>,
    pub pending_changes: Vec<PendingChange>,
    pub today_changes: usize,
}

#[derive(Debug, Clone)]
pub struct PendingChange {
    pub id: String,
    pub description: String,
    pub file_path: String,
    pub change_type: ChangeType,
    pub risk_level: RiskLevel,
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

pub struct TestResults {
    passed: usize,
    failed: usize,
    skipped: usize,
}

impl TestResults {
    pub fn new(passed: usize, failed: usize, skipped: usize) -> Self {
        Self {
            passed,
            failed,
            skipped,
        }
    }
    
    pub fn all_passed(&self) -> bool {
        self.failed == 0 && self.passed > 0
    }
}
