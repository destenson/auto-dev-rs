//! Instruction parsing from files and strings

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionDocument {
    pub raw_content: String,
    pub metadata: ProjectMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectMetadata {
    pub project_name: Option<String>,
    pub description: Option<String>,
    pub language: Option<String>,
    pub framework: Option<String>,
    pub dependencies: Vec<String>,
    pub features: Vec<String>,
}

pub struct InstructionParser;

impl InstructionParser {
    pub async fn from_file(path: &Path) -> Result<InstructionDocument> {
        let content =
            tokio::fs::read_to_string(path).await.context("Failed to read instruction file")?;

        Self::from_string(&content)
    }

    pub fn from_string(content: &str) -> Result<InstructionDocument> {
        let metadata = Self::extract_metadata(content);

        Ok(InstructionDocument { raw_content: content.to_string(), metadata })
    }

    fn extract_metadata(content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata::default();
        let content_lower = content.to_lowercase();

        // Extract project name
        metadata.project_name = Self::extract_project_name(&content_lower);

        // Extract description (first line or paragraph)
        metadata.description = content
            .lines()
            .find(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string());

        // Extract language hints
        metadata.language = Self::detect_language(&content_lower);

        // Extract framework hints
        metadata.framework = Self::detect_framework(&content_lower);

        // Extract potential dependencies
        metadata.dependencies = Self::extract_dependencies(&content_lower);

        // Extract feature keywords
        metadata.features = Self::extract_features(&content_lower);

        metadata
    }

    fn extract_project_name(content: &str) -> Option<String> {
        regex_utils::project_name::extract(content)
    }

    fn detect_language(content: &str) -> Option<String> {
        let matcher = regex_utils::language::LanguageMatcher::new();
        matcher.detect(content)
    }

    fn detect_framework(content: &str) -> Option<String> {
        regex_utils::framework::detect(content)
    }

    fn extract_dependencies(content: &str) -> Vec<String> {
        let mut deps = Vec::new();

        // Common dependency keywords
        let dep_keywords = [
            "database",
            "postgres",
            "mysql",
            "sqlite",
            "mongodb",
            "redis",
            "authentication",
            "jwt",
            "oauth",
            "rest",
            "graphql",
            "websocket",
            "logging",
            "testing",
            "docker",
        ];

        for keyword in &dep_keywords {
            if content.contains(keyword) {
                deps.push(keyword.to_string());
            }
        }

        deps
    }

    fn extract_features(content: &str) -> Vec<String> {
        let mut features = Vec::new();

        // Look for feature keywords
        let feature_keywords = [
            "crud",
            "api",
            "cli",
            "web",
            "server",
            "client",
            "real-time",
            "async",
            "concurrent",
            "parallel",
            "microservice",
            "monolith",
            "serverless",
        ];

        for keyword in &feature_keywords {
            if content.contains(keyword) {
                features.push(keyword.to_string());
            }
        }

        features
    }
}
