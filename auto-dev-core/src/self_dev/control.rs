#![allow(unused)]
//! Operator control interface for managing self-development

use super::{Result, SelfDevError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tracing::{debug, info, warn};
use uuid::Uuid;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandTicket(Uuid);

impl CommandTicket {
    fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn id(&self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResult {
    Pending,
    Success,
    Failure(String),
}

struct AuditEntry {
    id: CommandTicket,
    timestamp: std::time::SystemTime,
    command: ControlCommand,
    operator: Option<String>,
    result: CommandResult,
}

pub struct OperatorInterface {
    command_tx: mpsc::Sender<(CommandTicket, ControlCommand)>,
    command_rx: Arc<Mutex<mpsc::Receiver<(CommandTicket, ControlCommand)>>>,
    audit_log: Arc<Mutex<Vec<AuditEntry>>>,
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

    pub async fn handle_command(&self, command: ControlCommand) -> Result<CommandTicket> {
        if !Self::validate_command(&command) {
            return Err(SelfDevError::Control("Invalid control command".to_string()));
        }

        info!("Handling operator command: {:?}", command);
        let ticket = CommandTicket::new();
        self.log_command(ticket, command.clone(), CommandResult::Pending).await;
        Ok(ticket)
    }

    pub async fn finalize_command(&self, ticket: CommandTicket, result: CommandResult) {
        let mut log = self.audit_log.lock().await;
        if let Some(entry) = log.iter_mut().find(|entry| entry.id == ticket) {
            entry.result = result;
            entry.timestamp = std::time::SystemTime::now();
        }
    }

    pub async fn send_command(&self, command: ControlCommand) -> Result<CommandTicket> {
        if !Self::validate_command(&command) {
            return Err(SelfDevError::Control("Invalid control command".to_string()));
        }

        let ticket = CommandTicket::new();
        self.log_command(ticket, command.clone(), CommandResult::Pending).await;
        self.command_tx
            .send((ticket, command))
            .await
            .map_err(|e| SelfDevError::Control(format!("Failed to send command: {}", e)))?;
        Ok(ticket)
    }

    pub async fn receive_command(&self) -> Option<(CommandTicket, ControlCommand)> {
        self.command_rx.lock().await.recv().await
    }

    async fn log_command(
        &self,
        ticket: CommandTicket,
        command: ControlCommand,
        result: CommandResult,
    ) {
        let entry = AuditEntry {
            id: ticket,
            timestamp: std::time::SystemTime::now(),
            command,
            operator: None,
            result,
        };

        self.audit_log.lock().await.push(entry);
    }

    pub async fn get_audit_log(&self) -> Vec<(std::time::SystemTime, String, CommandResult)> {
        self.audit_log
            .lock()
            .await
            .iter()
            .map(|entry| (entry.timestamp, format!("{:?}", entry.command), entry.result.clone()))
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

        let ticket = interface.handle_command(ControlCommand::Start).await.unwrap();
        interface.finalize_command(ticket, CommandResult::Success).await;

        let ticket = interface.handle_command(ControlCommand::Pause).await.unwrap();
        interface.finalize_command(ticket, CommandResult::Success).await;

        let ticket = interface.handle_command(ControlCommand::Resume).await.unwrap();
        interface.finalize_command(ticket, CommandResult::Success).await;

        let ticket = interface.handle_command(ControlCommand::Stop).await.unwrap();
        interface.finalize_command(ticket, CommandResult::Success).await;

        let audit_log = interface.get_audit_log().await;
        assert_eq!(audit_log.len(), 4);
    }

    #[tokio::test]
    async fn test_command_validation() {
        assert!(OperatorInterface::validate_command(&ControlCommand::Start));
        assert!(OperatorInterface::validate_command(&ControlCommand::SetMaxChangesPerDay(10)));
        assert!(!OperatorInterface::validate_command(&ControlCommand::SetMaxChangesPerDay(0)));
        assert!(!OperatorInterface::validate_command(&ControlCommand::SetMaxChangesPerDay(10000)));
    }

    #[tokio::test]
    async fn test_send_receive_command() {
        let interface = OperatorInterface::new();

        let ticket = interface.send_command(ControlCommand::GetStatus).await.unwrap();

        let received = interface.receive_command().await;
        assert!(received.is_some());
        let (rx_ticket, command) = received.unwrap();
        assert_eq!(command, ControlCommand::GetStatus);
        assert_eq!(ticket.id(), rx_ticket.id());
    }
}
