//! Built-in tool integrations for common MCP servers

use super::client::{McpClient, McpClientManager};
use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Web search tool wrapper
pub struct WebSearchTool {
    client: Arc<RwLock<McpClient>>,
}

impl WebSearchTool {
    pub fn new(client: Arc<RwLock<McpClient>>) -> Self {
        Self { client }
    }

    /// Search the web
    pub async fn search(
        &self,
        query: &str,
        max_results: Option<usize>,
    ) -> Result<Vec<SearchResult>> {
        let client = self.client.read().await;

        let arguments = json!({
            "query": query,
            "max_results": max_results.unwrap_or(10)
        });

        let response = client.call_tool("web_search", arguments).await?;

        // Parse search results
        let results = response["results"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid search response"))?;

        let mut search_results = Vec::new();
        for result in results {
            if let Ok(sr) = serde_json::from_value::<SearchResult>(result.clone()) {
                search_results.push(sr);
            }
        }

        Ok(search_results)
    }

    /// Get page content
    pub async fn fetch_page(&self, url: &str) -> Result<String> {
        let client = self.client.read().await;

        let arguments = json!({
            "url": url
        });

        let response = client.call_tool("fetch_page", arguments).await?;

        response["content"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("Invalid page content"))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub score: Option<f32>,
}

/// Database tool wrapper
pub struct DatabaseTool {
    client: Arc<RwLock<McpClient>>,
}

impl DatabaseTool {
    pub fn new(client: Arc<RwLock<McpClient>>) -> Self {
        Self { client }
    }

    /// Execute a SQL query
    pub async fn query(
        &self,
        sql: &str,
        params: Option<Vec<serde_json::Value>>,
    ) -> Result<QueryResult> {
        let client = self.client.read().await;

        let arguments = json!({
            "query": sql,
            "params": params.unwrap_or_default()
        });

        let response = client.call_tool("sql_query", arguments).await?;

        Ok(QueryResult {
            columns: response["columns"]
                .as_array()
                .and_then(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<_>>()
                        .into()
                })
                .unwrap_or_default(),
            rows: response["rows"].as_array().map(|arr| arr.to_vec()).unwrap_or_default(),
            affected_rows: response["affected_rows"].as_u64(),
        })
    }

    /// Get database schema
    pub async fn get_schema(&self, table: Option<&str>) -> Result<serde_json::Value> {
        let client = self.client.read().await;

        let arguments = if let Some(table) = table { json!({ "table": table }) } else { json!({}) };

        client.call_tool("get_schema", arguments).await
    }
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<serde_json::Value>,
    pub affected_rows: Option<u64>,
}

/// File system tool wrapper
pub struct FileSystemTool {
    client: Arc<RwLock<McpClient>>,
}

impl FileSystemTool {
    pub fn new(client: Arc<RwLock<McpClient>>) -> Self {
        Self { client }
    }

    /// Read a file
    pub async fn read_file(&self, path: &str) -> Result<String> {
        let client = self.client.read().await;

        let arguments = json!({
            "path": path
        });

        let response = client.call_tool("read_file", arguments).await?;

        response["content"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("Invalid file content"))
    }

    /// Write a file
    pub async fn write_file(&self, path: &str, content: &str) -> Result<()> {
        let client = self.client.read().await;

        let arguments = json!({
            "path": path,
            "content": content
        });

        client.call_tool("write_file", arguments).await?;
        Ok(())
    }

    /// List directory contents
    pub async fn list_directory(&self, path: &str) -> Result<Vec<FileInfo>> {
        let client = self.client.read().await;

        let arguments = json!({
            "path": path
        });

        let response = client.call_tool("list_directory", arguments).await?;

        let entries = response["entries"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid directory listing"))?;

        let mut files = Vec::new();
        for entry in entries {
            if let Ok(fi) = serde_json::from_value::<FileInfo>(entry.clone()) {
                files.push(fi);
            }
        }

        Ok(files)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: Option<u64>,
    pub modified: Option<String>,
}

/// Git tool wrapper
pub struct GitTool {
    client: Arc<RwLock<McpClient>>,
}

impl GitTool {
    pub fn new(client: Arc<RwLock<McpClient>>) -> Self {
        Self { client }
    }

    /// Get repository status
    pub async fn status(&self, repo_path: &str) -> Result<serde_json::Value> {
        let client = self.client.read().await;

        let arguments = json!({
            "repo_path": repo_path
        });

        client.call_tool("git_status", arguments).await
    }

    /// Get commit history
    pub async fn log(
        &self,
        repo_path: &str,
        limit: Option<usize>,
        branch: Option<&str>,
    ) -> Result<Vec<Commit>> {
        let client = self.client.read().await;

        let arguments = json!({
            "repo_path": repo_path,
            "limit": limit.unwrap_or(50),
            "branch": branch
        });

        let response = client.call_tool("git_log", arguments).await?;

        let commits = response["commits"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid git log response"))?;

        let mut commit_list = Vec::new();
        for commit in commits {
            if let Ok(c) = serde_json::from_value::<Commit>(commit.clone()) {
                commit_list.push(c);
            }
        }

        Ok(commit_list)
    }

    /// Get diff
    pub async fn diff(
        &self,
        repo_path: &str,
        commit1: Option<&str>,
        commit2: Option<&str>,
    ) -> Result<String> {
        let client = self.client.read().await;

        let arguments = json!({
            "repo_path": repo_path,
            "commit1": commit1,
            "commit2": commit2
        });

        let response = client.call_tool("git_diff", arguments).await?;

        response["diff"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("Invalid diff response"))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Commit {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
    pub files_changed: Option<Vec<String>>,
}

/// Slack tool wrapper
pub struct SlackTool {
    client: Arc<RwLock<McpClient>>,
}

impl SlackTool {
    pub fn new(client: Arc<RwLock<McpClient>>) -> Self {
        Self { client }
    }

    /// Send a message
    pub async fn send_message(
        &self,
        channel: &str,
        text: &str,
        thread_ts: Option<&str>,
    ) -> Result<String> {
        let client = self.client.read().await;

        let arguments = json!({
            "channel": channel,
            "text": text,
            "thread_ts": thread_ts
        });

        let response = client.call_tool("slack_send_message", arguments).await?;

        response["ts"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("Invalid message timestamp"))
    }

    /// Get channel history
    pub async fn get_history(&self, channel: &str, limit: Option<usize>) -> Result<Vec<Message>> {
        let client = self.client.read().await;

        let arguments = json!({
            "channel": channel,
            "limit": limit.unwrap_or(100)
        });

        let response = client.call_tool("slack_get_history", arguments).await?;

        let messages = response["messages"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid history response"))?;

        let mut message_list = Vec::new();
        for msg in messages {
            if let Ok(m) = serde_json::from_value::<Message>(msg.clone()) {
                message_list.push(m);
            }
        }

        Ok(message_list)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub ts: String,
    pub user: String,
    pub text: String,
    pub thread_ts: Option<String>,
    pub reactions: Option<Vec<String>>,
}

/// Tool registry for easy access
pub struct ToolRegistry {
    manager: Arc<McpClientManager>,
}

impl ToolRegistry {
    pub fn new(manager: Arc<McpClientManager>) -> Self {
        Self { manager }
    }

    /// Create a web search tool if available
    pub async fn web_search(&self) -> Option<WebSearchTool> {
        // Try to find a client with web search capabilities
        for client_name in self.manager.list_clients() {
            if let Some(client) = self.manager.get_client(&client_name) {
                let c = client.read().await;
                if c.get_tool("web_search").await.is_some() {
                    return Some(WebSearchTool::new(client.clone()));
                }
            }
        }
        None
    }

    /// Create a database tool if available
    pub async fn database(&self) -> Option<DatabaseTool> {
        for client_name in self.manager.list_clients() {
            if let Some(client) = self.manager.get_client(&client_name) {
                let c = client.read().await;
                if c.get_tool("sql_query").await.is_some() {
                    return Some(DatabaseTool::new(client.clone()));
                }
            }
        }
        None
    }

    /// Create a file system tool if available
    pub async fn filesystem(&self) -> Option<FileSystemTool> {
        for client_name in self.manager.list_clients() {
            if let Some(client) = self.manager.get_client(&client_name) {
                let c = client.read().await;
                if c.get_tool("read_file").await.is_some() {
                    return Some(FileSystemTool::new(client.clone()));
                }
            }
        }
        None
    }

    /// Create a git tool if available
    pub async fn git(&self) -> Option<GitTool> {
        for client_name in self.manager.list_clients() {
            if let Some(client) = self.manager.get_client(&client_name) {
                let c = client.read().await;
                if c.get_tool("git_status").await.is_some() {
                    return Some(GitTool::new(client.clone()));
                }
            }
        }
        None
    }

    /// Create a Slack tool if available
    pub async fn slack(&self) -> Option<SlackTool> {
        for client_name in self.manager.list_clients() {
            if let Some(client) = self.manager.get_client(&client_name) {
                let c = client.read().await;
                if c.get_tool("slack_send_message").await.is_some() {
                    return Some(SlackTool::new(client.clone()));
                }
            }
        }
        None
    }
}
