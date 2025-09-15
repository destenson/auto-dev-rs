#![allow(unused)]
//! Markdown specification parser

use anyhow::Result;
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use std::path::PathBuf;

use crate::parser::model::*;

/// Parser for extracting specifications from Markdown documents
#[derive(Debug)]
pub struct MarkdownParser {
    current_section: String,
    current_line: usize,
}

impl MarkdownParser {
    /// Create a new Markdown parser
    pub fn new() -> Self {
        Self { current_section: String::new(), current_line: 0 }
    }

    /// Parse a Markdown document and extract specifications
    pub fn parse(&self, content: &str) -> Result<Specification> {
        let mut spec = Specification::new(PathBuf::new());
        let parser = Parser::new(content);

        let mut current_section = String::new();
        let mut in_code_block = false;
        let mut code_block_lang = String::new();
        let mut code_block_content = String::new();
        let mut list_items: Vec<String> = Vec::new();
        let mut in_list = false;
        let mut current_heading_level = 0;

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    current_heading_level = level as usize;
                    in_list = false;
                }
                Event::Text(text) => {
                    let text_str = text.to_string();

                    // Update current section if we're in a heading
                    if current_heading_level > 0 {
                        current_section = text_str.clone();
                        current_heading_level = 0;

                        // Check for specific section types
                        if is_requirements_section(&current_section) {
                            // Mark as requirements section
                        } else if is_api_section(&current_section) {
                            // Mark as API section
                        }
                    }

                    // Collect list items
                    if in_list {
                        list_items.push(text_str.clone());
                    }

                    // Accumulate code block content
                    if in_code_block {
                        code_block_content.push_str(&text_str);
                    }
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    in_code_block = true;
                    if let CodeBlockKind::Fenced(lang) = kind {
                        code_block_lang = lang.to_string();
                    }
                    code_block_content.clear();
                }
                Event::End(TagEnd::CodeBlock) => {
                    in_code_block = false;

                    // Process code block based on language and context
                    if !code_block_content.is_empty() {
                        self.process_code_block(
                            &mut spec,
                            &current_section,
                            &code_block_lang,
                            &code_block_content,
                        )?;
                    }

                    code_block_lang.clear();
                    code_block_content.clear();
                }
                Event::Start(Tag::List(_)) => {
                    in_list = true;
                    list_items.clear();
                }
                Event::End(TagEnd::List(_)) => {
                    in_list = false;

                    // Process list items based on current section
                    if !list_items.is_empty() {
                        self.process_list_items(&mut spec, &current_section, &list_items)?;
                    }

                    list_items.clear();
                }
                _ => {}
            }
        }

        Ok(spec)
    }

    /// Process a code block and extract relevant information
    fn process_code_block(
        &self,
        spec: &mut Specification,
        section: &str,
        language: &str,
        content: &str,
    ) -> Result<()> {
        // Check if it's an API definition
        if is_api_section(section) || language == "yaml" || language == "json" {
            if let Some(api) = self.extract_api_from_code(content, language)? {
                spec.apis.push(api);
            }
        }

        // Add as an example
        if !content.trim().is_empty() {
            spec.examples.push(Example {
                title: section.to_string(),
                language: language.to_string(),
                code: content.to_string(),
                description: format!("Code example from {}", section),
                expected_output: None,
            });
        }

        Ok(())
    }

    /// Process list items and extract requirements or acceptance criteria
    fn process_list_items(
        &self,
        spec: &mut Specification,
        section: &str,
        items: &[String],
    ) -> Result<()> {
        if is_requirements_section(section) {
            // Extract requirements from list items
            for (idx, item) in items.iter().enumerate() {
                if let Some(req) = self.extract_requirement_from_text(item, idx) {
                    spec.requirements.push(req);
                }
            }
        } else if is_acceptance_criteria_section(section) {
            // Store as acceptance criteria for the last requirement
            if let Some(last_req) = spec.requirements.last_mut() {
                last_req
                    .acceptance_criteria
                    .extend(items.iter().map(|s| s.trim_start_matches("- [ ]").trim().to_string()));
            }
        }

        Ok(())
    }

    /// Extract a requirement from text
    fn extract_requirement_from_text(&self, text: &str, index: usize) -> Option<Requirement> {
        let text = text.trim();

        // Check for requirement keywords
        let priority = if text.contains("MUST") || text.contains("SHALL") {
            Priority::Critical
        } else if text.contains("SHOULD") {
            Priority::High
        } else if text.contains("COULD") || text.contains("MAY") {
            Priority::Medium
        } else {
            return None;
        };

        let mut requirement = Requirement::new(format!("REQ-{:03}", index + 1), text.to_string());
        requirement.priority = priority;
        requirement.category = RequirementType::Functional;

        Some(requirement)
    }

    /// Extract API definition from code block
    fn extract_api_from_code(
        &self,
        content: &str,
        language: &str,
    ) -> Result<Option<ApiDefinition>> {
        // Simple pattern matching for API definitions
        if language == "yaml" || content.contains("POST") || content.contains("GET") {
            // Extract method and endpoint
            let lines: Vec<&str> = content.lines().collect();
            if lines.is_empty() {
                return Ok(None);
            }

            let first_line = lines[0];
            let parts: Vec<&str> = first_line.split_whitespace().collect();

            if parts.len() >= 2 {
                let method = HttpMethod::from(parts[0]);
                let endpoint = parts[1].to_string();

                let api = ApiDefinition {
                    endpoint,
                    method,
                    request_schema: None,
                    response_schema: None,
                    query_params: Vec::new(),
                    path_params: Vec::new(),
                    headers: Vec::new(),
                    description: String::new(),
                    examples: Vec::new(),
                };

                return Ok(Some(api));
            }
        }

        Ok(None)
    }
}

impl Default for MarkdownParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a section name indicates requirements
fn is_requirements_section(section: &str) -> bool {
    let lower = section.to_lowercase();
    lower.contains("requirement")
        || lower.contains("must have")
        || lower.contains("should have")
        || lower.contains("features")
}

/// Check if a section name indicates API specifications
fn is_api_section(section: &str) -> bool {
    let lower = section.to_lowercase();
    lower.contains("api")
        || lower.contains("endpoint")
        || lower.contains("route")
        || lower.contains("interface")
}

/// Check if a section name indicates acceptance criteria
fn is_acceptance_criteria_section(section: &str) -> bool {
    let lower = section.to_lowercase();
    lower.contains("acceptance")
        || lower.contains("criteria")
        || lower.contains("test")
        || lower.contains("scenario")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_parser_creation() {
        let parser = MarkdownParser::new();
        assert_eq!(parser.current_line, 0);
    }

    #[test]
    fn test_parse_simple_markdown() {
        let parser = MarkdownParser::new();
        let content = r#"
# Feature: User Authentication

## Requirements
- MUST support email/password login
- SHOULD support OAuth2
- MAY support biometric authentication

## API Specification
```yaml
POST /api/auth/login
  body:
    email: string
    password: string
```
"#;

        let spec = parser.parse(content).unwrap();
        assert!(!spec.requirements.is_empty());
        assert!(!spec.examples.is_empty());
    }

    #[test]
    fn test_requirement_extraction() {
        let parser = MarkdownParser::new();

        let req1 = parser.extract_requirement_from_text("MUST support user login", 0);
        assert!(req1.is_some());
        assert_eq!(req1.unwrap().priority, Priority::Critical);

        let req2 = parser.extract_requirement_from_text("SHOULD validate email format", 1);
        assert!(req2.is_some());
        assert_eq!(req2.unwrap().priority, Priority::High);

        let req3 = parser.extract_requirement_from_text("Regular text without keywords", 2);
        assert!(req3.is_none());
    }

    #[test]
    fn test_section_detection() {
        assert!(is_requirements_section("Requirements"));
        assert!(is_requirements_section("Functional Requirements"));
        assert!(is_requirements_section("Must Have Features"));
        assert!(!is_requirements_section("Introduction"));

        assert!(is_api_section("API Specification"));
        assert!(is_api_section("REST Endpoints"));
        assert!(!is_api_section("Overview"));

        assert!(is_acceptance_criteria_section("Acceptance Criteria"));
        assert!(is_acceptance_criteria_section("Test Scenarios"));
        assert!(!is_acceptance_criteria_section("Description"));
    }
}
