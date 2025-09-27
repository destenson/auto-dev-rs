//! Self-Documentation System for auto-dev-rs
//! 
//! This module provides automatic documentation generation capabilities,
//! ensuring all self-modifications are properly documented.

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

pub mod extractor;
pub mod generator;
pub mod formatter;
pub mod changelog;
pub mod examples;

pub use extractor::DocExtractor;
pub use generator::DocGenerator;
pub use formatter::DocFormatter;
pub use changelog::ChangelogBuilder;
pub use examples::ExampleGenerator;

/// Main documentation system interface
pub struct DocumentationSystem {
    project_root: PathBuf,
    output_dir: PathBuf,
    extractor: DocExtractor,
    generator: DocGenerator,
    changelog: ChangelogBuilder,
}

/// Documentation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocConfig {
    /// Output directory for documentation
    pub output_dir: PathBuf,
    /// Formats to generate
    pub formats: Vec<DocFormat>,
    /// Include private items
    pub include_private: bool,
    /// Generate examples
    pub generate_examples: bool,
    /// Auto-update README
    pub update_readme: bool,
    /// Changelog settings
    pub changelog: ChangelogConfig,
}

/// Documentation output format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocFormat {
    Markdown,
    Html,
    Json,
}

/// Changelog configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogConfig {
    /// Path to changelog file
    pub file_path: PathBuf,
    /// Include unreleased changes
    pub include_unreleased: bool,
    /// Group by categories
    pub categorize: bool,
}

/// Documentation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocMetadata {
    /// Module or component name
    pub name: String,
    /// Brief description
    pub description: String,
    /// Version
    pub version: String,
    /// Authors
    pub authors: Vec<String>,
    /// Last modified
    pub last_modified: DateTime<Local>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Documentation entry for a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocEntry {
    /// Component metadata
    pub metadata: DocMetadata,
    /// Documentation sections
    pub sections: Vec<DocSection>,
    /// API documentation
    pub api_docs: Vec<ApiDoc>,
    /// Usage examples
    pub examples: Vec<Example>,
    /// Related documentation
    pub see_also: Vec<String>,
}

/// Documentation section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocSection {
    /// Section title
    pub title: String,
    /// Section content
    pub content: String,
    /// Subsections
    pub subsections: Vec<DocSection>,
}

/// API documentation entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDoc {
    /// Item name (function, struct, etc.)
    pub name: String,
    /// Item kind
    pub kind: ApiItemKind,
    /// Signature
    pub signature: String,
    /// Description
    pub description: String,
    /// Parameters
    pub parameters: Vec<Parameter>,
    /// Return value
    pub returns: Option<String>,
    /// Examples
    pub examples: Vec<String>,
    /// Since version
    pub since: Option<String>,
}

/// API item kind
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiItemKind {
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    Module,
    Macro,
    Constant,
}

/// Function parameter documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: String,
    /// Description
    pub description: String,
    /// Optional/required
    pub required: bool,
}

/// Code example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    /// Example title
    pub title: String,
    /// Example description
    pub description: String,
    /// Code snippet
    pub code: String,
    /// Expected output
    pub output: Option<String>,
    /// Language
    pub language: String,
}

/// Documentation generation result
#[derive(Debug)]
pub struct GenerationResult {
    /// Generated files
    pub files: Vec<PathBuf>,
    /// Statistics
    pub stats: DocStats,
    /// Warnings
    pub warnings: Vec<String>,
}

/// Documentation statistics
#[derive(Debug)]
pub struct DocStats {
    /// Total modules documented
    pub modules: usize,
    /// Total functions documented
    pub functions: usize,
    /// Total examples generated
    pub examples: usize,
    /// Coverage percentage
    pub coverage: f64,
}

impl DocumentationSystem {
    /// Create new documentation system
    pub fn new(project_root: PathBuf, config: DocConfig) -> Result<Self> {
        let extractor = DocExtractor::new(project_root.clone());
        let generator = DocGenerator::new(config.clone());
        let changelog = ChangelogBuilder::new(config.changelog);
        
        Ok(Self {
            project_root,
            output_dir: config.output_dir,
            extractor,
            generator,
            changelog,
        })
    }

    /// Generate documentation for entire project
    pub async fn generate_all(&self) -> Result<GenerationResult> {
        // Extract documentation from code
        let entries = self.extractor.extract_all(&self.project_root)?;
        
        // Generate documentation files
        let files = self.generator.generate_docs(&entries, &self.output_dir)?;
        
        // Update changelog
        self.changelog.update()?;
        
        // Calculate statistics
        let stats = self.calculate_stats(&entries);
        
        Ok(GenerationResult {
            files,
            stats,
            warnings: vec![],
        })
    }

    /// Generate documentation for specific module
    pub async fn generate_module(&self, module_path: &Path) -> Result<DocEntry> {
        self.extractor.extract_module(module_path)
    }

    /// Update README with latest information
    pub async fn update_readme(&self) -> Result<()> {
        let readme_path = self.project_root.join("README.md");
        self.generator.update_readme(&readme_path)
    }

    /// Generate API documentation
    pub async fn generate_api_docs(&self) -> Result<Vec<PathBuf>> {
        let api_docs = self.extractor.extract_api(&self.project_root)?;
        self.generator.generate_api_docs(&api_docs, &self.output_dir)
    }

    /// Validate existing documentation
    pub async fn validate(&self) -> Result<Vec<String>> {
        let issues = self.extractor.validate_docs(&self.project_root)?;
        Ok(issues)
    }

    /// Check documentation coverage
    pub async fn check_coverage(&self) -> Result<f64> {
        let coverage = self.extractor.calculate_coverage(&self.project_root)?;
        Ok(coverage)
    }

    fn calculate_stats(&self, entries: &[DocEntry]) -> DocStats {
        let modules = entries.len();
        let functions = entries.iter()
            .flat_map(|e| &e.api_docs)
            .filter(|api| matches!(api.kind, ApiItemKind::Function | ApiItemKind::Method))
            .count();
        let examples = entries.iter()
            .flat_map(|e| &e.examples)
            .count();
        
        DocStats {
            modules,
            functions,
            examples,
            coverage: 0.0, // Calculate actual coverage
        }
    }
}

impl Default for DocConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("docs"),
            formats: vec![DocFormat::Markdown],
            include_private: false,
            generate_examples: true,
            update_readme: true,
            changelog: ChangelogConfig::default(),
        }
    }
}

impl Default for ChangelogConfig {
    fn default() -> Self {
        Self {
            file_path: PathBuf::from("CHANGELOG.md"),
            include_unreleased: true,
            categorize: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_config_default() {
        let config = DocConfig::default();
        assert_eq!(config.output_dir, PathBuf::from("docs"));
        assert!(!config.include_private);
        assert!(config.generate_examples);
    }

    #[test]
    fn test_api_item_kind() {
        let kind = ApiItemKind::Function;
        match kind {
            ApiItemKind::Function => assert!(true),
            _ => assert!(false),
        }
    }
}