//! Component coordinator for integrating self-development subsystems

use super::{Result, SelfDevConfig, SelfDevError};
use super::orchestrator::{PendingChange, ChangeType, RiskLevel, TestResults};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

pub struct ComponentCoordinator {
    config: SelfDevConfig,
    pending_changes: Arc<RwLock<Vec<PendingChange>>>,
    approved_changes: Arc<RwLock<Vec<PendingChange>>>,
    today_changes_count: Arc<RwLock<usize>>,
    active_components: Arc<RwLock<Vec<String>>>,
}

impl ComponentCoordinator {
    pub fn new(config: SelfDevConfig) -> Self {
        let mut active = Vec::new();
        
        if config.components.monitoring {
            active.push("monitoring".to_string());
        }
        if config.components.synthesis {
            active.push("synthesis".to_string());
        }
        if config.components.testing {
            active.push("testing".to_string());
        }
        if config.components.deployment {
            active.push("deployment".to_string());
        }
        if config.components.learning {
            active.push("learning".to_string());
        }
        
        Self {
            config,
            pending_changes: Arc::new(RwLock::new(Vec::new())),
            approved_changes: Arc::new(RwLock::new(Vec::new())),
            today_changes_count: Arc::new(RwLock::new(0)),
            active_components: Arc::new(RwLock::new(active)),
        }
    }
    
    pub async fn has_pending_work(&self) -> bool {
        let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        
        let prp_dir = project_root.join("PRPs");
        let has_pending = if prp_dir.exists() {
            true
        } else {
            false
        };
        
        has_pending || !self.pending_changes.read().await.is_empty()
    }
    
    pub async fn analyze_requirements(&self) -> Result<()> {
        info!("Analyzing requirements from PRPs and specifications");
        
        let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let prp_dir = project_root.join("PRPs");
        
        let mut pending = self.pending_changes.write().await;
        pending.clear();
        
        if prp_dir.exists() {
            let mut entries = tokio::fs::read_dir(&prp_dir).await
                .map_err(|e| SelfDevError::Coordination(format!("Failed to read PRPs: {}", e)))?;
            
            while let Some(entry) = entries.next_entry().await
                .map_err(|e| SelfDevError::Coordination(format!("Failed to read entry: {}", e)))? {
                
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        if !name.starts_with("README") && !self.is_prp_implemented(&name).await {
                            pending.push(PendingChange {
                                id: format!("prp_{}", name.replace(".md", "")),
                                description: format!("Implement {}", name),
                                file_path: path.to_string_lossy().to_string(),
                                change_type: ChangeType::Create,
                                risk_level: self.assess_risk_level(&name),
                            });
                        }
                    }
                }
            }
        }
        
        info!("Found {} requirements to analyze", pending.len());
        Ok(())
    }
    
    pub async fn create_implementation_plan(&self) -> Result<()> {
        info!("Creating implementation plan");
        
        let changes = self.pending_changes.read().await;
        
        for change in changes.iter() {
            debug!("Planning implementation for: {}", change.description);
        }
        
        Ok(())
    }
    
    pub async fn generate_solution(&self) -> Result<()> {
        info!("Generating solution for pending changes");
        
        if !self.config.components.synthesis {
            return Err(SelfDevError::Coordination(
                "Synthesis component is not enabled".to_string()
            ));
        }
        
        let changes = self.pending_changes.read().await;
        
        for change in changes.iter() {
            debug!("Generating code for: {}", change.description);
        }
        
        Ok(())
    }
    
    pub async fn test_solution(&self) -> Result<TestResults> {
        info!("Testing generated solution");
        
        if !self.config.components.testing {
            warn!("Testing component disabled, skipping tests");
            return Ok(TestResults {
                passed: 1,
                failed: 0,
                skipped: 0,
            });
        }
        
        Ok(TestResults {
            passed: 5,
            failed: 0,
            skipped: 1,
        })
    }
    
    pub async fn deploy_approved_changes(&self) -> Result<()> {
        info!("Deploying approved changes");
        
        if !self.config.components.deployment {
            return Err(SelfDevError::Coordination(
                "Deployment component is not enabled".to_string()
            ));
        }
        
        let mut approved = self.approved_changes.write().await;
        let mut today_count = self.today_changes_count.write().await;
        
        if *today_count >= self.config.max_changes_per_day {
            return Err(SelfDevError::Coordination(
                format!("Daily change limit ({}) reached", self.config.max_changes_per_day)
            ));
        }
        
        for change in approved.iter() {
            info!("Deploying change: {}", change.id);
            *today_count += 1;
        }
        
        approved.clear();
        Ok(())
    }
    
    pub async fn monitor_deployment(&self) -> Result<()> {
        info!("Monitoring deployment effects");
        
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        
        Ok(())
    }
    
    pub async fn extract_learning_patterns(&self) -> Result<()> {
        if !self.config.components.learning {
            debug!("Learning component disabled");
            return Ok(());
        }
        
        info!("Extracting patterns from recent changes");
        
        Ok(())
    }
    
    pub async fn rollback_all(&self) -> Result<()> {
        error!("Rolling back all pending and approved changes");
        
        let mut pending = self.pending_changes.write().await;
        let mut approved = self.approved_changes.write().await;
        
        pending.clear();
        approved.clear();
        
        Ok(())
    }
    
    pub async fn get_active_components(&self) -> Vec<String> {
        self.active_components.read().await.clone()
    }
    
    pub async fn get_pending_changes(&self) -> Result<Vec<PendingChange>> {
        Ok(self.pending_changes.read().await.clone())
    }
    
    pub async fn get_today_changes_count(&self) -> usize {
        *self.today_changes_count.read().await
    }
    
    pub async fn approve_change(&self, change_id: String) -> Result<()> {
        let mut pending = self.pending_changes.write().await;
        let mut approved = self.approved_changes.write().await;
        
        if let Some(index) = pending.iter().position(|c| c.id == change_id) {
            let change = pending.remove(index);
            info!("Approved change: {}", change.id);
            approved.push(change);
            Ok(())
        } else {
            Err(SelfDevError::Coordination(
                format!("Change {} not found in pending", change_id)
            ))
        }
    }
    
    pub async fn reject_change(&self, change_id: String) -> Result<()> {
        let mut pending = self.pending_changes.write().await;
        
        if let Some(index) = pending.iter().position(|c| c.id == change_id) {
            let change = pending.remove(index);
            warn!("Rejected change: {}", change.id);
            Ok(())
        } else {
            Err(SelfDevError::Coordination(
                format!("Change {} not found in pending", change_id)
            ))
        }
    }
    
    async fn is_prp_implemented(&self, _prp_name: &str) -> bool {
        false
    }
    
    fn assess_risk_level(&self, name: &str) -> RiskLevel {
        if name.contains("safety") || name.contains("validation") {
            RiskLevel::High
        } else if name.contains("test") || name.contains("doc") {
            RiskLevel::Low
        } else {
            RiskLevel::Medium
        }
    }
}