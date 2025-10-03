//! Format-specific parsers for different instruction file types
//!
//! Handles JSON, YAML, Markdown, and plain text formats.

use crate::instructions::parser::InstructionSection;
use anyhow::{Context, Result};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::collections::HashMap;
use std::path::Path;

/// Supported instruction formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Format {
    Json,
    Yaml,
    Markdown,
    Text,
}

/// Detect format from file extension and content
pub fn detect_format(path: &Path, content: &str) -> Result<Format> {
    // First try by extension
    if let Some(ext) = path.extension() {
        let ext = ext.to_str().unwrap_or("").to_lowercase();
        match ext.as_str() {
            "json" => return Ok(Format::Json),
            "yaml" | "yml" => return Ok(Format::Yaml),
            "md" | "markdown" => return Ok(Format::Markdown),
            _ => {}
        }
    }

    // Try to detect from content
    let trimmed = content.trim();

    // Check for JSON
    if (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        if serde_json::from_str::<JsonValue>(trimmed).is_ok() {
            return Ok(Format::Json);
        }
    }

    // Check for YAML (simple heuristic)
    if trimmed.contains(":\n") || trimmed.contains(": ") {
        if serde_yaml::from_str::<YamlValue>(trimmed).is_ok() {
            return Ok(Format::Yaml);
        }
    }

    // Check for Markdown
    if trimmed.contains("# ")
        || trimmed.contains("## ")
        || trimmed.contains("```")
        || trimmed.contains("- ")
    {
        return Ok(Format::Markdown);
    }

    // Default to text
    Ok(Format::Text)
}

/// Parse JSON format
pub fn parse_json(
    content: &str,
) -> Result<(Vec<InstructionSection>, HashMap<String, String>, String)> {
    let value: JsonValue = serde_json::from_str(content).context("Failed to parse JSON")?;

    let mut sections = Vec::new();
    let mut metadata = HashMap::new();
    let mut instruction_text = String::new();

    match &value {
        JsonValue::Object(map) => {
            // Extract known fields
            if let Some(JsonValue::String(s)) = map.get("instruction") {
                instruction_text = s.clone();
            }
            if let Some(JsonValue::String(s)) = map.get("instructions") {
                instruction_text = s.clone();
            }
            if let Some(JsonValue::String(s)) = map.get("description") {
                if instruction_text.is_empty() {
                    instruction_text = s.clone();
                }
            }

            // Extract metadata
            for (key, val) in map {
                if let JsonValue::String(s) = val {
                    metadata.insert(key.clone(), s.clone());
                } else if !matches!(key.as_str(), "instruction" | "instructions" | "sections") {
                    metadata.insert(key.clone(), val.to_string());
                }
            }

            // Extract sections if present
            if let Some(JsonValue::Array(arr)) = map.get("sections") {
                for (i, item) in arr.iter().enumerate() {
                    if let JsonValue::Object(section_map) = item {
                        let title = section_map
                            .get("title")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&format!("Section {}", i + 1))
                            .to_string();
                        let content = section_map
                            .get("content")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        sections.push(InstructionSection { title, content, level: 1 });
                    }
                }
            }
        }
        _ => {
            instruction_text = serde_json::to_string_pretty(&value)?;
        }
    }

    // If no instruction text but we have metadata, combine it
    if instruction_text.is_empty() && !metadata.is_empty() {
        instruction_text = metadata
            .iter()
            .filter(|(k, _)| !k.starts_with('_'))
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\n");
    }

    Ok((sections, metadata, instruction_text))
}

/// Parse YAML format
pub fn parse_yaml(
    content: &str,
) -> Result<(Vec<InstructionSection>, HashMap<String, String>, String)> {
    let value: YamlValue = serde_yaml::from_str(content).context("Failed to parse YAML")?;

    let mut sections = Vec::new();
    let mut metadata = HashMap::new();
    let mut instruction_text = String::new();

    if let YamlValue::Mapping(map) = &value {
        // Extract known fields
        for (key, val) in map {
            if let Some(key_str) = key.as_str() {
                match key_str {
                    "instruction" | "instructions" => {
                        if let Some(s) = val.as_str() {
                            instruction_text = s.to_string();
                        }
                    }
                    "description" if instruction_text.is_empty() => {
                        if let Some(s) = val.as_str() {
                            instruction_text = s.to_string();
                        }
                    }
                    "sections" => {
                        if let YamlValue::Sequence(seq) = val {
                            for (i, item) in seq.iter().enumerate() {
                                if let YamlValue::Mapping(section_map) = item {
                                    let title = section_map
                                        .get(&YamlValue::String("title".to_string()))
                                        .and_then(|v| v.as_str())
                                        .unwrap_or(&format!("Section {}", i + 1))
                                        .to_string();
                                    let content = section_map
                                        .get(&YamlValue::String("content".to_string()))
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();

                                    sections.push(InstructionSection { title, content, level: 1 });
                                }
                            }
                        }
                    }
                    _ => {
                        if let Some(s) = val.as_str() {
                            metadata.insert(key_str.to_string(), s.to_string());
                        } else {
                            metadata.insert(key_str.to_string(), format!("{:?}", val));
                        }
                    }
                }
            }
        }
    } else {
        instruction_text = serde_yaml::to_string(&value)?;
    }

    // If no instruction text but we have metadata, combine it
    if instruction_text.is_empty() && !metadata.is_empty() {
        instruction_text = metadata
            .iter()
            .filter(|(k, _)| !k.starts_with('_'))
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\n");
    }

    Ok((sections, metadata, instruction_text))
}

/// Parse Markdown format
pub fn parse_markdown(
    content: &str,
) -> Result<(Vec<InstructionSection>, HashMap<String, String>, String)> {
    let mut sections = Vec::new();
    let mut metadata = HashMap::new();
    let mut instruction_text = String::new();

    let parser = Parser::new(content);
    let mut current_section: Option<(String, usize, Vec<String>)> = None;
    let mut in_code_block = false;
    let mut in_heading = false;
    let mut full_text = Vec::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                // Save previous section if exists
                if let Some((title, level, content)) = current_section.take() {
                    sections.push(InstructionSection {
                        title,
                        content: content.join("\n").trim().to_string(),
                        level: level as usize,
                    });
                }
                in_heading = true;
                current_section =
                    Some((String::new(), heading_level_to_usize(level) as usize, Vec::new()));
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
            }
            Event::Text(text) => {
                if in_heading {
                    if let Some((ref mut title, _, _)) = current_section {
                        *title = text.to_string();
                    }
                } else if let Some((_, _, ref mut content)) = current_section {
                    content.push(text.to_string());
                }
                if !in_heading {
                    full_text.push(text.to_string());
                }
            }
            Event::Code(code) => {
                let code_str = format!("`{}`", code);
                if let Some((_, _, ref mut content)) = current_section {
                    content.push(code_str.clone());
                }
                full_text.push(code_str);
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                if let Some((_, _, ref mut content)) = current_section {
                    content.push("```".to_string());
                }
                full_text.push("```".to_string());
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                if let Some((_, _, ref mut content)) = current_section {
                    content.push("```".to_string());
                }
                full_text.push("```".to_string());
            }
            _ => {}
        }
    }

    // Save last section if exists
    if let Some((title, level, content)) = current_section {
        sections.push(InstructionSection {
            title,
            content: content.join("\n").trim().to_string(),
            level: level as usize,
        });
    }

    instruction_text = full_text.join(" ").trim().to_string();

    // Try to extract metadata from sections
    for section in &sections {
        let lower_title = section.title.to_lowercase();
        if lower_title.contains("metadata") || lower_title.contains("frontmatter") {
            // Parse key: value pairs from content
            for line in section.content.lines() {
                if let Some(colon_pos) = line.find(':') {
                    let key = line[..colon_pos].trim().to_string();
                    let value = line[colon_pos + 1..].trim().to_string();
                    if !key.is_empty() && !value.is_empty() {
                        metadata.insert(key, value);
                    }
                }
            }
        }
    }

    Ok((sections, metadata, instruction_text))
}

fn heading_level_to_usize(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Parse plain text format
pub fn parse_text(
    content: &str,
) -> Result<(Vec<InstructionSection>, HashMap<String, String>, String)> {
    let mut sections = Vec::new();
    let mut metadata = HashMap::new();

    // Simple heuristics for text files
    let lines: Vec<&str> = content.lines().collect();
    let mut current_section: Option<(String, Vec<String>)> = None;

    for line in &lines {
        let trimmed = line.trim();

        // Check for section-like headers (all caps, followed by colon, etc.)
        if trimmed.len() > 3
            && trimmed.ends_with(':')
            && trimmed
                .chars()
                .take(trimmed.len() - 1)
                .all(|c| c.is_uppercase() || c.is_whitespace() || c == '_')
        {
            // Save previous section
            if let Some((title, content)) = current_section.take() {
                sections.push(InstructionSection {
                    title,
                    content: content.join("\n").trim().to_string(),
                    level: 1,
                });
            }
            current_section = Some((trimmed[..trimmed.len() - 1].to_string(), Vec::new()));
        } else if let Some((_, ref mut content)) = current_section {
            content.push(line.to_string());
        }

        // Look for metadata patterns (key: value)
        if trimmed.contains(": ") && !trimmed.contains("://") {
            if let Some(colon_pos) = trimmed.find(": ") {
                let key = trimmed[..colon_pos].trim();
                let value = trimmed[colon_pos + 2..].trim();

                // Simple heuristic: keys should be relatively short and not contain spaces
                if key.len() < 30 && !key.contains(' ') {
                    metadata.insert(key.to_string(), value.to_string());
                }
            }
        }
    }

    // Save last section
    if let Some((title, content)) = current_section {
        sections.push(InstructionSection {
            title,
            content: content.join("\n").trim().to_string(),
            level: 1,
        });
    }

    let instruction_text = content.trim().to_string();

    Ok((sections, metadata, instruction_text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_format() {
        assert_eq!(detect_format(Path::new("test.json"), "{}").unwrap(), Format::Json);
        assert_eq!(detect_format(Path::new("test.yaml"), "key: value").unwrap(), Format::Yaml);
        assert_eq!(detect_format(Path::new("test.md"), "# Title").unwrap(), Format::Markdown);
        assert_eq!(detect_format(Path::new("test.txt"), "plain text").unwrap(), Format::Text);
    }

    #[test]
    fn test_parse_json() {
        let json = r#"{
            "instruction": "Build a web app",
            "language": "rust",
            "framework": "actix"
        }"#;

        let (sections, metadata, text) = parse_json(json).unwrap();
        assert_eq!(text, "Build a web app");
        assert_eq!(metadata.get("language"), Some(&"rust".to_string()));
        assert_eq!(metadata.get("framework"), Some(&"actix".to_string()));
    }

    #[test]
    fn test_parse_yaml() {
        let yaml = r#"
instruction: Create a CLI tool
language: rust
dependencies:
  - clap
  - tokio
"#;

        let (_, metadata, text) = parse_yaml(yaml).unwrap();
        assert_eq!(text, "Create a CLI tool");
        assert_eq!(metadata.get("language"), Some(&"rust".to_string()));
    }

    #[test]
    fn test_parse_markdown() {
        let md = r#"# Project Instructions

Build a web server with the following features:

## Requirements
- Fast response times
- JSON API

## Dependencies
- actix-web
- serde
"#;

        let (sections, _, text) = parse_markdown(md).unwrap();
        assert!(!sections.is_empty());
        assert!(text.contains("Build a web server"));
        assert_eq!(sections[0].title, "Project Instructions");
    }
}
