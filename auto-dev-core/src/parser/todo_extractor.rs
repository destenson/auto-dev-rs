//! TODO comment and documentation specification extraction

use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::parser::model::*;

/// Configuration for TODO extraction
#[derive(Debug, Clone)]
pub struct TodoConfig {
    /// Enable TODO extraction
    pub include_todos: bool,
    /// TODO marker patterns to search for
    pub todo_patterns: Vec<String>,
    /// File patterns to search
    pub file_types: Vec<String>,
    /// Priority mapping from marker to Priority enum
    pub priority_mapping: HashMap<String, Priority>,
}

impl Default for TodoConfig {
    fn default() -> Self {
        let mut priority_mapping = HashMap::new();
        priority_mapping.insert("FIXME".to_string(), Priority::High);
        priority_mapping.insert("TODO".to_string(), Priority::Medium);
        priority_mapping.insert("HACK".to_string(), Priority::Low);
        priority_mapping.insert("XXX".to_string(), Priority::Medium);
        priority_mapping.insert("BUG".to_string(), Priority::High);
        priority_mapping.insert("NOTE".to_string(), Priority::Low);

        Self {
            include_todos: true,
            todo_patterns: vec![
                "TODO".to_string(),
                "FIXME".to_string(),
                "HACK".to_string(),
                "XXX".to_string(),
                "BUG".to_string(),
                "NOTE".to_string(),
            ],
            file_types: vec![
                "*.rs".to_string(),
                "*.md".to_string(),
                "*.toml".to_string(),
                "*.yml".to_string(),
                "*.yaml".to_string(),
                "*.js".to_string(),
                "*.ts".to_string(),
                "*.py".to_string(),
            ],
            priority_mapping,
        }
    }
}

/// Extracts TODO comments and converts them to specifications
pub struct TodoExtractor {
    config: TodoConfig,
    patterns: Vec<Regex>,
}

impl TodoExtractor {
    /// Create a new TODO extractor with default configuration
    pub fn new() -> Self {
        Self::with_config(TodoConfig::default())
    }

    /// Create a new TODO extractor with custom configuration
    pub fn with_config(config: TodoConfig) -> Self {
        let patterns = Self::compile_patterns(&config.todo_patterns);
        Self { config, patterns }
    }

    /// Extract TODOs from a single file
    pub async fn extract_from_file(&self, path: &Path) -> Result<Vec<Requirement>> {
        if !self.config.include_todos {
            return Ok(Vec::new());
        }

        // Check if file matches configured patterns
        if !self.should_process_file(path) {
            return Ok(Vec::new());
        }

        let content = tokio::fs::read_to_string(path).await?;
        self.extract_from_content(&content, path)
    }

    /// Extract TODOs from text content
    pub fn extract_from_content(
        &self,
        content: &str,
        source_path: &Path,
    ) -> Result<Vec<Requirement>> {
        let mut requirements = Vec::new();
        let mut req_counter = 1;

        for (line_num, line) in content.lines().enumerate() {
            if let Some(todo) = self.extract_todo_from_line(line, line_num + 1) {
                let (marker, description) = todo;

                // Generate unique ID based on file and line
                let id = format!(
                    "TODO-{}-{:04}",
                    source_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_uppercase(),
                    req_counter
                );
                req_counter += 1;

                // Map marker to priority
                let priority = self
                    .config
                    .priority_mapping
                    .get(&marker.to_uppercase())
                    .copied()
                    .unwrap_or(Priority::Medium);

                // Determine category based on content
                let category = self.infer_category(&description);

                // Create requirement from TODO
                let mut requirement = Requirement::new(id, description.clone());
                requirement.priority = priority;
                requirement.category = category;
                requirement.source_location =
                    SourceLocation::new(source_path.to_path_buf(), line_num + 1);
                requirement.tags = vec!["todo".to_string(), marker.to_lowercase()];

                // Add acceptance criteria if we can infer them
                if let Some(criteria) = self.infer_acceptance_criteria(&description) {
                    requirement.acceptance_criteria = criteria;
                }

                requirements.push(requirement);
            }
        }

        Ok(requirements)
    }

    /// Extract TODO from a single line
    fn extract_todo_from_line(&self, line: &str, _line_num: usize) -> Option<(String, String)> {
        for pattern in &self.patterns {
            if let Some(captures) = pattern.captures(line) {
                // Get the marker (TODO, FIXME, etc.)
                let marker = captures
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "TODO".to_string());

                // Get the description after the marker
                let description = captures
                    .get(2)
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_else(|| line.to_string());

                // Skip if description is empty or too short
                if description.len() < 3 {
                    continue;
                }

                // Clean up the description
                let description = self.clean_description(&description);

                return Some((marker, description));
            }
        }
        None
    }

    /// Clean up TODO description
    fn clean_description(&self, desc: &str) -> String {
        let mut cleaned = desc.trim();

        // Remove common prefixes
        if cleaned.starts_with(':') || cleaned.starts_with('-') {
            cleaned = &cleaned[1..];
        }

        // Remove trailing comment markers
        if cleaned.ends_with("*/") {
            cleaned = &cleaned[..cleaned.len() - 2];
        }

        // Handle multiline TODOs (for now, just take first line)
        if let Some(first_line) = cleaned.lines().next() {
            cleaned = first_line;
        }

        cleaned.trim().to_string()
    }

    /// Infer requirement category from description
    fn infer_category(&self, description: &str) -> RequirementType {
        let desc_lower = description.to_lowercase();

        if desc_lower.contains("implement")
            || desc_lower.contains("add")
            || desc_lower.contains("create")
        {
            RequirementType::Functional
        } else if desc_lower.contains("fix")
            || desc_lower.contains("bug")
            || desc_lower.contains("error")
        {
            RequirementType::Reliability
        } else if desc_lower.contains("test") || desc_lower.contains("validate") {
            RequirementType::Reliability
        } else if desc_lower.contains("api") || desc_lower.contains("endpoint") {
            RequirementType::Api
        } else if desc_lower.contains("security") || desc_lower.contains("auth") {
            RequirementType::Security
        } else if desc_lower.contains("performance") || desc_lower.contains("optimize") {
            RequirementType::Performance
        } else if desc_lower.contains("ui") || desc_lower.contains("interface") {
            RequirementType::Usability
        } else {
            RequirementType::Functional
        }
    }

    /// Infer acceptance criteria from TODO description
    fn infer_acceptance_criteria(&self, description: &str) -> Option<Vec<String>> {
        let mut criteria = Vec::new();
        let desc_lower = description.to_lowercase();

        // For implementation TODOs
        if desc_lower.contains("implement") {
            criteria.push(format!("Feature is implemented: {}", description));
            criteria.push("Unit tests are written and passing".to_string());
            criteria.push("Documentation is updated".to_string());
        }
        // For bug fixes
        else if desc_lower.contains("fix") || desc_lower.contains("bug") {
            criteria.push(format!("Bug is fixed: {}", description));
            criteria.push("Regression test is added".to_string());
            criteria.push("No new issues introduced".to_string());
        }
        // For test TODOs
        else if desc_lower.contains("test") {
            criteria.push(format!("Test is implemented: {}", description));
            criteria.push("Test passes consistently".to_string());
            criteria.push("Test coverage is improved".to_string());
        }

        if criteria.is_empty() { None } else { Some(criteria) }
    }

    /// Check if file should be processed based on patterns
    fn should_process_file(&self, path: &Path) -> bool {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Check against file type patterns
        for pattern in &self.config.file_types {
            if self.matches_pattern(file_name, pattern) {
                return true;
            }
        }

        false
    }

    /// Simple glob pattern matching
    fn matches_pattern(&self, file_name: &str, pattern: &str) -> bool {
        if pattern.starts_with('*') {
            let suffix = &pattern[1..];
            file_name.ends_with(suffix)
        } else if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len() - 1];
            file_name.starts_with(prefix)
        } else {
            file_name == pattern
        }
    }

    /// Compile regex patterns for TODO markers
    fn compile_patterns(markers: &[String]) -> Vec<Regex> {
        let mut patterns = Vec::new();

        // Create patterns for each marker
        for marker in markers {
            // Pattern for single-line comments (// TODO: ...)
            let pattern1 = format!(r"(?i)//\s*({})\s*:?\s*(.+)", regex::escape(marker));

            // Pattern for multi-line comments (/* TODO: ... */)
            let pattern2 = format!(r"(?i)/\*\s*({})\s*:?\s*(.+?)\*/", regex::escape(marker));

            // Pattern for hash comments (# TODO: ...)
            let pattern3 = format!(r"(?i)#\s*({})\s*:?\s*(.+)", regex::escape(marker));

            // Pattern with brackets ([TODO] ...)
            let pattern4 = format!(r"(?i)\[({})\]\s*:?\s*(.+)", regex::escape(marker));

            // Pattern with parentheses (TODO(username): ...)
            let pattern5 = format!(r"(?i)({})\([^)]*\)\s*:?\s*(.+)", regex::escape(marker));

            // Try to compile each pattern
            for pattern in [pattern1, pattern2, pattern3, pattern4, pattern5] {
                if let Ok(regex) = Regex::new(&pattern) {
                    patterns.push(regex);
                }
            }
        }

        patterns
    }

    /// Extract TODOs from a directory recursively
    pub async fn extract_from_directory(&self, dir: &Path) -> Result<Vec<Requirement>> {
        let mut all_requirements = Vec::new();

        let mut entries = tokio::fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_dir() {
                // Skip common directories that shouldn't be scanned
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if matches!(dir_name, "target" | "node_modules" | ".git" | "dist" | "build") {
                    continue;
                }

                // Recursively process subdirectory
                let sub_reqs = Box::pin(self.extract_from_directory(&path)).await?;
                all_requirements.extend(sub_reqs);
            } else if path.is_file() {
                // Process file
                if let Ok(reqs) = self.extract_from_file(&path).await {
                    all_requirements.extend(reqs);
                }
            }
        }

        Ok(all_requirements)
    }
}

impl Default for TodoExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_todo_extractor_creation() {
        let extractor = TodoExtractor::new();
        assert!(extractor.config.include_todos);
        assert!(!extractor.patterns.is_empty());
    }

    #[test]
    fn test_extract_todo_from_line() {
        let extractor = TodoExtractor::new();

        // Test various TODO formats
        let test_cases = vec![
            (
                "// TODO: Implement feature X",
                Some(("TODO".to_string(), "Implement feature X".to_string())),
            ),
            (
                "// FIXME: Critical bug in Y",
                Some(("FIXME".to_string(), "Critical bug in Y".to_string())),
            ),
            (
                "# TODO: Add documentation",
                Some(("TODO".to_string(), "Add documentation".to_string())),
            ),
            (
                "/* HACK: Temporary workaround */",
                Some(("HACK".to_string(), "Temporary workaround".to_string())),
            ),
            ("Regular comment without TODO", None),
        ];

        for (input, expected) in test_cases {
            let result = extractor.extract_todo_from_line(input, 1);
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_priority_mapping() {
        let config = TodoConfig::default();
        assert_eq!(config.priority_mapping.get("FIXME"), Some(&Priority::High));
        assert_eq!(config.priority_mapping.get("TODO"), Some(&Priority::Medium));
        assert_eq!(config.priority_mapping.get("HACK"), Some(&Priority::Low));
    }

    #[test]
    fn test_category_inference() {
        let extractor = TodoExtractor::new();

        assert_eq!(extractor.infer_category("Implement new feature"), RequirementType::Functional);
        assert_eq!(extractor.infer_category("Fix memory leak"), RequirementType::Reliability);
        assert_eq!(extractor.infer_category("Add API endpoint"), RequirementType::Api);
        assert_eq!(extractor.infer_category("Improve security checks"), RequirementType::Security);
    }

    #[test]
    fn test_file_pattern_matching() {
        let extractor = TodoExtractor::new();

        assert!(extractor.matches_pattern("main.rs", "*.rs"));
        assert!(extractor.matches_pattern("README.md", "*.md"));
        assert!(!extractor.matches_pattern("main.rs", "*.py"));
    }

    #[tokio::test]
    async fn test_extract_from_content() {
        let extractor = TodoExtractor::new();

        let content = r#"
// TODO: Implement user authentication
// FIXME: Memory leak in connection pool
fn main() {
    // HACK: Temporary workaround for issue #123
    println!("Hello");
}
// Regular comment
"#;

        let path = PathBuf::from("test.rs");
        let requirements = extractor.extract_from_content(content, &path).unwrap();

        assert_eq!(requirements.len(), 3);

        // Check first TODO
        assert!(requirements[0].description.contains("user authentication"));
        assert_eq!(requirements[0].priority, Priority::Medium);

        // Check FIXME
        assert!(requirements[1].description.contains("Memory leak"));
        assert_eq!(requirements[1].priority, Priority::High);

        // Check HACK
        assert!(requirements[2].description.contains("Temporary workaround"));
        assert_eq!(requirements[2].priority, Priority::Low);
    }
}
