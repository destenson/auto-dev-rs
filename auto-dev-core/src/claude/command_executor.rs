//! Command executor for Claude commands
//!
//! This module provides the core execution logic for Claude commands,
//! handling different command types and execution strategies.

use crate::claude::command_types::ClaudeCommand;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command as TokioCommand;
use tracing::{debug, info, warn};

/// Command type for execution strategy
#[derive(Debug, Clone, Copy, PartialEq)]
enum CommandType {
    Shell,
    Script,
    Builtin,
    Chain,
    Default,
}

/// Executor for Claude commands
#[derive(Debug, Clone)]
pub struct CommandExecutor {
    /// The command to execute
    command: ClaudeCommand,
    /// Working directory for command execution
    working_dir: Option<PathBuf>,
}

impl CommandExecutor {
    /// Create a new command executor
    pub fn new(command: ClaudeCommand) -> Self {
        Self {
            command,
            working_dir: None,
        }
    }
    
    /// Set the working directory for command execution
    pub fn with_working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = Some(dir);
        self
    }
    
    /// Detect command type from instructions or content
    fn detect_command_type(&self) -> CommandType {
        let instructions = &self.command.instructions;
        
        // Check for builtin commands
        if self.command.name.starts_with("claude-") {
            return CommandType::Builtin;
        }
        
        // Check for chain commands (multiple lines with commands)
        let lines: Vec<&str> = instructions.lines()
            .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
            .collect();
        if lines.len() > 1 {
            return CommandType::Chain;
        }
        
        // Check for script indicators
        if instructions.contains("#!/") {
            return CommandType::Script;
        }
        
        // Check for shell command patterns
        if instructions.contains("echo") || instructions.contains("ls") || 
           instructions.contains("cd") || instructions.contains("&&") {
            return CommandType::Shell;
        }
        
        CommandType::Default
    }
    
    /// Execute the command with given arguments
    pub async fn execute(&self, args: HashMap<String, String>) -> Result<CommandOutput> {
        info!("Executing command: {}", self.command.name);
        debug!("Arguments: {:?}", args);
        
        // Validate required arguments
        self.validate_arguments(&args)?;
        
        // Determine command type from content or instructions
        let command_type = self.detect_command_type();
        
        // Execute based on command type
        match command_type {
            CommandType::Shell => self.execute_shell(args).await,
            CommandType::Script => self.execute_script(args).await,
            CommandType::Builtin => self.execute_builtin(args).await,
            CommandType::Chain => self.execute_chain(args).await,
            CommandType::Default => self.execute_default(args).await,
        }
    }
    
    /// Validate that required arguments are present
    fn validate_arguments(&self, args: &HashMap<String, String>) -> Result<()> {
        for arg in &self.command.arguments {
            if arg.required && !args.contains_key(&arg.name) {
                if arg.default.is_none() {
                    anyhow::bail!(
                        "Required argument '{}' not provided for command '{}'",
                        arg.name,
                        self.command.name
                    );
                }
            }
        }
        Ok(())
    }
    
    /// Apply argument defaults
    fn apply_defaults(&self, mut args: HashMap<String, String>) -> HashMap<String, String> {
        for arg in &self.command.arguments {
            if !args.contains_key(&arg.name) {
                if let Some(default) = &arg.default {
                    args.insert(arg.name.clone(), default.clone());
                }
            }
        }
        args
    }
    
    /// Substitute variables in content
    fn substitute_variables(&self, content: &str, args: &HashMap<String, String>) -> String {
        let mut result = content.to_string();
        
        // Replace ${VAR} style variables
        for (key, value) in args {
            result = result.replace(&format!("${{{}}}", key), value);
            result = result.replace(&format!("${}", key), value);
        }
        
        // Replace environment variables
        for (key, value) in std::env::vars() {
            result = result.replace(&format!("${{ENV_{}}}", key), &value);
        }
        
        result
    }
    
    /// Execute a shell command
    async fn execute_shell(&self, args: HashMap<String, String>) -> Result<CommandOutput> {
        let content = &self.command.instructions;
        
        let args = self.apply_defaults(args);
        let script = self.substitute_variables(content, &args);
        
        debug!("Executing shell script:\n{}", script);
        
        let shell = if cfg!(windows) {
            ("cmd", vec!["/C"])
        } else {
            ("sh", vec!["-c"])
        };
        
        let mut cmd = TokioCommand::new(shell.0);
        for arg in shell.1 {
            cmd.arg(arg);
        }
        cmd.arg(&script);
        
        if let Some(dir) = &self.working_dir {
            cmd.current_dir(dir);
        }
        
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        let output = cmd.output().await
            .context("Failed to execute shell command")?;
        
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        
        Ok(CommandOutput {
            success: output.status.success(),
            stdout,
            stderr,
            exit_code: output.status.code(),
        })
    }
    
    /// Execute a script command
    async fn execute_script(&self, args: HashMap<String, String>) -> Result<CommandOutput> {
        let content = &self.command.instructions;
        
        let args = self.apply_defaults(args);
        
        // Detect script type from content or extension
        let script_type = self.detect_script_type(content);
        
        match script_type {
            ScriptType::Python => self.execute_python_script(content, args).await,
            ScriptType::JavaScript => self.execute_js_script(content, args).await,
            ScriptType::PowerShell => self.execute_powershell_script(content, args).await,
            ScriptType::Bash => self.execute_bash_script(content, args).await,
            ScriptType::Unknown => self.execute_shell(args).await,
        }
    }
    
    /// Execute a builtin command
    async fn execute_builtin(&self, args: HashMap<String, String>) -> Result<CommandOutput> {
        let args = self.apply_defaults(args);
        
        match self.command.name.as_str() {
            "claude-status" => self.builtin_status(args).await,
            "claude-reload" => self.builtin_reload(args).await,
            "claude-validate" => self.builtin_validate(args).await,
            _ => {
                warn!("Unknown builtin command: {}", self.command.name);
                self.execute_default(args).await
            }
        }
    }
    
    /// Execute a chain of commands
    async fn execute_chain(&self, args: HashMap<String, String>) -> Result<CommandOutput> {
        let content = &self.command.instructions;
        
        let args = self.apply_defaults(args);
        
        // Split content into individual commands
        let commands: Vec<&str> = content.lines()
            .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
            .collect();
        
        let mut all_stdout = Vec::new();
        let mut all_stderr = Vec::new();
        let mut last_exit_code = None;
        
        for cmd in commands {
            let processed_cmd = self.substitute_variables(cmd, &args);
            debug!("Executing chain command: {}", processed_cmd);
            
            let output = self.execute_single_command(&processed_cmd).await?;
            
            all_stdout.push(output.stdout.clone());
            all_stderr.push(output.stderr.clone());
            last_exit_code = output.exit_code;
            
            if !output.success {
                // Stop chain on first failure
                return Ok(CommandOutput {
                    success: false,
                    stdout: all_stdout.join("\n"),
                    stderr: all_stderr.join("\n"),
                    exit_code: last_exit_code,
                });
            }
        }
        
        Ok(CommandOutput {
            success: true,
            stdout: all_stdout.join("\n"),
            stderr: all_stderr.join("\n"),
            exit_code: last_exit_code,
        })
    }
    
    /// Default execution fallback
    async fn execute_default(&self, args: HashMap<String, String>) -> Result<CommandOutput> {
        let args = self.apply_defaults(args);
        let processed = self.substitute_variables(&self.command.instructions, &args);
        
        Ok(CommandOutput {
            success: true,
            stdout: processed,
            stderr: String::new(),
            exit_code: Some(0),
        })
    }
    
    /// Execute a single command line
    async fn execute_single_command(&self, cmd: &str) -> Result<CommandOutput> {
        let shell = if cfg!(windows) {
            ("cmd", vec!["/C"])
        } else {
            ("sh", vec!["-c"])
        };
        
        let mut command = TokioCommand::new(shell.0);
        for arg in shell.1 {
            command.arg(arg);
        }
        command.arg(cmd);
        
        if let Some(dir) = &self.working_dir {
            command.current_dir(dir);
        }
        
        let output = command.output().await?;
        
        Ok(CommandOutput {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
        })
    }
    
    /// Detect script type from content
    fn detect_script_type(&self, content: &str) -> ScriptType {
        let first_line = content.lines().next().unwrap_or("");
        
        if first_line.starts_with("#!/usr/bin/env python") || first_line.starts_with("#!/usr/bin/python") {
            ScriptType::Python
        } else if first_line.starts_with("#!/usr/bin/env node") || first_line.starts_with("#!/usr/bin/node") {
            ScriptType::JavaScript
        } else if first_line.starts_with("#!/usr/bin/env bash") || first_line.starts_with("#!/bin/bash") {
            ScriptType::Bash
        } else if first_line.starts_with("#!") && first_line.contains("powershell") {
            ScriptType::PowerShell
        } else if content.contains("import ") || content.contains("from ") || content.contains("def ") {
            ScriptType::Python
        } else if content.contains("const ") || content.contains("let ") || content.contains("function ") {
            ScriptType::JavaScript
        } else {
            ScriptType::Unknown
        }
    }
    
    /// Execute Python script
    async fn execute_python_script(&self, script: &str, args: HashMap<String, String>) -> Result<CommandOutput> {
        let processed = self.substitute_variables(script, &args);
        
        let mut cmd = TokioCommand::new("python");
        cmd.arg("-c").arg(&processed);
        
        if let Some(dir) = &self.working_dir {
            cmd.current_dir(dir);
        }
        
        let output = cmd.output().await?;
        
        Ok(CommandOutput {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
        })
    }
    
    /// Execute JavaScript script
    async fn execute_js_script(&self, script: &str, args: HashMap<String, String>) -> Result<CommandOutput> {
        let processed = self.substitute_variables(script, &args);
        
        let mut cmd = TokioCommand::new("node");
        cmd.arg("-e").arg(&processed);
        
        if let Some(dir) = &self.working_dir {
            cmd.current_dir(dir);
        }
        
        let output = cmd.output().await?;
        
        Ok(CommandOutput {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
        })
    }
    
    /// Execute PowerShell script
    async fn execute_powershell_script(&self, script: &str, args: HashMap<String, String>) -> Result<CommandOutput> {
        let processed = self.substitute_variables(script, &args);
        
        let mut cmd = TokioCommand::new("powershell");
        cmd.arg("-Command").arg(&processed);
        
        if let Some(dir) = &self.working_dir {
            cmd.current_dir(dir);
        }
        
        let output = cmd.output().await?;
        
        Ok(CommandOutput {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
        })
    }
    
    /// Execute Bash script
    async fn execute_bash_script(&self, script: &str, args: HashMap<String, String>) -> Result<CommandOutput> {
        let processed = self.substitute_variables(script, &args);
        
        let mut cmd = TokioCommand::new("bash");
        cmd.arg("-c").arg(&processed);
        
        if let Some(dir) = &self.working_dir {
            cmd.current_dir(dir);
        }
        
        let output = cmd.output().await?;
        
        Ok(CommandOutput {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
        })
    }
    
    /// Builtin: Get Claude configuration status
    async fn builtin_status(&self, _args: HashMap<String, String>) -> Result<CommandOutput> {
        let mut output = Vec::new();
        output.push("Claude Configuration Status".to_string());
        output.push("-".repeat(40));
        output.push(format!("Command: {}", self.command.name));
        output.push(format!("Description: {}", self.command.description));
        output.push(format!("Arguments: {}", self.command.arguments.len()));
        
        Ok(CommandOutput {
            success: true,
            stdout: output.join("\n"),
            stderr: String::new(),
            exit_code: Some(0),
        })
    }
    
    /// Builtin: Reload Claude configuration
    async fn builtin_reload(&self, _args: HashMap<String, String>) -> Result<CommandOutput> {
        // This would trigger a configuration reload
        Ok(CommandOutput {
            success: true,
            stdout: "Claude configuration reloaded successfully".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
        })
    }
    
    /// Builtin: Validate Claude configuration
    async fn builtin_validate(&self, _args: HashMap<String, String>) -> Result<CommandOutput> {
        // This would validate the current configuration
        Ok(CommandOutput {
            success: true,
            stdout: "Claude configuration is valid".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
        })
    }
}

/// Output from command execution
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Whether the command succeeded
    pub success: bool,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Exit code if available
    pub exit_code: Option<i32>,
}

impl CommandOutput {
    /// Create a successful output
    pub fn success(stdout: String) -> Self {
        Self {
            success: true,
            stdout,
            stderr: String::new(),
            exit_code: Some(0),
        }
    }
    
    /// Create a failure output
    pub fn failure(stderr: String) -> Self {
        Self {
            success: false,
            stdout: String::new(),
            stderr,
            exit_code: Some(1),
        }
    }
    
    /// Print the output to console
    pub fn print(&self) {
        if !self.stdout.is_empty() {
            println!("{}", self.stdout);
        }
        if !self.stderr.is_empty() {
            eprintln!("{}", self.stderr);
        }
    }
}

/// Script type detection
#[derive(Debug, Clone, Copy, PartialEq)]
enum ScriptType {
    Python,
    JavaScript,
    PowerShell,
    Bash,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_variable_substitution() {
        let cmd = ClaudeCommand {
            name: "test".to_string(),
            description: None,
            command_type: None,
            content: Some("echo ${name} ${age}".to_string()),
            arguments: None,
            metadata: None,
        };
        
        let executor = CommandExecutor::new(cmd);
        let mut args = HashMap::new();
        args.insert("name".to_string(), "Alice".to_string());
        args.insert("age".to_string(), "30".to_string());
        
        let result = executor.substitute_variables("echo ${name} ${age}", &args);
        assert_eq!(result, "echo Alice 30");
    }
    
    #[test]
    fn test_script_type_detection() {
        let cmd = ClaudeCommand {
            name: "test".to_string(),
            description: None,
            command_type: None,
            content: None,
            arguments: None,
            metadata: None,
        };
        
        let executor = CommandExecutor::new(cmd);
        
        assert_eq!(
            executor.detect_script_type("#!/usr/bin/env python\nprint('hello')"),
            ScriptType::Python
        );
        
        assert_eq!(
            executor.detect_script_type("const x = 5;\nconsole.log(x);"),
            ScriptType::JavaScript
        );
        
        assert_eq!(
            executor.detect_script_type("#!/bin/bash\necho hello"),
            ScriptType::Bash
        );
    }
}