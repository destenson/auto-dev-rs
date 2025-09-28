//! Main instruction parser logic
//!
//! Coordinates format detection and parsing of instruction files or strings.

use crate::instructions::formats::{Format, detect_format, parse_json, parse_yaml, parse_markdown, parse_text};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::{debug, info};

/// Parsed instruction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedInstruction {
    /// Original raw content
    pub raw_content: String,
    /// Detected format
    pub format: Format,
    /// Parsed sections (if structured format)
    pub sections: Vec<InstructionSection>,
    /// Extracted key-value pairs
    pub metadata: std::collections::HashMap<String, String>,
    /// Full instruction text (cleaned)
    pub instruction_text: String,
}

/// A section within the instruction document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionSection {
    /// Section title/heading
    pub title: String,
    /// Section content
    pub content: String,
    /// Nesting level (for hierarchical formats)
    pub level: usize,
}

/// Main instruction parser
pub struct InstructionParser;

impl InstructionParser {
    /// Parse instructions from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<ParsedInstruction> {
        let path = path.as_ref();
        info!("Loading instruction file: {:?}", path);
        
        let raw_content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read instruction file: {:?}", path))?;
        
        let format = detect_format(path, &raw_content)?;
        debug!("Detected format: {:?}", format);
        
        Self::parse_content(raw_content, format)
    }
    
    /// Parse instructions from a string
    pub fn from_string(content: &str) -> Result<ParsedInstruction> {
        let format = detect_format(Path::new("inline.txt"), content)?;
        debug!("Detected format for inline content: {:?}", format);
        
        Self::parse_content(content.to_string(), format)
    }
    
    /// Parse content based on detected format
    fn parse_content(raw_content: String, format: Format) -> Result<ParsedInstruction> {
        let (sections, metadata, instruction_text) = match format {
            Format::Json => parse_json(&raw_content)?,
            Format::Yaml => parse_yaml(&raw_content)?,
            Format::Markdown => parse_markdown(&raw_content)?,
            Format::Text => parse_text(&raw_content)?,
        };
        
        Ok(ParsedInstruction {
            raw_content,
            format,
            sections,
            metadata,
            instruction_text,
        })
    }
    
    /// Validate parsed instructions
    pub fn validate(instruction: &ParsedInstruction) -> Result<()> {
        if instruction.instruction_text.is_empty() && instruction.sections.is_empty() {
            anyhow::bail!("No instruction content found");
        }
        
        if instruction.instruction_text.len() < 10 {
            anyhow::bail!("Instruction text too short (less than 10 characters)");
        }
        
        Ok(())
    }
}

impl ParsedInstruction {
    /// Get a metadata value by key
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
    
    /// Find sections by title (case-insensitive)
    pub fn find_sections(&self, title: &str) -> Vec<&InstructionSection> {
        let title_lower = title.to_lowercase();
        self.sections
            .iter()
            .filter(|s| s.title.to_lowercase().contains(&title_lower))
            .collect()
    }
    
    /// Get the main instruction content
    pub fn get_main_content(&self) -> &str {
        if !self.instruction_text.is_empty() {
            &self.instruction_text
        } else {
            &self.raw_content
        }
    }
    
    /// Check if instructions contain certain keywords
    pub fn contains_keywords(&self, keywords: &[&str]) -> bool {
        let content = self.get_main_content().to_lowercase();
        keywords.iter().any(|k| content.contains(&k.to_lowercase()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_from_string() {
        let content = "Create a web application with user authentication";
        let result = InstructionParser::from_string(content);
        assert!(result.is_ok());
        
        let parsed = result.unwrap();
        assert_eq!(parsed.format, Format::Text);
        assert!(parsed.instruction_text.contains("web application"));
    }
    
    #[test]
    fn test_validate_empty() {
        let instruction = ParsedInstruction {
            raw_content: String::new(),
            format: Format::Text,
            sections: vec![],
            metadata: Default::default(),
            instruction_text: String::new(),
        };
        
        assert!(InstructionParser::validate(&instruction).is_err());
    }
    
    #[test]
    fn test_find_sections() {
        let instruction = ParsedInstruction {
            raw_content: String::new(),
            format: Format::Markdown,
            sections: vec![
                InstructionSection {
                    title: "Requirements".to_string(),
                    content: "Some requirements".to_string(),
                    level: 1,
                },
                InstructionSection {
                    title: "Dependencies".to_string(),
                    content: "Some deps".to_string(),
                    level: 1,
                },
            ],
            metadata: Default::default(),
            instruction_text: String::new(),
        };
        
        let found = instruction.find_sections("req");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Requirements");
    }
    
    #[test]
    fn test_contains_keywords() {
        let instruction = ParsedInstruction {
            raw_content: String::new(),
            format: Format::Text,
            sections: vec![],
            metadata: Default::default(),
            instruction_text: "Build a Rust web server with async support".to_string(),
        };
        
        assert!(instruction.contains_keywords(&["rust", "web"]));
        assert!(instruction.contains_keywords(&["async"]));
        assert!(!instruction.contains_keywords(&["python", "django"]));
    }
}