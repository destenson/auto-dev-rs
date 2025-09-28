//! Command data structures for Claude command files
//!
//! This module defines the types used to represent parsed Claude commands.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a parsed Claude command from a markdown file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeCommand {
    /// Command name (derived from filename)
    pub name: String,
    /// Brief description from first paragraph
    pub description: String,
    /// Full usage instructions
    pub usage: String,
    /// Command arguments
    pub arguments: Vec<CommandArgument>,
    /// Full command instructions/body
    pub instructions: String,
    /// Raw markdown content
    pub raw_content: String,
    /// Optional examples
    pub examples: Vec<String>,
}

/// Represents a command argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandArgument {
    /// Argument name
    pub name: String,
    /// Argument type (if specified)
    pub arg_type: Option<String>,
    /// Whether the argument is required
    pub required: bool,
    /// Argument description
    pub description: String,
    /// Default value if optional
    pub default: Option<String>,
}

/// Registry of all parsed commands
#[derive(Debug, Clone, Default)]
pub struct CommandRegistry {
    /// Commands indexed by name
    commands: HashMap<String, ClaudeCommand>,
    /// Metadata about the registry
    pub metadata: RegistryMetadata,
}

/// Metadata about the command registry
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RegistryMetadata {
    /// Number of commands loaded
    pub command_count: usize,
    /// List of command sources
    pub sources: Vec<String>,
    /// Timestamp of last load
    pub last_loaded: Option<std::time::SystemTime>,
}

impl CommandRegistry {
    /// Create a new command registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a command to the registry
    pub fn add_command(&mut self, command: ClaudeCommand) {
        self.commands.insert(command.name.clone(), command);
        self.metadata.command_count = self.commands.len();
    }

    /// Get a command by name
    pub fn get(&self, name: &str) -> Option<&ClaudeCommand> {
        self.commands.get(name)
    }

    /// Get all command names
    pub fn command_names(&self) -> Vec<String> {
        self.commands.keys().cloned().collect()
    }

    /// Get all commands
    pub fn all_commands(&self) -> Vec<&ClaudeCommand> {
        self.commands.values().collect()
    }

    /// Check if a command exists
    pub fn contains(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }
}

impl ClaudeCommand {
    /// Create a new command with basic information
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            usage: String::new(),
            arguments: Vec::new(),
            instructions: String::new(),
            raw_content: String::new(),
            examples: Vec::new(),
        }
    }

    /// Check if command has required arguments
    pub fn has_required_arguments(&self) -> bool {
        self.arguments.iter().any(|arg| arg.required)
    }

    /// Get required arguments
    pub fn required_arguments(&self) -> Vec<&CommandArgument> {
        self.arguments.iter().filter(|arg| arg.required).collect()
    }

    /// Get optional arguments
    pub fn optional_arguments(&self) -> Vec<&CommandArgument> {
        self.arguments.iter().filter(|arg| !arg.required).collect()
    }
}

impl CommandArgument {
    /// Create a new required argument
    pub fn required(name: String, description: String) -> Self {
        Self {
            name,
            arg_type: None,
            required: true,
            description,
            default: None,
        }
    }

    /// Create a new optional argument
    pub fn optional(name: String, description: String, default: Option<String>) -> Self {
        Self {
            name,
            arg_type: None,
            required: false,
            description,
            default,
        }
    }

    /// Set the argument type
    pub fn with_type(mut self, arg_type: String) -> Self {
        self.arg_type = Some(arg_type);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_creation() {
        let cmd = ClaudeCommand::new("test".to_string(), "Test command".to_string());
        assert_eq!(cmd.name, "test");
        assert_eq!(cmd.description, "Test command");
        assert!(cmd.arguments.is_empty());
    }

    #[test]
    fn test_registry_operations() {
        let mut registry = CommandRegistry::new();
        
        let cmd1 = ClaudeCommand::new("cmd1".to_string(), "Command 1".to_string());
        let cmd2 = ClaudeCommand::new("cmd2".to_string(), "Command 2".to_string());
        
        registry.add_command(cmd1);
        registry.add_command(cmd2);
        
        assert_eq!(registry.metadata.command_count, 2);
        assert!(registry.contains("cmd1"));
        assert!(registry.contains("cmd2"));
        assert!(!registry.contains("cmd3"));
        
        let names = registry.command_names();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_argument_types() {
        let required = CommandArgument::required(
            "input".to_string(),
            "Input file".to_string()
        );
        assert!(required.required);
        assert!(required.default.is_none());
        
        let optional = CommandArgument::optional(
            "output".to_string(),
            "Output file".to_string(),
            Some("output.txt".to_string())
        );
        assert!(!optional.required);
        assert_eq!(optional.default, Some("output.txt".to_string()));
    }
}