//! Operator control interface for managing self-development

use super::{Result, SelfDevError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlCommand {
    Start,
    Stop,
    Pause,
    Resume,
    EmergencyStop,
    GetStatus,
    ReviewChanges,
    ApproveChange(String),
    RejectChange(String),
    SetMode(super::DevelopmentMode),
    SetSafetyLevel(super::SafetyLevel),
    EnableComponent(String),
    DisableComponent(String),
    SetMaxChangesPerDay(usize),
}

pub struct OperatorInterface {
    command_tx: mpsc::Sender<ControlCommand>,
    command_rx: Arc<Mutex<mpsc::Receiver<ControlCommand>>>,
    audit_log: Arc<Mutex<Vec<AuditEntry>>>,
}

#[derive(Debug, Clone)]
struct AuditEntry {
    timestamp: std::time::SystemTime,
    command: ControlCommand,
    operator: Option<String>,
    result: CommandResult,
}

#[derive(Debug, Clone)]
enum CommandResult {
    Success,
    Failure(String),
    Pending,
}

impl OperatorInterface {
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::channel(100);
        
        Self {
            command_tx,
            command_rx: Arc::new(Mutex::new(command_rx)),
            audit_log: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub async fn handle_command(&self, command: ControlCommand) -> Result<()> {
        info!("Handling operator command: {:?}", command);
        
        self.log_command(command.clone(), CommandResult::Pending).await;
        
        match &command {
            ControlCommand::Start => {
                info!("Starting self-development via operator command");
                self.log_command(command, CommandResult::Success).await;
                Ok(())
            }
            ControlCommand::Stop => {
                info!("Stopping self-development via operator command");
                self.log_command(command, CommandResult::Success).await;
                Ok(())
            }
            ControlCommand::Pause => {
                info!("Pausing self-development via operator command");
                self.log_command(command, CommandResult::Success).await;
                Ok(())
            }
            ControlCommand::Resume => {
                info!("Resuming self-development via operator command");
                self.log_command(command, CommandResult::Success).await;
                Ok(())
            }
            ControlCommand::EmergencyStop => {
                warn!("Emergency stop triggered via operator command");
                self.log_command(command, CommandResult::Success).await;
                Ok(())
            }
            ControlCommand::GetStatus => {
                debug!("Status request via operator command");
                self.log_command(command, CommandResult::Success).await;
                Ok(())
            }
            ControlCommand::ReviewChanges => {
                debug!("Review changes request via operator command");
                self.log_command(command, CommandResult::Success).await;
                Ok(())
            }
            ControlCommand::ApproveChange(id) => {
                info!("Approving change {} via operator command", id);
                self.log_command(command, CommandResult::Success).await;
                Ok(())
            }
            ControlCommand::RejectChange(id) => {
                info!("Rejecting change {} via operator command", id);
                self.log_command(command, CommandResult::Success).await;
                Ok(())
            }
            ControlCommand::SetMode(mode) => {
                info!("Setting development mode to {:?} via operator command", mode);
                self.log_command(command, CommandResult::Success).await;
                Ok(())
            }
            ControlCommand::SetSafetyLevel(level) => {
                info!("Setting safety level to {:?} via operator command", level);
                self.log_command(command, CommandResult::Success).await;
                Ok(())
            }
            ControlCommand::EnableComponent(component) => {
                info!("Enabling component {} via operator command", component);
                self.log_command(command, CommandResult::Success).await;
                Ok(())
            }
            ControlCommand::DisableComponent(component) => {
                info!("Disabling component {} via operator command", component);
                self.log_command(command, CommandResult::Success).await;
                Ok(())
            }
            ControlCommand::SetMaxChangesPerDay(limit) => {
                info!("Setting max changes per day to {} via operator command", limit);
                self.log_command(command, CommandResult::Success).await;
                Ok(())
            }
        }
    }
    
    pub async fn send_command(&self, command: ControlCommand) -> Result<()> {
        self.command_tx.send(command).await
            .map_err(|e| SelfDevError::Control(format!("Failed to send command: {}", e)))
    }
    
    pub async fn receive_command(&self) -> Option<ControlCommand> {
        self.command_rx.lock().await.recv().await
    }
    
    async fn log_command(&self, command: ControlCommand, result: CommandResult) {
        let entry = AuditEntry {
            timestamp: std::time::SystemTime::now(),
            command,
            operator: None,
            result,
        };
        
        self.audit_log.lock().await.push(entry);
    }
    
    pub async fn get_audit_log(&self) -> Vec<(std::time::SystemTime, String)> {
        self.audit_log
            .lock()
            .await
            .iter()
            .map(|entry| (entry.timestamp, format!("{:?}", entry.command)))
            .collect()
    }
    
    pub async fn clear_audit_log(&self) {
        self.audit_log.lock().await.clear();
    }
    
    pub fn validate_command(command: &ControlCommand) -> bool {
        match command {
            ControlCommand::SetMaxChangesPerDay(limit) => *limit > 0 && *limit <= 1000,
            _ => true,
        }
    }
}

impl Default for OperatorInterface {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_operator_interface_creation() {
        let interface = OperatorInterface::new();
        assert!(interface.get_audit_log().await.is_empty());
    }
    
    #[tokio::test]
    async fn test_command_handling() {
        let interface = OperatorInterface::new();
        
        let result = interface.handle_command(ControlCommand::Start).await;
        assert!(result.is_ok());
        
        let result = interface.handle_command(ControlCommand::Pause).await;
        assert!(result.is_ok());
        
        let result = interface.handle_command(ControlCommand::Resume).await;
        assert!(result.is_ok());
        
        let result = interface.handle_command(ControlCommand::Stop).await;
        assert!(result.is_ok());
        
        let audit_log = interface.get_audit_log().await;
        assert_eq!(audit_log.len(), 8);
    }
    
    #[tokio::test]
    async fn test_command_validation() {
        assert!(OperatorInterface::validate_command(&ControlCommand::Start));
        assert!(OperatorInterface::validate_command(
            &ControlCommand::SetMaxChangesPerDay(10)
        ));
        assert!(!OperatorInterface::validate_command(
            &ControlCommand::SetMaxChangesPerDay(0)
        ));
        assert!(!OperatorInterface::validate_command(
            &ControlCommand::SetMaxChangesPerDay(10000)
        ));
    }
    
    #[tokio::test]
    async fn test_send_receive_command() {
        let interface = OperatorInterface::new();
        
        interface.send_command(ControlCommand::GetStatus).await.unwrap();
        
        let received = interface.receive_command().await;
        assert!(received.is_some());
        assert!(matches!(received.unwrap(), ControlCommand::GetStatus));
    }
}