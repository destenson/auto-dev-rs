//! MCP client implementation for connecting to MCP servers

use super::*;
use anyhow::{Result, Context};
use serde_json::json;
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;
use tracing::{info, debug, warn};

/// MCP client for interacting with MCP servers
pub struct McpClient {
    transport: Box<dyn McpTransport>,
    tools: Arc<RwLock<HashMap<String, Tool>>>,
    resources: Arc<RwLock<HashMap<String, Resource>>>,
    prompts: Arc<RwLock<HashMap<String, Prompt>>>,
    initialized: bool,
}

impl McpClient {
    /// Create a new MCP client
    pub async fn new(transport: Box<dyn McpTransport>) -> Result<Self> {
        Ok(Self {
            transport,
            tools: Arc::new(RwLock::new(HashMap::new())),
            resources: Arc::new(RwLock::new(HashMap::new())),
            prompts: Arc::new(RwLock::new(HashMap::new())),
            initialized: false,
        })
    }
    
    /// Initialize the MCP connection
    pub async fn initialize(&mut self) -> Result<()> {
        let request = InitializeRequest {
            protocol_version: MCP_VERSION.to_string(),
            capabilities: ClientCapabilities {
                tools: Some(ToolCapabilities { call: true }),
                resources: Some(ResourceCapabilities { 
                    read: true, 
                    write: true 
                }),
                prompts: Some(PromptCapabilities { get: true }),
            },
            client_info: ClientInfo {
                name: "auto-dev-rs".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };
        
        let response = self.transport
            .send_request("initialize", serde_json::to_value(request)?)
            .await?;
        
        info!("MCP client initialized with server");
        
        // Send initialized notification
        self.transport
            .send_notification("initialized", json!({}))
            .await?;
        
        // Discover available tools, resources, and prompts
        self.discover_capabilities().await?;
        
        self.initialized = true;
        Ok(())
    }
    
    /// Discover available capabilities from the server
    async fn discover_capabilities(&mut self) -> Result<()> {
        // List tools
        let tools_response = self.transport
            .send_request("tools/list", json!({}))
            .await?;
        
        if let Some(tools) = tools_response["tools"].as_array() {
            let mut tools_map = self.tools.write().await;
            for tool in tools {
                if let Ok(t) = serde_json::from_value::<Tool>(tool.clone()) {
                    debug!("Discovered tool: {}", t.name);
                    tools_map.insert(t.name.clone(), t);
                }
            }
        }
        
        // List resources
        let resources_response = self.transport
            .send_request("resources/list", json!({}))
            .await?;
        
        if let Some(resources) = resources_response["resources"].as_array() {
            let mut resources_map = self.resources.write().await;
            for resource in resources {
                if let Ok(r) = serde_json::from_value::<Resource>(resource.clone()) {
                    debug!("Discovered resource: {}", r.uri);
                    resources_map.insert(r.uri.clone(), r);
                }
            }
        }
        
        // List prompts
        let prompts_response = self.transport
            .send_request("prompts/list", json!({}))
            .await?;
        
        if let Some(prompts) = prompts_response["prompts"].as_array() {
            let mut prompts_map = self.prompts.write().await;
            for prompt in prompts {
                if let Ok(p) = serde_json::from_value::<Prompt>(prompt.clone()) {
                    debug!("Discovered prompt: {}", p.name);
                    prompts_map.insert(p.name.clone(), p);
                }
            }
        }
        
        Ok(())
    }
    
    /// Call a tool
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Client not initialized"));
        }
        
        // Check if tool exists
        let tools = self.tools.read().await;
        if !tools.contains_key(name) {
            return Err(anyhow::anyhow!("Tool '{}' not found", name));
        }
        
        let request = ToolCallRequest {
            name: name.to_string(),
            arguments,
        };
        
        let response = self.transport
            .send_request("tools/call", serde_json::to_value(request)?)
            .await?;
        
        Ok(response)
    }
    
    /// Read a resource
    pub async fn read_resource(&self, uri: &str) -> Result<serde_json::Value> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Client not initialized"));
        }
        
        let request = ResourceReadRequest {
            uri: uri.to_string(),
        };
        
        let response = self.transport
            .send_request("resources/read", serde_json::to_value(request)?)
            .await?;
        
        Ok(response)
    }
    
    /// Get a prompt
    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<HashMap<String, String>>,
    ) -> Result<String> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Client not initialized"));
        }
        
        let request = PromptGetRequest {
            name: name.to_string(),
            arguments,
        };
        
        let response = self.transport
            .send_request("prompts/get", serde_json::to_value(request)?)
            .await?;
        
        response["prompt"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("Invalid prompt response"))
    }
    
    /// List available tools
    pub async fn list_tools(&self) -> Vec<Tool> {
        self.tools.read().await.values().cloned().collect()
    }
    
    /// List available resources
    pub async fn list_resources(&self) -> Vec<Resource> {
        self.resources.read().await.values().cloned().collect()
    }
    
    /// List available prompts
    pub async fn list_prompts(&self) -> Vec<Prompt> {
        self.prompts.read().await.values().cloned().collect()
    }
    
    /// Get tool by name
    pub async fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tools.read().await.get(name).cloned()
    }
}

/// Transport trait for MCP communication
#[async_trait::async_trait]
pub trait McpTransport: Send + Sync {
    /// Send a request and wait for response
    async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value>;
    
    /// Send a notification (no response expected)
    async fn send_notification(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<()>;
    
    /// Close the transport
    async fn close(&self) -> Result<()>;
}

/// Manager for multiple MCP clients
pub struct McpClientManager {
    clients: HashMap<String, Arc<RwLock<McpClient>>>,
}

impl McpClientManager {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }
    
    /// Register a new MCP client
    pub async fn register_client(
        &mut self,
        name: String,
        transport: Box<dyn McpTransport>,
    ) -> Result<()> {
        let mut client = McpClient::new(transport).await?;
        client.initialize().await?;
        
        self.clients.insert(name.clone(), Arc::new(RwLock::new(client)));
        info!("Registered MCP client: {}", name);
        
        Ok(())
    }
    
    /// Get a client by name
    pub fn get_client(&self, name: &str) -> Option<Arc<RwLock<McpClient>>> {
        self.clients.get(name).cloned()
    }
    
    /// List all registered clients
    pub fn list_clients(&self) -> Vec<String> {
        self.clients.keys().cloned().collect()
    }
    
    /// Call a tool on any available client
    pub async fn call_tool_any(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value> {
        // Try to find a client that has this tool
        for (client_name, client) in &self.clients {
            let client = client.read().await;
            if let Some(_tool) = client.get_tool(tool_name).await {
                debug!("Calling tool '{}' on client '{}'", tool_name, client_name);
                return client.call_tool(tool_name, arguments).await;
            }
        }
        
        Err(anyhow::anyhow!("Tool '{}' not found in any client", tool_name))
    }
}