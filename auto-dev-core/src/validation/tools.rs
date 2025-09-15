//! Tool discovery and execution system for validation
//!
//! Dynamically discovers and executes validation tools at runtime,
//! allowing for flexible tool installation and configuration.

use crate::validation::{
    ErrorCategory, Improvement, Priority, ValidationResult, ValidationWarning,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Represents a validation tool that can be discovered and executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationTool {
    /// Name of the tool (e.g., "clippy", "cargo-audit")
    pub name: String,

    /// Command to check if tool is installed
    pub check_command: Vec<String>,

    /// Command to execute the tool
    pub run_command: Vec<String>,

    /// Category of validation this tool performs
    pub category: ToolCategory,

    /// Priority of this tool (higher priority tools run first)
    pub priority: u8,

    /// Whether this tool is required (vs optional)
    pub required: bool,

    /// Installation instructions if tool is missing
    pub install_instructions: Option<String>,

    /// Parser for the tool's output
    pub output_parser: OutputParserType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolCategory {
    Compilation,
    Testing,
    Linting,
    Security,
    Performance,
    Documentation,
    Dependencies,
    Formatting,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputParserType {
    Json,
    Regex(String),
    LineByLine,
    ExitCode,
    Custom(String),
}

/// Tool registry that manages available validation tools
pub struct ToolRegistry {
    tools: HashMap<String, ValidationTool>,
    discovered_tools: HashMap<String, bool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self { tools: HashMap::new(), discovered_tools: HashMap::new() };

        // Register default tools (but don't require them)
        registry.register_default_tools();
        registry
    }

    /// Register default Rust ecosystem tools
    fn register_default_tools(&mut self) {
        // Core cargo tools (likely to be available)
        self.register_tool(ValidationTool {
            name: "cargo-check".to_string(),
            check_command: vec!["cargo".to_string(), "check".to_string(), "--version".to_string()],
            run_command: vec![
                "cargo".to_string(),
                "check".to_string(),
                "--all-targets".to_string(),
            ],
            category: ToolCategory::Compilation,
            priority: 100,
            required: false,
            install_instructions: None,
            output_parser: OutputParserType::ExitCode,
        });

        self.register_tool(ValidationTool {
            name: "cargo-test".to_string(),
            check_command: vec!["cargo".to_string(), "test".to_string(), "--version".to_string()],
            run_command: vec!["cargo".to_string(), "test".to_string(), "--all".to_string()],
            category: ToolCategory::Testing,
            priority: 90,
            required: false,
            install_instructions: None,
            output_parser: OutputParserType::LineByLine,
        });

        // Optional but recommended tools
        self.register_tool(ValidationTool {
            name: "clippy".to_string(),
            check_command: vec!["cargo".to_string(), "clippy".to_string(), "--version".to_string()],
            run_command: vec![
                "cargo".to_string(),
                "clippy".to_string(),
                "--all-targets".to_string(),
                "--".to_string(),
                "-W".to_string(),
                "clippy::all".to_string(),
            ],
            category: ToolCategory::Linting,
            priority: 80,
            required: false,
            install_instructions: Some("rustup component add clippy".to_string()),
            output_parser: OutputParserType::Regex(r"(?:warning|error): (.+)".to_string()),
        });

        self.register_tool(ValidationTool {
            name: "rustfmt".to_string(),
            check_command: vec!["cargo".to_string(), "fmt".to_string(), "--version".to_string()],
            run_command: vec![
                "cargo".to_string(),
                "fmt".to_string(),
                "--".to_string(),
                "--check".to_string(),
            ],
            category: ToolCategory::Formatting,
            priority: 70,
            required: false,
            install_instructions: Some("rustup component add rustfmt".to_string()),
            output_parser: OutputParserType::ExitCode,
        });

        // Optional third-party tools
        self.register_tool(ValidationTool {
            name: "cargo-audit".to_string(),
            check_command: vec!["cargo".to_string(), "audit".to_string(), "--version".to_string()],
            run_command: vec!["cargo".to_string(), "audit".to_string()],
            category: ToolCategory::Security,
            priority: 60,
            required: false,
            install_instructions: Some("cargo install cargo-audit".to_string()),
            output_parser: OutputParserType::Json,
        });

        self.register_tool(ValidationTool {
            name: "cargo-outdated".to_string(),
            check_command: vec![
                "cargo".to_string(),
                "outdated".to_string(),
                "--version".to_string(),
            ],
            run_command: vec![
                "cargo".to_string(),
                "outdated".to_string(),
                "--format".to_string(),
                "json".to_string(),
            ],
            category: ToolCategory::Dependencies,
            priority: 50,
            required: false,
            install_instructions: Some("cargo install cargo-outdated".to_string()),
            output_parser: OutputParserType::Json,
        });

        self.register_tool(ValidationTool {
            name: "cargo-deny".to_string(),
            check_command: vec!["cargo".to_string(), "deny".to_string(), "--version".to_string()],
            run_command: vec!["cargo".to_string(), "deny".to_string(), "check".to_string()],
            category: ToolCategory::Security,
            priority: 55,
            required: false,
            install_instructions: Some("cargo install cargo-deny".to_string()),
            output_parser: OutputParserType::LineByLine,
        });

        self.register_tool(ValidationTool {
            name: "cargo-geiger".to_string(),
            check_command: vec!["cargo".to_string(), "geiger".to_string(), "--version".to_string()],
            run_command: vec![
                "cargo".to_string(),
                "geiger".to_string(),
                "--output-format".to_string(),
                "json".to_string(),
            ],
            category: ToolCategory::Security,
            priority: 45,
            required: false,
            install_instructions: Some("cargo install cargo-geiger".to_string()),
            output_parser: OutputParserType::Json,
        });

        self.register_tool(ValidationTool {
            name: "cargo-tarpaulin".to_string(),
            check_command: vec![
                "cargo".to_string(),
                "tarpaulin".to_string(),
                "--version".to_string(),
            ],
            run_command: vec![
                "cargo".to_string(),
                "tarpaulin".to_string(),
                "--print-summary".to_string(),
            ],
            category: ToolCategory::Testing,
            priority: 40,
            required: false,
            install_instructions: Some("cargo install cargo-tarpaulin".to_string()),
            output_parser: OutputParserType::Regex(r"(\d+\.\d+)% coverage".to_string()),
        });

        self.register_tool(ValidationTool {
            name: "tokei".to_string(),
            check_command: vec!["tokei".to_string(), "--version".to_string()],
            run_command: vec!["tokei".to_string(), "--output".to_string(), "json".to_string()],
            category: ToolCategory::Custom("metrics".to_string()),
            priority: 30,
            required: false,
            install_instructions: Some("cargo install tokei".to_string()),
            output_parser: OutputParserType::Json,
        });
    }

    /// Register a new tool
    pub fn register_tool(&mut self, tool: ValidationTool) {
        self.tools.insert(tool.name.clone(), tool);
    }

    /// Register a custom tool from configuration
    pub fn register_custom_tool(&mut self, config: CustomToolConfig) {
        let tool = ValidationTool {
            name: config.name,
            check_command: config.check_command,
            run_command: config.run_command,
            category: config
                .category
                .map(ToolCategory::Custom)
                .unwrap_or(ToolCategory::Custom("custom".to_string())),
            priority: config.priority.unwrap_or(10),
            required: config.required.unwrap_or(false),
            install_instructions: config.install_instructions,
            output_parser: config.output_parser.unwrap_or(OutputParserType::ExitCode),
        };
        self.register_tool(tool);
    }

    /// Discover which tools are available on the system
    pub async fn discover_tools(&mut self) -> Result<DiscoveryReport> {
        let mut available = Vec::new();
        let mut missing = Vec::new();

        for (name, tool) in &self.tools {
            let is_available = self.check_tool_availability(tool).await?;
            self.discovered_tools.insert(name.clone(), is_available);

            if is_available {
                available.push(name.clone());
            } else {
                missing.push(MissingTool {
                    name: name.clone(),
                    required: tool.required,
                    install_instructions: tool.install_instructions.clone(),
                });
            }
        }

        Ok(DiscoveryReport {
            available_tools: available,
            missing_tools: missing,
            total_registered: self.tools.len(),
        })
    }

    /// Check if a specific tool is available
    async fn check_tool_availability(&self, tool: &ValidationTool) -> Result<bool> {
        if tool.check_command.is_empty() {
            return Ok(false);
        }

        let output = Command::new(&tool.check_command[0]).args(&tool.check_command[1..]).output();

        Ok(output.is_ok() && output.unwrap().status.success())
    }

    /// Run all available tools
    pub async fn run_available_tools(&self, project_path: &str) -> Result<ValidationResult> {
        let mut results = Vec::new();
        let mut tools_to_run: Vec<_> = self
            .tools
            .values()
            .filter(|tool| self.discovered_tools.get(&tool.name).copied().unwrap_or(false))
            .collect();

        // Sort by priority (higher priority first)
        tools_to_run.sort_by(|a, b| b.priority.cmp(&a.priority));

        for tool in tools_to_run {
            match self.run_tool(tool, project_path).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    // Log error but continue with other tools
                    eprintln!("Tool {} failed: {}", tool.name, e);
                }
            }
        }

        Ok(ValidationResult::aggregate(results))
    }

    /// Run a specific tool
    async fn run_tool(
        &self,
        tool: &ValidationTool,
        project_path: &str,
    ) -> Result<ValidationResult> {
        if tool.run_command.is_empty() {
            return Ok(ValidationResult::new());
        }

        let output = Command::new(&tool.run_command[0])
            .args(&tool.run_command[1..])
            .current_dir(project_path)
            .output()?;

        // Parse output based on parser type
        let mut result = ValidationResult::new();

        match &tool.output_parser {
            OutputParserType::ExitCode => {
                if !output.status.success() {
                    result.warnings.push(ValidationWarning {
                        category: self.category_to_error_category(&tool.category),
                        message: format!("{} check failed", tool.name),
                        location: None,
                    });
                }
            }
            OutputParserType::Json => {
                // Parse JSON output (simplified)
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(_json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    // Process JSON based on tool type
                    // This would be tool-specific
                }
            }
            OutputParserType::Regex(pattern) => {
                let regex = regex::Regex::new(pattern)?;
                let stderr_text = String::from_utf8_lossy(&output.stderr);
                let stdout_text = String::from_utf8_lossy(&output.stdout);
                let output_text = format!("{}{}", stderr_text, stdout_text);

                for cap in regex.captures_iter(&output_text) {
                    if let Some(message) = cap.get(1) {
                        result.warnings.push(ValidationWarning {
                            category: self.category_to_error_category(&tool.category),
                            message: message.as_str().to_string(),
                            location: None,
                        });
                    }
                }
            }
            OutputParserType::LineByLine => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                for line in stderr.lines() {
                    if line.contains("error") || line.contains("warning") {
                        result.warnings.push(ValidationWarning {
                            category: self.category_to_error_category(&tool.category),
                            message: line.to_string(),
                            location: None,
                        });
                    }
                }
            }
            OutputParserType::Custom(parser_name) => {
                // Would call a custom parser function based on name
                // This could be extended with a plugin system
            }
        }

        Ok(result)
    }

    /// Convert tool category to error category
    fn category_to_error_category(&self, category: &ToolCategory) -> ErrorCategory {
        match category {
            ToolCategory::Compilation => ErrorCategory::Compilation,
            ToolCategory::Testing => ErrorCategory::Test,
            ToolCategory::Linting => ErrorCategory::Quality,
            ToolCategory::Security => ErrorCategory::Security,
            ToolCategory::Performance => ErrorCategory::Performance,
            ToolCategory::Documentation => ErrorCategory::Quality,
            ToolCategory::Dependencies => ErrorCategory::Quality,
            ToolCategory::Formatting => ErrorCategory::Standards,
            ToolCategory::Custom(_) => ErrorCategory::Quality,
        }
    }

    /// Get recommendations for missing tools
    pub fn get_tool_recommendations(&self, report: &DiscoveryReport) -> Vec<Improvement> {
        let mut recommendations = Vec::new();

        for missing in &report.missing_tools {
            if let Some(instructions) = &missing.install_instructions {
                let priority = if missing.required {
                    Priority::High
                } else if report.available_tools.len() < 3 {
                    Priority::Medium
                } else {
                    Priority::Low
                };

                recommendations.push(Improvement {
                    category: "tools".to_string(),
                    description: format!("Install {} with: {}", missing.name, instructions),
                    priority,
                });
            }
        }

        recommendations
    }
}

/// Report from tool discovery
#[derive(Debug, Clone)]
pub struct DiscoveryReport {
    pub available_tools: Vec<String>,
    pub missing_tools: Vec<MissingTool>,
    pub total_registered: usize,
}

#[derive(Debug, Clone)]
pub struct MissingTool {
    pub name: String,
    pub required: bool,
    pub install_instructions: Option<String>,
}

/// Configuration for custom tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomToolConfig {
    pub name: String,
    pub check_command: Vec<String>,
    pub run_command: Vec<String>,
    pub category: Option<String>,
    pub priority: Option<u8>,
    pub required: Option<bool>,
    pub install_instructions: Option<String>,
    pub output_parser: Option<OutputParserType>,
}

/// Tool configuration loaded from file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig {
    /// Additional tools to register
    pub custom_tools: Vec<CustomToolConfig>,

    /// Tools to disable from defaults
    pub disabled_tools: Vec<String>,

    /// Override settings for default tools
    pub tool_overrides: HashMap<String, ToolOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOverride {
    pub run_command: Option<Vec<String>>,
    pub priority: Option<u8>,
    pub required: Option<bool>,
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            custom_tools: Vec::new(),
            disabled_tools: Vec::new(),
            tool_overrides: HashMap::new(),
        }
    }
}

/// Load tools configuration from a file
pub async fn load_tools_config(path: &Path) -> Result<ToolsConfig> {
    if !path.exists() {
        return Ok(ToolsConfig::default());
    }

    let content = tokio::fs::read_to_string(path).await?;
    let config: ToolsConfig = toml::from_str(&content)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_registry_creation() {
        let registry = ToolRegistry::new();
        assert!(!registry.tools.is_empty());
    }

    #[test]
    fn test_custom_tool_registration() {
        let mut registry = ToolRegistry::new();
        let initial_count = registry.tools.len();

        registry.register_custom_tool(CustomToolConfig {
            name: "my-tool".to_string(),
            check_command: vec!["my-tool".to_string(), "--version".to_string()],
            run_command: vec!["my-tool".to_string(), "check".to_string()],
            category: Some("custom".to_string()),
            priority: Some(50),
            required: Some(false),
            install_instructions: Some("cargo install my-tool".to_string()),
            output_parser: Some(OutputParserType::ExitCode),
        });

        assert_eq!(registry.tools.len(), initial_count + 1);
        assert!(registry.tools.contains_key("my-tool"));
    }
}
