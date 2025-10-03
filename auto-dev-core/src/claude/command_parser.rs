//! Parser for Claude command markdown files
//!
//! This module handles parsing of command files from .claude/commands/ directory.

use crate::claude::command_types::{ClaudeCommand, CommandArgument, CommandRegistry};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Parser for Claude command files
pub struct CommandParser {
    /// Parsed commands
    registry: CommandRegistry,
}

impl CommandParser {
    /// Create a new command parser
    pub fn new() -> Self {
        Self { registry: CommandRegistry::new() }
    }

    /// Parse all command files in a directory
    pub fn parse_directory(&mut self, dir: &Path) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(dir)
            .with_context(|| format!("Failed to read command directory: {}", dir.display()))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "md") {
                if let Err(e) = self.parse_file(&path) {
                    eprintln!("Warning: Failed to parse command file {}: {}", path.display(), e);
                }
            }
        }

        self.registry.metadata.last_loaded = Some(std::time::SystemTime::now());
        self.registry.metadata.sources.push(dir.to_string_lossy().into_owned());

        Ok(())
    }

    /// Parse a single command file
    pub fn parse_file(&mut self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        let command = self.parse_command_content(path, &content)?;
        self.registry.add_command(command);

        Ok(())
    }

    /// Parse command content from markdown
    fn parse_command_content(&self, path: &Path, content: &str) -> Result<ClaudeCommand> {
        let name = extract_command_name(path)?;
        validate_command_name(&name)?;

        let mut command = ClaudeCommand::new(name, String::new());
        command.raw_content = content.to_string();

        let sections = parse_markdown_sections(content);

        // Extract description (first non-empty paragraph before any section)
        command.description = extract_description(content);

        // Extract usage section
        if let Some(usage_content) = sections.get("usage") {
            command.usage = usage_content.trim().to_string();
            command.arguments.extend(extract_arguments_from_usage(usage_content));
        }

        // Extract arguments section
        if let Some(args_content) = sections.get("arguments") {
            let args = parse_arguments_section(args_content);
            // Merge with usage-based arguments, preferring explicit definitions
            for arg in args {
                if !command.arguments.iter().any(|a| a.name == arg.name) {
                    command.arguments.push(arg);
                }
            }
        }

        // Extract examples section
        if let Some(examples_content) = sections.get("examples") {
            command.examples = extract_examples(examples_content);
        }

        // Extract instructions (implementation/execution section or full content)
        command.instructions = sections
            .get("implementation")
            .or_else(|| sections.get("execution"))
            .or_else(|| sections.get("instructions"))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| content.to_string());

        Ok(command)
    }

    /// Get the command registry
    pub fn registry(&self) -> &CommandRegistry {
        &self.registry
    }

    /// Take ownership of the registry
    pub fn into_registry(self) -> CommandRegistry {
        self.registry
    }
}

/// Extract command name from file path
fn extract_command_name(path: &Path) -> Result<String> {
    let file_stem = path.file_stem().and_then(|s| s.to_str()).context("Invalid file name")?;

    Ok(file_stem.to_string())
}

/// Validate command name
fn validate_command_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Command name cannot be empty");
    }

    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        anyhow::bail!(
            "Command name can only contain alphanumeric characters, hyphens, and underscores"
        );
    }

    Ok(())
}

/// Parse markdown into sections based on headers
fn parse_markdown_sections(content: &str) -> HashMap<String, String> {
    let mut sections = HashMap::new();
    let mut current_section = None;
    let mut section_content = String::new();

    for line in content.lines() {
        if line.starts_with("##") {
            // Save previous section if exists
            if let Some(section_name) = current_section.take() {
                sections.insert(section_name, section_content.trim().to_string());
                section_content.clear();
            }

            // Start new section
            let header = line.trim_start_matches('#').trim().to_lowercase();
            current_section = Some(header);
        } else if current_section.is_some() {
            section_content.push_str(line);
            section_content.push('\n');
        }
    }

    // Save last section
    if let Some(section_name) = current_section {
        sections.insert(section_name, section_content.trim().to_string());
    }

    sections
}

/// Extract description from the first paragraph
fn extract_description(content: &str) -> String {
    let mut description = String::new();
    let mut in_paragraph = false;

    for line in content.lines() {
        if line.starts_with('#') {
            if in_paragraph {
                break;
            }
            continue;
        }

        if line.trim().is_empty() {
            if in_paragraph {
                break;
            }
        } else {
            in_paragraph = true;
            if !description.is_empty() {
                description.push(' ');
            }
            description.push_str(line.trim());
        }
    }

    description
}

/// Extract arguments from usage patterns
fn extract_arguments_from_usage(usage: &str) -> Vec<CommandArgument> {
    let mut arguments = Vec::new();

    // Look for patterns like:
    // - <required_arg>
    // - [optional_arg]
    // - --flag
    // - --option=value

    for line in usage.lines() {
        if let Some(arg) = extract_argument_from_line(line) {
            arguments.push(arg);
        }
    }

    arguments
}

/// Extract a single argument from a line
fn extract_argument_from_line(line: &str) -> Option<CommandArgument> {
    let line = line.trim();

    // Check for <required> pattern
    if line.contains('<') && line.contains('>') {
        if let Some(start) = line.find('<') {
            if let Some(end) = line.find('>') {
                let name = line[start + 1..end].trim().to_string();
                let description = line[end + 1..].trim().trim_start_matches(':').trim().to_string();
                return Some(CommandArgument::required(name, description));
            }
        }
    }

    // Check for [optional] pattern
    if line.contains('[') && line.contains(']') {
        if let Some(start) = line.find('[') {
            if let Some(end) = line.find(']') {
                let name = line[start + 1..end].trim().to_string();
                let description = line[end + 1..].trim().trim_start_matches(':').trim().to_string();
                return Some(CommandArgument::optional(name, description, None));
            }
        }
    }

    // Check for --flag pattern
    if line.starts_with("--") || line.contains(" --") {
        if let Some(flag_start) = line.find("--") {
            let flag_part = &line[flag_start + 2..];
            if let Some(space_idx) = flag_part.find(' ') {
                let name = flag_part[..space_idx].trim_end_matches('=').to_string();
                let description = flag_part[space_idx..].trim().to_string();
                return Some(CommandArgument::optional(name, description, None));
            }
        }
    }

    None
}

/// Parse arguments from dedicated arguments section
fn parse_arguments_section(content: &str) -> Vec<CommandArgument> {
    let mut arguments = Vec::new();

    // Look for list items or structured definitions
    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        // Parse list items (- arg_name: description)
        if line.starts_with('-') || line.starts_with('*') {
            let content = line.trim_start_matches('-').trim_start_matches('*').trim();

            if let Some(colon_idx) = content.find(':') {
                let name = content[..colon_idx].trim().to_string();
                let description = content[colon_idx + 1..].trim().to_string();

                // Determine if required based on keywords
                let required = description.to_lowercase().contains("required")
                    || name.starts_with('<') && name.ends_with('>');

                let clean_name = name.trim_start_matches('<').trim_end_matches('>').to_string();

                if required {
                    arguments.push(CommandArgument::required(clean_name, description));
                } else {
                    arguments.push(CommandArgument::optional(clean_name, description, None));
                }
            }
        }
    }

    arguments
}

/// Extract examples from examples section
fn extract_examples(content: &str) -> Vec<String> {
    let mut examples = Vec::new();
    let mut current_example = String::new();
    let mut in_code_block = false;

    for line in content.lines() {
        if line.trim().starts_with("```") {
            if in_code_block {
                // End of code block
                if !current_example.is_empty() {
                    examples.push(current_example.trim().to_string());
                    current_example.clear();
                }
                in_code_block = false;
            } else {
                // Start of code block
                in_code_block = true;
            }
        } else if in_code_block {
            current_example.push_str(line);
            current_example.push('\n');
        }
    }

    // Add last example if exists
    if !current_example.is_empty() {
        examples.push(current_example.trim().to_string());
    }

    examples
}

impl Default for CommandParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_command_name_extraction() {
        let path = Path::new("/some/dir/my-command.md");
        let name = extract_command_name(path).unwrap();
        assert_eq!(name, "my-command");
    }

    #[test]
    fn test_command_name_validation() {
        assert!(validate_command_name("valid-name").is_ok());
        assert!(validate_command_name("valid_name").is_ok());
        assert!(validate_command_name("valid123").is_ok());
        assert!(validate_command_name("").is_err());
        assert!(validate_command_name("invalid name").is_err());
        assert!(validate_command_name("invalid!name").is_err());
    }

    #[test]
    fn test_parse_markdown_sections() {
        let content = r#"
# Title

Description paragraph.

## Usage

Command usage here.

## Arguments

- arg1: First argument
- arg2: Second argument

## Examples

Example content
"#;

        let sections = parse_markdown_sections(content);
        assert!(sections.contains_key("usage"));
        assert!(sections.contains_key("arguments"));
        assert!(sections.contains_key("examples"));
        assert_eq!(sections.get("usage").unwrap().trim(), "Command usage here.");
    }

    #[test]
    fn test_extract_description() {
        let content = r#"# Command Title

This is the command description.
It can span multiple lines.

## Usage

More content here.
"#;

        let desc = extract_description(content);
        assert_eq!(desc, "This is the command description. It can span multiple lines.");
    }

    #[test]
    fn test_parse_command_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test-command.md");

        let content = r#"# Test Command

This is a test command for demonstration.

## Usage

test-command <input_file> [output_file] --verbose

## Arguments

- input_file: The input file to process (required)
- output_file: The output file to write (optional)
- verbose: Enable verbose output

## Examples

```
test-command input.txt output.txt --verbose
```

## Instructions

Process the input file and generate output.
"#;

        fs::write(&file_path, content)?;

        let mut parser = CommandParser::new();
        parser.parse_file(&file_path)?;

        let registry = parser.registry();
        assert!(registry.contains("test-command"));

        let command = registry.get("test-command").unwrap();
        assert_eq!(command.name, "test-command");
        assert_eq!(command.description, "This is a test command for demonstration.");
        assert!(!command.usage.is_empty());
        assert!(!command.arguments.is_empty());
        assert_eq!(command.examples.len(), 1);

        Ok(())
    }

    #[test]
    fn test_parse_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;

        // Create multiple command files
        for i in 1..=3 {
            let file_path = temp_dir.path().join(format!("command{}.md", i));
            let content = format!("# Command {}\n\nDescription for command {}.", i, i);
            fs::write(&file_path, content)?;
        }

        let mut parser = CommandParser::new();
        parser.parse_directory(temp_dir.path())?;

        let registry = parser.registry();
        assert_eq!(registry.metadata.command_count, 3);
        assert!(registry.contains("command1"));
        assert!(registry.contains("command2"));
        assert!(registry.contains("command3"));

        Ok(())
    }

    #[test]
    fn test_argument_extraction() {
        let usage = r#"
        command <required_arg> [optional_arg] --flag --option=value
        "#;

        let args = extract_arguments_from_usage(usage);
        assert!(!args.is_empty());

        // Check for required argument
        assert!(args.iter().any(|a| a.name == "required_arg" && a.required));

        // Check for optional argument
        assert!(args.iter().any(|a| a.name == "optional_arg" && !a.required));
    }
}
