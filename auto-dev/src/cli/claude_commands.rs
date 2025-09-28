//! Claude command CLI integration handler
//!
//! This module provides dynamic integration of Claude commands
//! into the CLI, allowing user-defined commands from .claude/commands/
//! to be executed as subcommands.

use anyhow::{Context, Result};
use auto_dev_core::claude::{
    ClaudeCommand, ClaudeConfigDiscovery, CommandParser, CommandRegistrySystem,
    CommandSource, CommandExecutor, CommandOutput,
};
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Handler for Claude commands in the CLI
pub struct ClaudeCommandHandler {
    /// Registry of discovered commands
    registry: CommandRegistrySystem,
    /// Discovery service for finding commands
    discovery: ClaudeConfigDiscovery,
    /// Cached commands for quick access
    commands_cache: HashMap<String, ClaudeCommand>,
}

impl ClaudeCommandHandler {
    /// Create a new Claude command handler
    pub async fn new() -> Result<Self> {
        let discovery = ClaudeConfigDiscovery::new();
        let registry = CommandRegistrySystem::new();
        
        // Discover configuration paths
        let paths = discovery.discover().await?;
        
        // Parse commands from discovered paths
        // Find .claude directories from the paths
        let mut global_claude_dir = None;
        let mut project_claude_dir = None;
        
        // Derive directories from CLAUDE.md paths
        if let Some(global_md) = &paths.global_claude_md {
            if let Some(parent) = global_md.parent() {
                global_claude_dir = Some(parent.to_path_buf());
            }
        }
        
        if let Some(project_md) = &paths.project_claude_md {
            if let Some(parent) = project_md.parent() {
                project_claude_dir = Some(parent.to_path_buf());
            }
        }
        
        // Register global commands if available
        if let Some(global_dir) = global_claude_dir {
            let commands_dir = global_dir.join("commands");
            if commands_dir.exists() {
                let mut parser = CommandParser::new();
                parser.parse_directory(&commands_dir)?;
                let global_registry = parser.into_registry();
                for cmd in global_registry.all_commands() {
                    registry.register_command(cmd.clone(), CommandSource::Global)?;
                }
            }
        }
        
        // Register project commands if available
        if let Some(project_dir) = project_claude_dir {
            let commands_dir = project_dir.join("commands");
            if commands_dir.exists() {
                let mut parser = CommandParser::new();
                parser.parse_directory(&commands_dir)?;
                let project_registry = parser.into_registry();
                for cmd in project_registry.all_commands() {
                    registry.register_command(cmd.clone(), CommandSource::Project)?;
                }
            }
        }
        
        // Build command cache
        let mut commands_cache = HashMap::new();
        for name in registry.list_commands() {
            if let Some(cmd) = registry.get_command(&name) {
                commands_cache.insert(name.clone(), cmd);
            }
        }
        
        Ok(Self {
            registry,
            discovery,
            commands_cache,
        })
    }
    
    /// Build clap subcommands from registered Claude commands
    pub fn build_subcommands(&self) -> Vec<Command> {
        let mut subcommands = Vec::new();
        
        for (name, cmd) in &self.commands_cache {
            let mut clap_cmd = Command::new(name.as_str())
                .about(cmd.description.clone())
                .long_about(self.extract_help_text(cmd).unwrap_or_else(|| cmd.usage.clone()));
            
            // Add arguments from command definition
            for arg in &cmd.arguments {
                let mut clap_arg = Arg::new(arg.name.as_str())
                    .help(arg.description.clone());
                
                // Set argument properties based on definition
                if arg.required {
                    clap_arg = clap_arg.required(true);
                }
                    
                    // Handle different argument types
                    match arg.arg_type.as_deref() {
                        Some("boolean") | Some("bool") => {
                            clap_arg = clap_arg
                                .action(ArgAction::SetTrue)
                                .num_args(0);
                        }
                        Some("array") | Some("list") => {
                            clap_arg = clap_arg
                                .action(ArgAction::Append)
                                .num_args(1..);
                        }
                        _ => {
                            clap_arg = clap_arg
                                .action(ArgAction::Set)
                                .num_args(1);
                        }
                    }
                    
                // Add long flag (default to argument name)
                clap_arg = clap_arg.long(arg.name.as_str());
                    
                // Set default value if specified
                if let Some(default) = &arg.default {
                    clap_arg = clap_arg.default_value(default.as_str());
                }
                
                clap_cmd = clap_cmd.arg(clap_arg);
            }
            
            subcommands.push(clap_cmd);
        }
        
        subcommands
    }
    
    /// Extract help text from command's markdown content
    fn extract_help_text(&self, cmd: &ClaudeCommand) -> Option<String> {
        // Use usage if available, otherwise use truncated instructions
        if !cmd.usage.is_empty() {
            Some(cmd.usage.clone())
        } else {
            let help_text = if cmd.instructions.len() > 500 {
                format!("{}...", &cmd.instructions[..497])
            } else {
                cmd.instructions.clone()
            };
            Some(help_text)
        }
    }
    
    /// Execute a Claude command with given arguments
    pub async fn execute_command(
        &self,
        name: &str,
        args: &ArgMatches,
    ) -> Result<()> {
        let cmd = self.commands_cache.get(name)
            .ok_or_else(|| anyhow::anyhow!("Command '{}' not found", name))?;
        
        info!("Executing Claude command: {}", name);
        debug!("Command arguments: {:?}", args);
        
        // Build arguments HashMap from ArgMatches
        let mut cmd_args = HashMap::new();
        
        for arg in &cmd.arguments {
                let value = if arg.arg_type.as_deref() == Some("array") {
                    // Handle array arguments
                    args.get_many::<String>(&arg.name)
                        .map(|values| values.cloned().collect::<Vec<_>>().join(","))
                        .unwrap_or_default()
                } else if arg.arg_type.as_deref() == Some("boolean") {
                    // Handle boolean arguments
                    args.get_flag(&arg.name).to_string()
                } else {
                    // Handle single value arguments
                    args.get_one::<String>(&arg.name)
                        .cloned()
                        .unwrap_or_else(|| arg.default.clone().unwrap_or_default())
                };
                
            cmd_args.insert(arg.name.clone(), value);
        }
        
        // Create command executor and run the command
        let executor = CommandExecutor::new(cmd.clone());
        let output = executor.execute(cmd_args).await
            .with_context(|| format!("Failed to execute command '{}'", name))?;
        
        // Print command output
        output.print();
        
        // Return error if command failed
        if !output.success {
            anyhow::bail!("Command '{}' failed with exit code {:?}", name, output.exit_code);
        }
        
        Ok(())
    }
    
    /// Get list of available command names
    pub fn list_command_names(&self) -> Vec<String> {
        self.registry.list_commands()
    }
    
    /// Search for commands matching a query
    pub fn search_commands(&self, query: &str) -> Vec<String> {
        self.registry.search_commands(query)
    }
    
    /// Get command by name
    pub fn get_command(&self, name: &str) -> Option<ClaudeCommand> {
        self.registry.get_command(name)
    }
}


/// Dynamic command builder for clap
pub struct DynamicCommandBuilder;

impl DynamicCommandBuilder {
    /// Build a clap Command with Claude commands dynamically added
    pub async fn build_with_claude_commands(mut app: Command) -> Result<Command> {
        // Try to initialize Claude commands
        match ClaudeCommandHandler::new().await {
            Ok(handler) => {
                info!("Claude commands discovered: {}", handler.list_command_names().len());
                
                // Add Claude subcommand that contains all discovered commands
                if !handler.commands_cache.is_empty() {
                    let mut claude_cmd = Command::new("claude")
                        .about("Execute Claude user-defined commands")
                        .subcommand_required(true)
                        .arg_required_else_help(true);
                    
                    // Add all discovered commands as subcommands
                    for subcmd in handler.build_subcommands() {
                        claude_cmd = claude_cmd.subcommand(subcmd);
                    }
                    
                    app = app.subcommand(claude_cmd);
                }
            }
            Err(e) => {
                // Log warning but don't fail the CLI
                warn!("Failed to load Claude commands: {}", e);
                debug!("Claude commands will not be available in this session");
            }
        }
        
        Ok(app)
    }
    
    /// Check if a command is a Claude command
    pub fn is_claude_command(name: &str) -> bool {
        // Check if this is the claude parent command or a subcommand
        name == "claude" || name.starts_with("claude.")
    }
}

/// Integration helper for main CLI
pub async fn integrate_claude_commands(app: Command) -> Result<Command> {
    DynamicCommandBuilder::build_with_claude_commands(app).await
}

/// Execute a Claude command from CLI matches
pub async fn execute_claude_command(matches: &ArgMatches) -> Result<()> {
    // Get the claude subcommand matches
    let (name, sub_matches) = matches.subcommand()
        .ok_or_else(|| anyhow::anyhow!("No Claude subcommand specified"))?;
    
    // Initialize handler and execute
    let handler = ClaudeCommandHandler::new().await?;
    handler.execute_command(name, sub_matches).await
}

/// Handle Claude command from main CLI
pub async fn handle_claude_command(command: String, args: Vec<String>) -> Result<()> {
    // Initialize handler
    let handler = match ClaudeCommandHandler::new().await {
        Ok(h) => h,
        Err(e) => {
            // If no Claude commands are available, provide helpful message
            eprintln!("Failed to load Claude commands: {}", e);
            eprintln!("\nTo use Claude commands:");
            eprintln!("1. Create a .claude directory in your project or home directory");
            eprintln!("2. Add command files to .claude/commands/");
            eprintln!("3. See documentation for command file format");
            return Err(e);
        }
    };
    
    // Check if command exists
    let cmd = handler.get_command(&command)
        .ok_or_else(|| {
            let available = handler.list_command_names();
            if available.is_empty() {
                anyhow::anyhow!("No Claude commands available. Create command files in .claude/commands/")
            } else {
                let suggestions = handler.search_commands(&command);
                if suggestions.is_empty() {
                    anyhow::anyhow!(
                        "Unknown command '{}'. Available commands: {}",
                        command,
                        available.join(", ")
                    )
                } else {
                    anyhow::anyhow!(
                        "Unknown command '{}'. Did you mean one of: {}?",
                        command,
                        suggestions.join(", ")
                    )
                }
            }
        })?;
    
    // Parse arguments into ArgMatches for the specific command
    let mut cmd_app = Command::new(command.as_str())
        .about(cmd.description.clone());
    
    // Add argument definitions
    for arg in &cmd.arguments {
        let mut clap_arg = Arg::new(arg.name.as_str())
            .help(arg.description.clone());
        
        if arg.required {
            clap_arg = clap_arg.required(true);
        }
            
            match arg.arg_type.as_deref() {
                Some("boolean") | Some("bool") => {
                    clap_arg = clap_arg
                        .action(ArgAction::SetTrue)
                        .num_args(0);
                }
                Some("array") | Some("list") => {
                    clap_arg = clap_arg
                        .action(ArgAction::Append)
                        .num_args(1..);
                }
                _ => {
                    clap_arg = clap_arg
                        .action(ArgAction::Set)
                        .num_args(1);
                }
            }
            
            clap_arg = clap_arg.long(arg.name.as_str());
            
        if let Some(default) = &arg.default {
            clap_arg = clap_arg.default_value(default.as_str());
        }
        
        cmd_app = cmd_app.arg(clap_arg);
    }
    
    // Parse the provided arguments
    let matches = cmd_app.try_get_matches_from(
        std::iter::once(command.clone()).chain(args.into_iter())
    )?;
    
    // Execute the command
    handler.execute_command(&command, &matches).await
}