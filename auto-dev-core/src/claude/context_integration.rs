//! Integration of Claude configuration into context management
//!
//! This module provides integration between CLAUDE.md configuration
//! and the LLM context management system.

use crate::claude::{
    ClaudeConfigDiscovery, ClaudeMdContent, ClaudeMdLoader, CommandParser, CommandRegistry,
};
use crate::llm::context_manager::ContextManager;
use anyhow::{Context, Result};
use std::path::Path;
use std::sync::{Arc, RwLock};

/// Provides Claude configuration context for LLM interactions
pub struct ClaudeContextProvider {
    /// Loaded CLAUDE.md content
    claude_md: Arc<RwLock<Option<ClaudeMdContent>>>,
    /// Parsed command registry
    commands: Arc<RwLock<CommandRegistry>>,
    /// Configuration discovery
    discovery: ClaudeConfigDiscovery,
    /// Loader for CLAUDE.md files
    loader: ClaudeMdLoader,
}

impl ClaudeContextProvider {
    /// Create a new Claude context provider
    pub fn new() -> Result<Self> {
        let discovery = ClaudeConfigDiscovery::new();
        let loader = ClaudeMdLoader::new();

        Ok(Self {
            claude_md: Arc::new(RwLock::new(None)),
            commands: Arc::new(RwLock::new(CommandRegistry::new())),
            discovery,
            loader,
        })
    }

    /// Initialize by discovering and loading configuration
    pub async fn initialize(&self) -> Result<()> {
        // Discover configuration paths
        let paths = self.discovery.discover().await?;

        // Load CLAUDE.md content
        let claude_md_paths = paths.claude_md_paths();
        if !claude_md_paths.is_empty() {
            if let Some(content) = self.loader.load_and_merge(&claude_md_paths).await? {
                let mut claude_md = self.claude_md.write().unwrap();
                *claude_md = Some(content);
            }
        }

        // Load commands if directory exists
        let commands_dirs = paths.commands_dirs();
        for commands_dir in commands_dirs {
            self.load_commands(&commands_dir)?;
        }

        Ok(())
    }

    /// Load commands from a directory
    fn load_commands(&self, commands_dir: &Path) -> Result<()> {
        let mut parser = CommandParser::new();
        parser
            .parse_directory(commands_dir)
            .with_context(|| format!("Failed to parse commands from {}", commands_dir.display()))?;

        let mut commands = self.commands.write().unwrap();
        *commands = parser.into_registry();

        Ok(())
    }

    /// Reload configuration from disk
    pub async fn reload(&self) -> Result<()> {
        self.initialize().await
    }

    /// Get formatted context for inclusion in LLM prompts
    pub fn get_formatted_context(&self) -> Option<String> {
        let claude_md = self.claude_md.read().unwrap();

        if let Some(content) = claude_md.as_ref() {
            Some(format_claude_context(content))
        } else {
            None
        }
    }

    /// Get command context if any commands are loaded
    pub fn get_command_context(&self) -> Option<String> {
        let commands = self.commands.read().unwrap();

        if commands.metadata.command_count > 0 {
            Some(format_command_context(&*commands))
        } else {
            None
        }
    }

    /// Get complete Claude context
    pub fn get_complete_context(&self) -> Option<String> {
        let mut parts = Vec::new();

        if let Some(claude_context) = self.get_formatted_context() {
            parts.push(claude_context);
        }

        if let Some(command_context) = self.get_command_context() {
            parts.push(command_context);
        }

        if parts.is_empty() { None } else { Some(parts.join("\n\n---\n\n")) }
    }

    /// Check if any configuration is loaded
    pub fn has_configuration(&self) -> bool {
        let claude_md = self.claude_md.read().unwrap();
        let commands = self.commands.read().unwrap();

        claude_md.is_some() || commands.metadata.command_count > 0
    }
}

/// Format CLAUDE.md content for inclusion in prompts
fn format_claude_context(content: &ClaudeMdContent) -> String {
    let mut formatted = String::new();

    formatted.push_str("=== User Configuration (CLAUDE.md) ===\n\n");

    formatted.push_str(&content.content);
    formatted.push_str("\n\n");

    if !content.sources.is_empty() {
        formatted.push_str("Sources: ");
        for (i, source) in content.sources.iter().enumerate() {
            if i > 0 {
                formatted.push_str(", ");
            }
            formatted.push_str(&source.display().to_string());
        }
        formatted.push_str("\n\n");
    }

    formatted.push_str("=== End User Configuration ===\n");

    formatted
}

/// Format command registry for inclusion in prompts
fn format_command_context(registry: &CommandRegistry) -> String {
    let mut formatted = String::new();

    formatted.push_str("=== Available Claude Commands ===\n\n");

    for command in registry.all_commands() {
        formatted.push_str(&format!("/{}: {}\n", command.name, command.description));
    }

    formatted.push_str("\n=== End Claude Commands ===\n");

    formatted
}

/// Extension trait for ContextManager to support Claude configuration
pub trait ClaudeContextExtension {
    /// Add Claude configuration as priority context
    fn with_claude_context(&mut self, provider: &ClaudeContextProvider);

    /// Format context with Claude configuration included
    fn build_context_with_claude(
        &self,
        primary_content: &str,
        claude_provider: Option<&ClaudeContextProvider>,
    ) -> String;
}

impl ClaudeContextExtension for ContextManager {
    fn with_claude_context(&mut self, provider: &ClaudeContextProvider) {
        if let Some(context) = provider.get_complete_context() {
            self.add_priority_context(context);
        }
    }

    fn build_context_with_claude(
        &self,
        primary_content: &str,
        claude_provider: Option<&ClaudeContextProvider>,
    ) -> String {
        let mut context_parts = Vec::new();

        // Add Claude context first if available
        if let Some(provider) = claude_provider {
            if let Some(claude_context) = provider.get_complete_context() {
                context_parts.push(claude_context);
            }
        }

        // Then add the regular context
        context_parts.push(self.build_context(primary_content, None));

        context_parts.join("\n\n---\n\n")
    }
}

impl Default for ClaudeContextProvider {
    fn default() -> Self {
        Self::new().expect("Failed to create default ClaudeContextProvider")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_context_provider_creation() {
        let provider = ClaudeContextProvider::new().unwrap();
        assert!(!provider.has_configuration());
    }

    #[test]
    fn test_format_claude_context() {
        let content = ClaudeMdContent {
            content: "Test instructions\n\nFollow these rules.".to_string(),
            sources: vec![PathBuf::from("/test/CLAUDE.md")],
            total_size: 100,
        };

        let formatted = format_claude_context(&content);
        assert!(formatted.contains("User Configuration"));
        assert!(formatted.contains("Test instructions"));
        assert!(formatted.contains("Sources:"));
    }

    #[test]
    fn test_format_command_context() {
        use crate::claude::command_types::ClaudeCommand;

        let mut registry = CommandRegistry::new();
        registry
            .add_command(ClaudeCommand::new("test-cmd".to_string(), "Test command".to_string()));

        let formatted = format_command_context(&registry);
        assert!(formatted.contains("/test-cmd: Test command"));
    }

    #[tokio::test]
    async fn test_initialize_with_files() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir(&claude_dir)?;

        // Create CLAUDE.md
        let claude_md = claude_dir.join("CLAUDE.md");
        fs::write(&claude_md, "# Test Instructions\n\nFollow these rules.")?;

        // Create commands directory
        let commands_dir = claude_dir.join("commands");
        fs::create_dir(&commands_dir)?;

        // Create a test command
        let cmd_file = commands_dir.join("test.md");
        fs::write(&cmd_file, "# Test\n\nA test command.")?;

        // Initialize provider with test directory as working dir
        std::env::set_current_dir(temp_dir.path())?;

        let provider = ClaudeContextProvider::new()?;
        provider.initialize().await?;

        assert!(provider.has_configuration());
        assert!(provider.get_formatted_context().is_some());
        assert!(provider.get_command_context().is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_reload_functionality() -> Result<()> {
        let provider = ClaudeContextProvider::new()?;

        // Initial state
        assert!(!provider.has_configuration());

        // Reload should work without error even with no config
        provider.reload().await?;

        Ok(())
    }

    #[test]
    fn test_complete_context() {
        let provider = ClaudeContextProvider::new().unwrap();

        // Without any configuration
        assert!(provider.get_complete_context().is_none());

        // With CLAUDE.md loaded
        let content = ClaudeMdContent {
            content: "Test instructions".to_string(),
            sources: vec![],
            total_size: 16,
        };

        let mut claude_md = provider.claude_md.write().unwrap();
        *claude_md = Some(content);
        drop(claude_md);

        let complete = provider.get_complete_context();
        assert!(complete.is_some());
        assert!(complete.unwrap().contains("User Configuration"));
    }
}
