//! Enhanced command registry with source tracking and thread safety
//!
//! This module provides a thread-safe registry for Claude commands
//! with support for multiple sources and conflict resolution.

use crate::claude::command_types::ClaudeCommand;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Source of a command
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandSource {
    /// Command from global .claude directory
    Global,
    /// Command from project .claude directory
    Project,
}

/// A registered command with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredCommand {
    /// The command itself
    pub command: ClaudeCommand,
    /// Source of the command
    pub source: CommandSource,
    /// When the command was registered
    pub registered_at: DateTime<Utc>,
    /// Number of times the command has been accessed
    pub usage_count: usize,
    /// Last time the command was accessed
    pub last_accessed: Option<DateTime<Utc>>,
}

/// Usage statistics for commands
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommandStats {
    /// Total number of registered commands
    pub total_commands: usize,
    /// Number of global commands
    pub global_commands: usize,
    /// Number of project commands
    pub project_commands: usize,
    /// Number of overridden global commands
    pub overridden_commands: usize,
    /// Most used commands
    pub top_commands: Vec<(String, usize)>,
}

/// Thread-safe command registry
#[derive(Clone)]
pub struct CommandRegistrySystem {
    /// Commands indexed by name
    commands: Arc<RwLock<HashMap<String, RegisteredCommand>>>,
    /// Overridden global commands (kept for reference)
    overridden: Arc<RwLock<HashMap<String, RegisteredCommand>>>,
}

impl CommandRegistrySystem {
    /// Create a new command registry
    pub fn new() -> Self {
        Self {
            commands: Arc::new(RwLock::new(HashMap::new())),
            overridden: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a command
    pub fn register_command(&self, command: ClaudeCommand, source: CommandSource) -> Result<()> {
        let registered = RegisteredCommand {
            command: command.clone(),
            source,
            registered_at: Utc::now(),
            usage_count: 0,
            last_accessed: None,
        };

        let mut commands = self.commands.write().unwrap();
        let mut overridden = self.overridden.write().unwrap();

        // Check for existing command
        if let Some(existing) = commands.get(&command.name) {
            if existing.source == CommandSource::Global && source == CommandSource::Project {
                // Project overrides global
                tracing::debug!(
                    "Command '{}' from project overrides global version",
                    command.name
                );
                overridden.insert(command.name.clone(), existing.clone());
            } else if existing.source == CommandSource::Project && source == CommandSource::Global {
                // Global doesn't override project
                tracing::debug!(
                    "Skipping global command '{}' as project version exists",
                    command.name
                );
                return Ok(());
            }
        }

        commands.insert(command.name.clone(), registered);
        Ok(())
    }

    /// Register multiple commands at once
    pub fn register_commands(&self, commands: Vec<ClaudeCommand>, source: CommandSource) -> Result<()> {
        for command in commands {
            self.register_command(command, source)?;
        }
        Ok(())
    }

    /// Get a command by exact name
    pub fn get_command(&self, name: &str) -> Option<ClaudeCommand> {
        let mut commands = self.commands.write().unwrap();
        
        if let Some(registered) = commands.get_mut(name) {
            // Update usage statistics
            registered.usage_count += 1;
            registered.last_accessed = Some(Utc::now());
            Some(registered.command.clone())
        } else {
            None
        }
    }

    /// Get a command without updating usage stats
    pub fn peek_command(&self, name: &str) -> Option<ClaudeCommand> {
        let commands = self.commands.read().unwrap();
        commands.get(name).map(|r| r.command.clone())
    }

    /// List all commands
    pub fn list_commands(&self) -> Vec<String> {
        let commands = self.commands.read().unwrap();
        commands.keys().cloned().collect()
    }

    /// List commands by source
    pub fn list_commands_by_source(&self, source: CommandSource) -> Vec<String> {
        let commands = self.commands.read().unwrap();
        commands
            .iter()
            .filter(|(_, r)| r.source == source)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Search commands by partial name
    pub fn search_commands(&self, query: &str) -> Vec<String> {
        let commands = self.commands.read().unwrap();
        let query_lower = query.to_lowercase();
        
        commands
            .keys()
            .filter(|name| name.to_lowercase().contains(&query_lower))
            .cloned()
            .collect()
    }

    /// Search commands with fuzzy matching
    pub fn fuzzy_search(&self, query: &str) -> Vec<(String, f32)> {
        let commands = self.commands.read().unwrap();
        let mut results = Vec::new();
        
        for name in commands.keys() {
            let score = calculate_similarity(query, name);
            if score > 0.3 {  // Minimum similarity threshold
                results.push((name.clone(), score));
            }
        }
        
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results
    }

    /// Get command with metadata
    pub fn get_registered_command(&self, name: &str) -> Option<RegisteredCommand> {
        let commands = self.commands.read().unwrap();
        commands.get(name).cloned()
    }

    /// Get statistics about registered commands
    pub fn get_stats(&self) -> CommandStats {
        let commands = self.commands.read().unwrap();
        let overridden = self.overridden.read().unwrap();
        
        let mut stats = CommandStats {
            total_commands: commands.len(),
            global_commands: 0,
            project_commands: 0,
            overridden_commands: overridden.len(),
            top_commands: Vec::new(),
        };
        
        let mut usage_map = Vec::new();
        
        for (name, registered) in commands.iter() {
            match registered.source {
                CommandSource::Global => stats.global_commands += 1,
                CommandSource::Project => stats.project_commands += 1,
            }
            
            if registered.usage_count > 0 {
                usage_map.push((name.clone(), registered.usage_count));
            }
        }
        
        // Get top 5 most used commands
        usage_map.sort_by(|a, b| b.1.cmp(&a.1));
        stats.top_commands = usage_map.into_iter().take(5).collect();
        
        stats
    }

    /// Clear all commands
    pub fn clear(&self) {
        let mut commands = self.commands.write().unwrap();
        let mut overridden = self.overridden.write().unwrap();
        commands.clear();
        overridden.clear();
    }

    /// Get overridden commands
    pub fn get_overridden_commands(&self) -> Vec<String> {
        let overridden = self.overridden.read().unwrap();
        overridden.keys().cloned().collect()
    }

    /// Check if a command exists
    pub fn contains(&self, name: &str) -> bool {
        let commands = self.commands.read().unwrap();
        commands.contains_key(name)
    }

    /// Get all commands with their metadata
    pub fn get_all_registered(&self) -> Vec<RegisteredCommand> {
        let commands = self.commands.read().unwrap();
        commands.values().cloned().collect()
    }
}

/// Calculate similarity between two strings (simple algorithm)
fn calculate_similarity(s1: &str, s2: &str) -> f32 {
    let s1_lower = s1.to_lowercase();
    let s2_lower = s2.to_lowercase();
    
    // Exact match
    if s1_lower == s2_lower {
        return 1.0;
    }
    
    // Prefix match
    if s2_lower.starts_with(&s1_lower) {
        return 0.8;
    }
    
    // Contains match
    if s2_lower.contains(&s1_lower) {
        return 0.6;
    }
    
    // Character-based similarity
    let common = s1_lower.chars()
        .filter(|c| s2_lower.contains(*c))
        .count() as f32;
    
    let max_len = s1_lower.len().max(s2_lower.len()) as f32;
    common / max_len
}

impl Default for CommandRegistrySystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::command_types::ClaudeCommand;

    fn create_test_command(name: &str) -> ClaudeCommand {
        ClaudeCommand::new(name.to_string(), format!("Description for {}", name))
    }

    #[test]
    fn test_register_and_get() {
        let registry = CommandRegistrySystem::new();
        let cmd = create_test_command("test");
        
        registry.register_command(cmd.clone(), CommandSource::Global).unwrap();
        
        let retrieved = registry.get_command("test");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test");
    }

    #[test]
    fn test_project_overrides_global() {
        let registry = CommandRegistrySystem::new();
        
        let global_cmd = create_test_command("cmd");
        let project_cmd = ClaudeCommand::new("cmd".to_string(), "Project version".to_string());
        
        registry.register_command(global_cmd, CommandSource::Global).unwrap();
        registry.register_command(project_cmd.clone(), CommandSource::Project).unwrap();
        
        let retrieved = registry.get_command("cmd");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().description, "Project version");
        
        // Check overridden list
        let overridden = registry.get_overridden_commands();
        assert!(overridden.contains(&"cmd".to_string()));
    }

    #[test]
    fn test_global_doesnt_override_project() {
        let registry = CommandRegistrySystem::new();
        
        let project_cmd = ClaudeCommand::new("cmd".to_string(), "Project version".to_string());
        let global_cmd = create_test_command("cmd");
        
        registry.register_command(project_cmd.clone(), CommandSource::Project).unwrap();
        registry.register_command(global_cmd, CommandSource::Global).unwrap();
        
        let retrieved = registry.get_command("cmd");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().description, "Project version");
    }

    #[test]
    fn test_search_commands() {
        let registry = CommandRegistrySystem::new();
        
        registry.register_command(create_test_command("test-one"), CommandSource::Global).unwrap();
        registry.register_command(create_test_command("test-two"), CommandSource::Global).unwrap();
        registry.register_command(create_test_command("other"), CommandSource::Global).unwrap();
        
        let results = registry.search_commands("test");
        assert_eq!(results.len(), 2);
        assert!(results.contains(&"test-one".to_string()));
        assert!(results.contains(&"test-two".to_string()));
    }

    #[test]
    fn test_fuzzy_search() {
        let registry = CommandRegistrySystem::new();
        
        registry.register_command(create_test_command("generate"), CommandSource::Global).unwrap();
        registry.register_command(create_test_command("gen-code"), CommandSource::Global).unwrap();
        registry.register_command(create_test_command("test"), CommandSource::Global).unwrap();
        
        let results = registry.fuzzy_search("gen");
        assert!(!results.is_empty());
        
        // "generate" should have high score for "gen" query
        let generate_result = results.iter().find(|(name, _)| name == "generate");
        assert!(generate_result.is_some());
    }

    #[test]
    fn test_usage_tracking() {
        let registry = CommandRegistrySystem::new();
        let cmd = create_test_command("tracked");
        
        registry.register_command(cmd, CommandSource::Global).unwrap();
        
        // Access the command multiple times
        registry.get_command("tracked");
        registry.get_command("tracked");
        registry.get_command("tracked");
        
        let registered = registry.get_registered_command("tracked").unwrap();
        assert_eq!(registered.usage_count, 3);
        assert!(registered.last_accessed.is_some());
    }

    #[test]
    fn test_stats() {
        let registry = CommandRegistrySystem::new();
        
        registry.register_command(create_test_command("global1"), CommandSource::Global).unwrap();
        registry.register_command(create_test_command("global2"), CommandSource::Global).unwrap();
        registry.register_command(create_test_command("project1"), CommandSource::Project).unwrap();
        
        // Use some commands
        registry.get_command("global1");
        registry.get_command("global1");
        registry.get_command("project1");
        
        let stats = registry.get_stats();
        assert_eq!(stats.total_commands, 3);
        assert_eq!(stats.global_commands, 2);
        assert_eq!(stats.project_commands, 1);
        assert!(!stats.top_commands.is_empty());
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;
        
        let registry = CommandRegistrySystem::new();
        let registry_clone1 = registry.clone();
        let registry_clone2 = registry.clone();
        
        // Register from multiple threads
        let handle1 = thread::spawn(move || {
            for i in 0..10 {
                let cmd = create_test_command(&format!("thread1-{}", i));
                registry_clone1.register_command(cmd, CommandSource::Global).unwrap();
            }
        });
        
        let handle2 = thread::spawn(move || {
            for i in 0..10 {
                let cmd = create_test_command(&format!("thread2-{}", i));
                registry_clone2.register_command(cmd, CommandSource::Project).unwrap();
            }
        });
        
        handle1.join().unwrap();
        handle2.join().unwrap();
        
        let all_commands = registry.list_commands();
        assert_eq!(all_commands.len(), 20);
    }

    #[test]
    fn test_similarity_calculation() {
        assert_eq!(calculate_similarity("test", "test"), 1.0);
        assert!(calculate_similarity("gen", "generate") > 0.5);
        assert!(calculate_similarity("test", "testing") > 0.5);
        assert!(calculate_similarity("abc", "xyz") < 0.3);
    }
}