//! Specification and documentation parsing engine
//!
//! This module provides parsing capabilities for extracting actionable requirements,
//! API definitions, behavioral specifications, and implementation directives from
//! documentation and specification files.

pub mod model;
pub mod markdown;
pub mod schema;
pub mod openapi;
pub mod gherkin;
pub mod extractor;

use std::path::Path;
use anyhow::Result;

pub use model::*;
use crate::parser::markdown::MarkdownParser;
use crate::parser::schema::SchemaParser;
use crate::parser::openapi::OpenApiParser;
use crate::parser::gherkin::GherkinParser;
use crate::parser::extractor::RequirementExtractor;

/// Main parser that orchestrates specification extraction
pub struct SpecParser {
    markdown_parser: MarkdownParser,
    schema_parser: SchemaParser,
    openapi_parser: OpenApiParser,
    gherkin_parser: GherkinParser,
    extractor: RequirementExtractor,
}

impl SpecParser {
    /// Create a new specification parser
    pub fn new() -> Self {
        Self {
            markdown_parser: MarkdownParser::new(),
            schema_parser: SchemaParser::new(),
            openapi_parser: OpenApiParser::new(),
            gherkin_parser: GherkinParser::new(),
            extractor: RequirementExtractor::new(),
        }
    }

    /// Parse a specification file based on its extension
    pub async fn parse_file(&self, path: &Path) -> Result<Specification> {
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        match extension {
            "md" | "markdown" => self.parse_markdown(path).await,
            "yaml" | "yml" => self.parse_yaml(path).await,
            "json" => self.parse_json(path).await,
            "feature" => self.parse_gherkin(path).await,
            _ => self.parse_markdown(path).await, // Default to markdown
        }
    }

    /// Parse a markdown specification file
    async fn parse_markdown(&self, path: &Path) -> Result<Specification> {
        let content = tokio::fs::read_to_string(path).await?;
        let mut spec = self.markdown_parser.parse(&content)?;
        spec.source = path.to_path_buf();
        
        // Extract requirements from natural language
        let extracted_reqs = self.extractor.extract_from_text(&content)?;
        spec.requirements.extend(extracted_reqs);
        
        Ok(spec)
    }

    /// Parse a YAML specification file
    async fn parse_yaml(&self, path: &Path) -> Result<Specification> {
        let content = tokio::fs::read_to_string(path).await?;
        
        // Check if it's an OpenAPI spec
        if content.contains("openapi:") || content.contains("swagger:") {
            let mut spec = self.openapi_parser.parse_yaml(&content)?;
            spec.source = path.to_path_buf();
            Ok(spec)
        } else {
            let mut spec = self.schema_parser.parse_yaml(&content)?;
            spec.source = path.to_path_buf();
            Ok(spec)
        }
    }

    /// Parse a JSON specification file
    async fn parse_json(&self, path: &Path) -> Result<Specification> {
        let content = tokio::fs::read_to_string(path).await?;
        
        // Check if it's an OpenAPI spec
        if content.contains("\"openapi\"") || content.contains("\"swagger\"") {
            let mut spec = self.openapi_parser.parse_json(&content)?;
            spec.source = path.to_path_buf();
            Ok(spec)
        } else {
            let mut spec = self.schema_parser.parse_json(&content)?;
            spec.source = path.to_path_buf();
            Ok(spec)
        }
    }

    /// Parse a Gherkin feature file
    async fn parse_gherkin(&self, path: &Path) -> Result<Specification> {
        let content = tokio::fs::read_to_string(path).await?;
        let mut spec = self.gherkin_parser.parse(&content)?;
        spec.source = path.to_path_buf();
        Ok(spec)
    }

    /// Parse multiple files and merge specifications
    pub async fn parse_directory(&self, dir: &Path) -> Result<Vec<Specification>> {
        let mut specs = Vec::new();
        
        let mut entries = tokio::fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                if let Ok(spec) = self.parse_file(&path).await {
                    specs.push(spec);
                }
            }
        }
        
        Ok(specs)
    }
}

impl Default for SpecParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_parser_creation() {
        let parser = SpecParser::new();
        // Ensure parser can be created
        let _ = format!("{:?}", parser.markdown_parser);
    }
}