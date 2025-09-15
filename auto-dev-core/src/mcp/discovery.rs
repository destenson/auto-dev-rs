//! MCP server discovery from various configuration sources
//!
//! Automatically discovers MCP servers from Claude Desktop, VS Code, and other tools

use super::transport::StdioTransport;
use super::client::{McpClientManager, McpTransport};
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use tracing::{info, debug, warn};

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Claude Desktop configuration
#[derive(Debug, Deserialize)]
struct ClaudeDesktopConfig {
    #[serde(rename = "mcpServers")]
    mcp_servers: Option<HashMap<String, ClaudeServerConfig>>,
}

#[derive(Debug, Deserialize)]
struct ClaudeServerConfig {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
}

/// VS Code MCP extension configuration
#[derive(Debug, Deserialize)]
struct VsCodeConfig {
    #[serde(rename = "mcp.servers")]
    servers: Option<Vec<VsCodeServerConfig>>,
}

#[derive(Debug, Deserialize)]
struct VsCodeServerConfig {
    name: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
}

/// MCP server discovery
pub struct McpDiscovery;

impl McpDiscovery {
    /// Discover all available MCP servers from various sources
    pub async fn discover_all() -> Vec<McpServerConfig> {
        let mut servers = Vec::new();
        
        // Discover from Claude Desktop
        if let Ok(claude_servers) = Self::discover_claude_desktop().await {
            info!("Found {} servers from Claude Desktop", claude_servers.len());
            servers.extend(claude_servers);
        }
        
        // Discover from VS Code
        if let Ok(vscode_servers) = Self::discover_vscode().await {
            info!("Found {} servers from VS Code", vscode_servers.len());
            servers.extend(vscode_servers);
        }
        
        // Discover from user config
        if let Ok(user_servers) = Self::discover_user_config().await {
            info!("Found {} servers from user config", user_servers.len());
            servers.extend(user_servers);
        }
        
        // Discover from environment
        if let Ok(env_servers) = Self::discover_from_env().await {
            info!("Found {} servers from environment", env_servers.len());
            servers.extend(env_servers);
        }
        
        // Remove duplicates based on name
        let mut seen = std::collections::HashSet::new();
        servers.retain(|s| seen.insert(s.name.clone()));
        
        servers
    }
    
    /// Discover MCP servers from Claude Desktop configuration
    pub async fn discover_claude_desktop() -> Result<Vec<McpServerConfig>> {
        let config_paths = Self::get_claude_config_paths();
        let mut servers = Vec::new();
        
        for path in config_paths {
            if path.exists() {
                debug!("Checking Claude config at: {:?}", path);
                
                let content = tokio::fs::read_to_string(&path).await
                    .context("Failed to read Claude config")?;
                
                let config: ClaudeDesktopConfig = serde_json::from_str(&content)
                    .context("Failed to parse Claude config")?;
                
                if let Some(mcp_servers) = config.mcp_servers {
                    for (name, server) in mcp_servers {
                        servers.push(McpServerConfig {
                            name: name.clone(),
                            command: server.command,
                            args: server.args,
                            env: server.env,
                            description: Some(format!("Claude Desktop: {}", name)),
                            url: None,
                        });
                    }
                }
            }
        }
        
        Ok(servers)
    }
    
    /// Discover MCP servers from VS Code configuration
    pub async fn discover_vscode() -> Result<Vec<McpServerConfig>> {
        let config_paths = Self::get_vscode_config_paths();
        let mut servers = Vec::new();
        
        for path in config_paths {
            if path.exists() {
                debug!("Checking VS Code config at: {:?}", path);
                
                let content = tokio::fs::read_to_string(&path).await
                    .context("Failed to read VS Code config")?;
                
                // VS Code settings.json might have comments, try to clean them
                let cleaned = Self::remove_json_comments(&content);
                
                if let Ok(config) = serde_json::from_str::<serde_json::Value>(&cleaned) {
                    // Look for mcp.servers in the config
                    if let Some(mcp_servers) = config.get("mcp.servers") {
                        if let Ok(vscode_servers) = serde_json::from_value::<Vec<VsCodeServerConfig>>(mcp_servers.clone()) {
                            for server in vscode_servers {
                                servers.push(McpServerConfig {
                                    name: server.name.clone(),
                                    command: server.command,
                                    args: server.args,
                                    env: server.env,
                                    description: Some(format!("VS Code: {}", server.name)),
                                    url: None,
                                });
                            }
                        }
                    }
                }
            }
        }
        
        Ok(servers)
    }
    
    /// Discover from user's MCP configuration file
    pub async fn discover_user_config() -> Result<Vec<McpServerConfig>> {
        let config_paths = vec![
            // User home directory
            dirs::config_dir()
                .map(|d| d.join("mcp").join("servers.json")),
            dirs::home_dir()
                .map(|d| d.join(".mcp").join("servers.json")),
            // Current directory
            Some(PathBuf::from(".mcp.json")),
            Some(PathBuf::from("mcp.json")),
        ];
        
        let mut servers = Vec::new();
        
        for path_opt in config_paths {
            if let Some(path) = path_opt {
                if path.exists() {
                    debug!("Checking user config at: {:?}", path);
                    
                    let content = tokio::fs::read_to_string(&path).await
                        .context("Failed to read user config")?;
                    
                    if let Ok(user_servers) = serde_json::from_str::<Vec<McpServerConfig>>(&content) {
                        servers.extend(user_servers);
                    }
                }
            }
        }
        
        Ok(servers)
    }
    
    /// Discover from environment variables
    pub async fn discover_from_env() -> Result<Vec<McpServerConfig>> {
        let mut servers = Vec::new();
        
        // Check for MCP_SERVERS environment variable
        if let Ok(mcp_servers) = std::env::var("MCP_SERVERS") {
            if let Ok(env_servers) = serde_json::from_str::<Vec<McpServerConfig>>(&mcp_servers) {
                servers.extend(env_servers);
            }
        }
        
        // Check for individual MCP_SERVER_* variables
        for (key, value) in std::env::vars() {
            if key.starts_with("MCP_SERVER_") {
                let name = key.strip_prefix("MCP_SERVER_").unwrap().to_lowercase();
                
                // Value format: "command:arg1:arg2"
                let parts: Vec<String> = value.split(':').map(String::from).collect();
                if !parts.is_empty() {
                    servers.push(McpServerConfig {
                        name,
                        command: parts[0].clone(),
                        args: parts[1..].to_vec(),
                        env: HashMap::new(),
                        description: Some("From environment".to_string()),
                        url: None,
                    });
                }
            }
        }
        
        Ok(servers)
    }
    
    /// Get Claude Desktop configuration paths
    fn get_claude_config_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        
        #[cfg(target_os = "windows")]
        {
            if let Some(appdata) = dirs::data_dir() {
                paths.push(appdata.join("Claude").join("claude_desktop_config.json"));
            }
            if let Some(roaming) = std::env::var_os("APPDATA") {
                let roaming_path = PathBuf::from(roaming);
                paths.push(roaming_path.join("Claude").join("claude_desktop_config.json"));
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join("Library")
                    .join("Application Support")
                    .join("Claude")
                    .join("claude_desktop_config.json"));
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            if let Some(config) = dirs::config_dir() {
                paths.push(config.join("Claude").join("claude_desktop_config.json"));
            }
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join(".config")
                    .join("Claude")
                    .join("claude_desktop_config.json"));
            }
        }
        
        paths
    }
    
    /// Get VS Code configuration paths
    fn get_vscode_config_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        
        #[cfg(target_os = "windows")]
        {
            if let Some(appdata) = std::env::var_os("APPDATA") {
                let appdata_path = PathBuf::from(appdata);
                paths.push(appdata_path.join("Code").join("User").join("settings.json"));
                paths.push(appdata_path.join("Code - Insiders").join("User").join("settings.json"));
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join("Library")
                    .join("Application Support")
                    .join("Code")
                    .join("User")
                    .join("settings.json"));
                paths.push(home.join("Library")
                    .join("Application Support")
                    .join("Code - Insiders")
                    .join("User")
                    .join("settings.json"));
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            if let Some(config) = dirs::config_dir() {
                paths.push(config.join("Code").join("User").join("settings.json"));
                paths.push(config.join("Code - Insiders").join("User").join("settings.json"));
            }
        }
        
        // Also check workspace settings
        paths.push(PathBuf::from(".vscode").join("settings.json"));
        
        paths
    }
    
    /// Remove JSON comments (// and /* */) for parsing
    fn remove_json_comments(json: &str) -> String {
        let mut result = String::new();
        let mut chars = json.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '/' {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '/' {
                        // Single-line comment
                        chars.next(); // consume second /
                        while let Some(c) = chars.next() {
                            if c == '\n' {
                                result.push('\n');
                                break;
                            }
                        }
                    } else if next_ch == '*' {
                        // Multi-line comment
                        chars.next(); // consume *
                        let mut prev = ' ';
                        while let Some(c) = chars.next() {
                            if prev == '*' && c == '/' {
                                break;
                            }
                            prev = c;
                        }
                    } else {
                        result.push(ch);
                    }
                } else {
                    result.push(ch);
                }
            } else {
                result.push(ch);
            }
        }
        
        result
    }
    
    /// Auto-connect to discovered servers
    pub async fn auto_connect(manager: &mut McpClientManager) -> Result<()> {
        let servers = Self::discover_all().await;
        
        info!("Discovered {} MCP servers", servers.len());
        
        for server in servers {
            info!("Connecting to MCP server: {}", server.name);
            
            // Set environment variables
            for (key, value) in &server.env {
                std::env::set_var(key, value);
            }
            
            // Create transport based on configuration
            let transport: Box<dyn McpTransport> = if let Some(url) = server.url {
                // HTTP transport for remote servers
                Box::new(super::transport::HttpTransport::new(url))
            } else {
                // Stdio transport for local processes
                match StdioTransport::spawn(&server.command, server.args.clone()).await {
                    Ok(transport) => Box::new(transport),
                    Err(e) => {
                        warn!("Failed to spawn MCP server '{}': {}", server.name, e);
                        continue;
                    }
                }
            };
            
            if let Err(e) = manager.register_client(server.name.clone(), transport).await {
                warn!("Failed to register MCP client '{}': {}", server.name, e);
            } else {
                info!("Successfully connected to MCP server: {}", server.name);
            }
        }
        
        Ok(())
    }
}

/// Predefined MCP server configurations for common tools
pub struct PredefinedServers;

impl PredefinedServers {
    /// Get predefined server configurations
    pub fn get_all() -> Vec<McpServerConfig> {
        let mut servers = vec![
            // Web search server
            McpServerConfig {
                name: "web-search".to_string(),
                command: "npx".to_string(),
                args: vec!["-y".to_string(), "@modelcontextprotocol/server-websearch".to_string()],
                env: HashMap::new(),
                description: Some("Web search capabilities via Brave Search API".to_string()),
                url: None,
            },
            
            // Filesystem server
            McpServerConfig {
                name: "filesystem".to_string(),
                command: "npx".to_string(),
                args: vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()],
                env: HashMap::new(),
                description: Some("File system access".to_string()),
                url: None,
            },
            
            // Git server
            McpServerConfig {
                name: "git".to_string(),
                command: "npx".to_string(),
                args: vec!["-y".to_string(), "@modelcontextprotocol/server-git".to_string()],
                env: HashMap::new(),
                description: Some("Git repository operations".to_string()),
                url: None,
            },
            
            // SQLite server
            McpServerConfig {
                name: "sqlite".to_string(),
                command: "npx".to_string(),
                args: vec!["-y".to_string(), "@modelcontextprotocol/server-sqlite".to_string()],
                env: HashMap::new(),
                description: Some("SQLite database access".to_string()),
                url: None,
            },
            
            // Slack server
            McpServerConfig {
                name: "slack".to_string(),
                command: "npx".to_string(),
                args: vec!["-y".to_string(), "@modelcontextprotocol/server-slack".to_string()],
                env: HashMap::new(),
                description: Some("Slack integration".to_string()),
                url: None,
            },
            
            // GitHub server
            McpServerConfig {
                name: "github".to_string(),
                command: "npx".to_string(),
                args: vec!["-y".to_string(), "@modelcontextprotocol/server-github".to_string()],
                env: HashMap::new(),
                description: Some("GitHub API access".to_string()),
                url: None,
            },
        ];
        
        // Add cargo-mcp if available
        if let Some(cargo_mcp) = Self::get_cargo_mcp_config() {
            servers.push(cargo_mcp);
        }
        
        servers
    }
    
    /// Install a predefined server
    pub async fn install(name: &str) -> Result<()> {
        let servers = Self::get_all();
        
        let server = servers.iter()
            .find(|s| s.name == name)
            .ok_or_else(|| anyhow::anyhow!("Unknown server: {}", name))?;
        
        info!("Installing MCP server: {}", name);
        
        // For npm-based servers, we can pre-install them
        if server.command == "npx" {
            let output = tokio::process::Command::new("npm")
                .args(&["install", "-g", &server.args[1]])
                .output()
                .await?;
            
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("Failed to install {}: {}", name, stderr));
            }
        }
        
        info!("Successfully installed MCP server: {}", name);
        Ok(())
    }
    
    /// Get cargo-mcp configuration if available
    fn get_cargo_mcp_config() -> Option<McpServerConfig> {
        // Check for cargo-mcp at the specified location
        let cargo_mcp_path = PathBuf::from(r"C:\Users\deste\.cargo\bin\cargo-mcp.exe");
        
        if cargo_mcp_path.exists() {
            return Some(McpServerConfig {
                name: "cargo".to_string(),
                command: cargo_mcp_path.to_string_lossy().to_string(),
                args: vec![],
                env: HashMap::new(),
                description: Some("Cargo MCP server for Rust project management".to_string()),
                url: None,
            });
        }
        
        // Also check in PATH
        if let Ok(output) = std::process::Command::new("which")
            .arg("cargo-mcp")
            .output()
        {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(McpServerConfig {
                        name: "cargo".to_string(),
                        command: path,
                        args: vec![],
                        env: HashMap::new(),
                        description: Some("Cargo MCP server for Rust project management".to_string()),
                        url: None,
                    });
                }
            }
        }
        
        // Try Windows where command
        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = std::process::Command::new("where")
                .arg("cargo-mcp")
                .output()
            {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        return Some(McpServerConfig {
                            name: "cargo".to_string(),
                            command: path,
                            args: vec![],
                            env: HashMap::new(),
                            description: Some("Cargo MCP server for Rust project management".to_string()),
                            url: None,
                        });
                    }
                }
            }
        }
        
        None
    }
}