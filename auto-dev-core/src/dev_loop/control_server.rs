//! Control server for IPC communication with the development loop

use super::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Control server for handling IPC commands
pub struct ControlServer {
    port: u16,
    shutdown_tx: mpsc::Sender<()>,
    command_tx: mpsc::Sender<ControlCommand>,
}

impl ControlServer {
    /// Create a new control server
    pub fn new(
        port: u16,
        shutdown_tx: mpsc::Sender<()>,
        command_tx: mpsc::Sender<ControlCommand>,
    ) -> Self {
        Self { port, shutdown_tx, command_tx }
    }

    /// Start the control server
    pub async fn start(self) -> Result<()> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        info!("Control server listening on {}", addr);

        // Save port to file for client discovery (create .auto-dev/loop as needed)
        let port_dir = PathBuf::from(".auto-dev/loop");
        if !port_dir.exists() {
            if let Err(e) = tokio::fs::create_dir_all(&port_dir).await {
                warn!("Could not create .auto-dev/loop directory: {} (continuing anyway)", e);
            }
        }

        let port_file = port_dir.join("control.port");

        // Try to write port file, but don't fail if we can't
        if let Err(e) = tokio::fs::write(&port_file, self.port.to_string()).await {
            warn!("Could not write port file: {} (continuing anyway)", e);
        }

        loop {
            tokio::select! {
                Ok((stream, addr)) = listener.accept() => {
                    debug!("Control connection from {}", addr);
                    let shutdown_tx = self.shutdown_tx.clone();
                    let command_tx = self.command_tx.clone();

                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, shutdown_tx, command_tx).await {
                            error!("Error handling control connection: {}", e);
                        }
                    });
                }
            }
        }
    }
}

/// Handle a control connection
async fn handle_connection(
    mut stream: TcpStream,
    shutdown_tx: mpsc::Sender<()>,
    command_tx: mpsc::Sender<ControlCommand>,
) -> Result<()> {
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    while reader.read_line(&mut line).await? > 0 {
        let request: ControlRequest = serde_json::from_str(&line)?;
        debug!("Received control request: {:?}", request);

        let response = match request {
            ControlRequest::Shutdown => {
                info!("Shutdown requested via control server");
                shutdown_tx.send(()).await.ok();
                ControlResponse::Success("Shutdown initiated".to_string())
            }
            ControlRequest::Status => {
                // Get status from orchestrator
                ControlResponse::Status(LoopStatus {
                    state: "running".to_string(),
                    uptime_seconds: 0, // Would calculate actual uptime
                    events_processed: 0,
                })
            }
            ControlRequest::QueueEvent(event) => {
                command_tx.send(ControlCommand::QueueEvent(event)).await?;
                ControlResponse::Success("Event queued".to_string())
            }
            ControlRequest::GetMetrics => {
                // Get metrics from orchestrator
                ControlResponse::Metrics(LoopMetrics::default())
            }
            ControlRequest::Ping => ControlResponse::Pong,
        };

        let response_json = serde_json::to_string(&response)? + "\n";
        writer.write_all(response_json.as_bytes()).await?;
        writer.flush().await?;

        line.clear();
    }

    Ok(())
}

/// Control request from client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlRequest {
    Shutdown,
    Status,
    QueueEvent(Event),
    GetMetrics,
    Ping,
}

/// Control response to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlResponse {
    Success(String),
    Error(String),
    Status(LoopStatus),
    Metrics(LoopMetrics),
    Pong,
}

/// Loop status for IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopStatus {
    pub state: String,
    pub uptime_seconds: u64,
    pub events_processed: u64,
}

/// Control command for internal communication
#[derive(Debug, Clone)]
pub enum ControlCommand {
    QueueEvent(Event),
    GetStatus,
    GetMetrics,
}

/// Control client for sending commands to the server
pub struct ControlClient {
    port: Option<u16>,
}

impl ControlClient {
    /// Create a new control client
    pub fn new() -> Self {
        Self { port: None }
    }

    /// Discover the control server port
    async fn discover_port(&mut self) -> Result<u16> {
        if let Some(port) = self.port {
            return Ok(port);
        }

        // Check for port file in .auto-dev/loop
        let port_file = PathBuf::from(".auto-dev/loop/control.port");

        if port_file.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&port_file).await {
                if let Ok(port) = content.trim().parse::<u16>() {
                    self.port = Some(port);
                    return Ok(port);
                }
            }
        }

        Err(anyhow::anyhow!("Control server not running (port file not found)"))
    }

    /// Send a control request
    pub async fn send_request(&mut self, request: ControlRequest) -> Result<ControlResponse> {
        let port = self.discover_port().await?;
        let addr = format!("127.0.0.1:{}", port);

        let mut stream = TcpStream::connect(&addr).await?;

        // Send request
        let request_json = serde_json::to_string(&request)? + "\n";
        stream.write_all(request_json.as_bytes()).await?;
        stream.flush().await?;

        // Read response
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        let response: ControlResponse = serde_json::from_str(&line)?;
        Ok(response)
    }

    /// Send shutdown command
    pub async fn shutdown(&mut self) -> Result<()> {
        match self.send_request(ControlRequest::Shutdown).await {
            Ok(ControlResponse::Success(msg)) => {
                info!("Shutdown response: {}", msg);
                Ok(())
            }
            Ok(ControlResponse::Error(err)) => Err(anyhow::anyhow!("Shutdown failed: {}", err)),
            Ok(_) => Err(anyhow::anyhow!("Unexpected response")),
            Err(e) => Err(e),
        }
    }

    /// Get loop status
    pub async fn get_status(&mut self) -> Result<LoopStatus> {
        match self.send_request(ControlRequest::Status).await {
            Ok(ControlResponse::Status(status)) => Ok(status),
            Ok(ControlResponse::Error(err)) => Err(anyhow::anyhow!("Status failed: {}", err)),
            Ok(_) => Err(anyhow::anyhow!("Unexpected response")),
            Err(e) => Err(e),
        }
    }

    /// Get loop metrics
    pub async fn get_metrics(&mut self) -> Result<LoopMetrics> {
        match self.send_request(ControlRequest::GetMetrics).await {
            Ok(ControlResponse::Metrics(metrics)) => Ok(metrics),
            Ok(ControlResponse::Error(err)) => Err(anyhow::anyhow!("Metrics failed: {}", err)),
            Ok(_) => Err(anyhow::anyhow!("Unexpected response")),
            Err(e) => Err(e),
        }
    }

    /// Queue an event
    pub async fn queue_event(&mut self, event: Event) -> Result<()> {
        match self.send_request(ControlRequest::QueueEvent(event)).await {
            Ok(ControlResponse::Success(msg)) => {
                debug!("Event queued: {}", msg);
                Ok(())
            }
            Ok(ControlResponse::Error(err)) => Err(anyhow::anyhow!("Queue event failed: {}", err)),
            Ok(_) => Err(anyhow::anyhow!("Unexpected response")),
            Err(e) => Err(e),
        }
    }

    /// Check if server is running
    pub async fn ping(&mut self) -> Result<bool> {
        match self.send_request(ControlRequest::Ping).await {
            Ok(ControlResponse::Pong) => Ok(true),
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_control_client() {
        let mut client = ControlClient::new();
        // Would test against a running server
    }
}
