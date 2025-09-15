//! Transport implementations for MCP communication

use super::client::McpTransport;
use anyhow::{Result, Context};
use async_trait::async_trait;
use serde_json::Value;
use std::process::Stdio;
use tokio::process::{Command, Child};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use std::collections::HashMap;
use tracing::{debug, warn};

/// Standard I/O transport for subprocess MCP servers
pub struct StdioTransport {
    process: Arc<Mutex<Child>>,
    request_tx: mpsc::Sender<Request>,
    pending_requests: Arc<Mutex<HashMap<String, mpsc::Sender<Value>>>>,
}

impl StdioTransport {
    /// Create a new stdio transport by spawning a process
    pub async fn spawn(command: &str, args: Vec<String>) -> Result<Self> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn MCP server process")?;
        
        let stdin = child.stdin.take().ok_or_else(|| anyhow::anyhow!("No stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("No stdout"))?;
        
        let (request_tx, mut request_rx) = mpsc::channel::<Request>(100);
        let pending_requests = Arc::new(Mutex::new(HashMap::new()));
        
        // Spawn task to handle writing to stdin
        let pending_clone = pending_requests.clone();
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(request) = request_rx.recv().await {
                let message = serde_json::to_string(&request.message).unwrap();
                if let Err(e) = stdin.write_all(message.as_bytes()).await {
                    warn!("Failed to write to stdin: {}", e);
                    break;
                }
                if let Err(e) = stdin.write_all(b"\n").await {
                    warn!("Failed to write newline: {}", e);
                    break;
                }
                if let Err(e) = stdin.flush().await {
                    warn!("Failed to flush stdin: {}", e);
                    break;
                }
                
                // Store pending request
                if let Some(id) = request.id {
                    let mut pending = pending_clone.lock().await;
                    pending.insert(id, request.response_tx);
                }
            }
        });
        
        // Spawn task to handle reading from stdout
        let pending_clone = pending_requests.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if let Ok(response) = serde_json::from_str::<Value>(&line) {
                            // Handle response
                            if let Some(id) = response["id"].as_str() {
                                let mut pending = pending_clone.lock().await;
                                if let Some(tx) = pending.remove(id) {
                                    let _ = tx.send(response).await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Error reading from stdout: {}", e);
                        break;
                    }
                }
            }
        });
        
        Ok(Self {
            process: Arc::new(Mutex::new(child)),
            request_tx,
            pending_requests,
        })
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn send_request(
        &self,
        method: &str,
        params: Value,
    ) -> Result<Value> {
        let id = uuid::Uuid::new_v4().to_string();
        let (response_tx, mut response_rx) = mpsc::channel(1);
        
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        
        let request = Request {
            id: Some(id),
            message,
            response_tx,
        };
        
        self.request_tx.send(request).await
            .context("Failed to send request")?;
        
        // Wait for response
        let response = response_rx.recv().await
            .ok_or_else(|| anyhow::anyhow!("No response received"))?;
        
        // Check for error
        if let Some(error) = response.get("error") {
            return Err(anyhow::anyhow!("MCP error: {}", error));
        }
        
        Ok(response["result"].clone())
    }
    
    async fn send_notification(
        &self,
        method: &str,
        params: Value,
    ) -> Result<()> {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        
        let (response_tx, _) = mpsc::channel(1);
        let request = Request {
            id: None,
            message,
            response_tx,
        };
        
        self.request_tx.send(request).await
            .context("Failed to send notification")?;
        
        Ok(())
    }
    
    async fn close(&self) -> Result<()> {
        let mut process = self.process.lock().await;
        process.kill().await.context("Failed to kill process")?;
        Ok(())
    }
}

struct Request {
    id: Option<String>,
    message: Value,
    response_tx: mpsc::Sender<Value>,
}

/// HTTP/WebSocket transport for remote MCP servers
pub struct HttpTransport {
    base_url: String,
    client: reqwest::Client,
}

impl HttpTransport {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl McpTransport for HttpTransport {
    async fn send_request(
        &self,
        method: &str,
        params: Value,
    ) -> Result<Value> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": uuid::Uuid::new_v4().to_string(),
            "method": method,
            "params": params
        });
        
        let response = self.client
            .post(&self.base_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send HTTP request")?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "HTTP error: {}",
                response.status()
            ));
        }
        
        let json_response: Value = response.json().await
            .context("Failed to parse response")?;
        
        // Check for error
        if let Some(error) = json_response.get("error") {
            return Err(anyhow::anyhow!("MCP error: {}", error));
        }
        
        Ok(json_response["result"].clone())
    }
    
    async fn send_notification(
        &self,
        method: &str,
        params: Value,
    ) -> Result<()> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        
        self.client
            .post(&self.base_url)
            .json(&notification)
            .send()
            .await
            .context("Failed to send notification")?;
        
        Ok(())
    }
    
    async fn close(&self) -> Result<()> {
        // HTTP transport doesn't need cleanup
        Ok(())
    }
}

/// In-memory transport for testing
pub struct InMemoryTransport {
    handlers: Arc<Mutex<HashMap<String, Box<dyn Fn(Value) -> Value + Send + Sync>>>>,
}

impl InMemoryTransport {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Register a handler for a method
    pub async fn register_handler<F>(&self, method: &str, handler: F)
    where
        F: Fn(Value) -> Value + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.lock().await;
        handlers.insert(method.to_string(), Box::new(handler));
    }
}

#[async_trait]
impl McpTransport for InMemoryTransport {
    async fn send_request(
        &self,
        method: &str,
        params: Value,
    ) -> Result<Value> {
        let handlers = self.handlers.lock().await;
        
        if let Some(handler) = handlers.get(method) {
            Ok(handler(params))
        } else {
            Err(anyhow::anyhow!("No handler for method: {}", method))
        }
    }
    
    async fn send_notification(
        &self,
        _method: &str,
        _params: Value,
    ) -> Result<()> {
        // Notifications are ignored in testing
        Ok(())
    }
    
    async fn close(&self) -> Result<()> {
        Ok(())
    }
}