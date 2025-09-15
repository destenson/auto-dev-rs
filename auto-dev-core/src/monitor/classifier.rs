#![allow(unused)]
//! File type classification based on patterns and extensions

use std::collections::HashMap;
use std::path::Path;

/// Categories of files to monitor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileCategory {
    Specification,  // SPEC.md, requirements.*, design.*
    Documentation,  // README.md, docs/*, *.md
    Test,           // *_test.rs, test_*.rs, tests/*
    Implementation, // *.rs, *.py, *.js (existing code)
    Configuration,  // Cargo.toml, package.json, etc.
    Schema,         // *.yaml, *.json schema files
    Example,        // examples/*, *.example.*
    Other,          // Everything else (not monitored)
}

/// Classifies files into categories based on patterns
pub struct FileClassifier {
    patterns: HashMap<FileCategory, Vec<Pattern>>,
}

struct Pattern {
    pattern_type: PatternType,
    value: String,
}

enum PatternType {
    Extension,    // File extension match
    Filename,     // Exact filename match
    PathContains, // Path contains substring
    Prefix,       // Filename starts with
    Suffix,       // Filename ends with
    Glob,         // Glob pattern
}

impl FileClassifier {
    /// Create a new file classifier with default patterns
    pub fn new() -> Self {
        let mut patterns = HashMap::new();

        // Specification patterns - check exact filenames first
        patterns.insert(
            FileCategory::Specification,
            vec![
                Pattern { pattern_type: PatternType::Filename, value: "spec.md".to_string() },
                Pattern {
                    pattern_type: PatternType::Filename,
                    value: "specification.md".to_string(),
                },
                Pattern {
                    pattern_type: PatternType::PathContains,
                    value: "requirements".to_string(),
                },
                Pattern { pattern_type: PatternType::PathContains, value: "design".to_string() },
                Pattern {
                    pattern_type: PatternType::PathContains,
                    value: "architecture".to_string(),
                },
                Pattern { pattern_type: PatternType::Prefix, value: "spec_".to_string() },
                Pattern { pattern_type: PatternType::Prefix, value: "requirement_".to_string() },
            ],
        );

        // Documentation patterns
        patterns.insert(
            FileCategory::Documentation,
            vec![
                Pattern { pattern_type: PatternType::Extension, value: "md".to_string() },
                Pattern { pattern_type: PatternType::Extension, value: "rst".to_string() },
                Pattern { pattern_type: PatternType::Extension, value: "txt".to_string() },
                Pattern { pattern_type: PatternType::PathContains, value: "docs".to_string() },
                Pattern {
                    pattern_type: PatternType::PathContains,
                    value: "documentation".to_string(),
                },
                Pattern { pattern_type: PatternType::Filename, value: "readme.md".to_string() },
                Pattern { pattern_type: PatternType::Filename, value: "api.md".to_string() },
                Pattern { pattern_type: PatternType::Filename, value: "changelog.md".to_string() },
            ],
        );

        // Test patterns
        patterns.insert(
            FileCategory::Test,
            vec![
                Pattern { pattern_type: PatternType::Suffix, value: "_test.rs".to_string() },
                Pattern { pattern_type: PatternType::Suffix, value: "_test.py".to_string() },
                Pattern { pattern_type: PatternType::Suffix, value: ".test.js".to_string() },
                Pattern { pattern_type: PatternType::Suffix, value: ".test.ts".to_string() },
                Pattern { pattern_type: PatternType::Suffix, value: ".spec.js".to_string() },
                Pattern { pattern_type: PatternType::Suffix, value: ".spec.ts".to_string() },
                Pattern { pattern_type: PatternType::Prefix, value: "test_".to_string() },
                Pattern { pattern_type: PatternType::PathContains, value: "tests".to_string() },
                Pattern { pattern_type: PatternType::PathContains, value: "__tests__".to_string() },
            ],
        );

        // Implementation patterns
        patterns.insert(
            FileCategory::Implementation,
            vec![
                Pattern { pattern_type: PatternType::Extension, value: "rs".to_string() },
                Pattern { pattern_type: PatternType::Extension, value: "py".to_string() },
                Pattern { pattern_type: PatternType::Extension, value: "js".to_string() },
                Pattern { pattern_type: PatternType::Extension, value: "ts".to_string() },
                Pattern { pattern_type: PatternType::Extension, value: "jsx".to_string() },
                Pattern { pattern_type: PatternType::Extension, value: "tsx".to_string() },
                Pattern { pattern_type: PatternType::Extension, value: "go".to_string() },
                Pattern { pattern_type: PatternType::Extension, value: "java".to_string() },
                Pattern { pattern_type: PatternType::Extension, value: "cpp".to_string() },
                Pattern { pattern_type: PatternType::Extension, value: "c".to_string() },
                Pattern { pattern_type: PatternType::Extension, value: "h".to_string() },
                Pattern { pattern_type: PatternType::Extension, value: "hpp".to_string() },
            ],
        );

        // Configuration patterns
        patterns.insert(
            FileCategory::Configuration,
            vec![
                Pattern { pattern_type: PatternType::Filename, value: "Cargo.toml".to_string() },
                Pattern { pattern_type: PatternType::Filename, value: "package.json".to_string() },
                Pattern {
                    pattern_type: PatternType::Filename,
                    value: "pyproject.toml".to_string(),
                },
                Pattern {
                    pattern_type: PatternType::Filename,
                    value: "requirements.txt".to_string(),
                },
                Pattern { pattern_type: PatternType::Filename, value: "Dockerfile".to_string() },
                Pattern {
                    pattern_type: PatternType::Filename,
                    value: "docker-compose.yml".to_string(),
                },
                Pattern { pattern_type: PatternType::Filename, value: ".env".to_string() },
                Pattern { pattern_type: PatternType::Extension, value: "toml".to_string() },
                Pattern { pattern_type: PatternType::Extension, value: "ini".to_string() },
                Pattern { pattern_type: PatternType::Extension, value: "cfg".to_string() },
            ],
        );

        // Schema patterns
        patterns.insert(
            FileCategory::Schema,
            vec![
                Pattern { pattern_type: PatternType::PathContains, value: "schemas".to_string() },
                Pattern { pattern_type: PatternType::Suffix, value: ".schema.json".to_string() },
                Pattern { pattern_type: PatternType::Suffix, value: ".schema.yaml".to_string() },
                Pattern { pattern_type: PatternType::PathContains, value: "openapi".to_string() },
                Pattern { pattern_type: PatternType::PathContains, value: "graphql".to_string() },
                Pattern { pattern_type: PatternType::Extension, value: "proto".to_string() },
            ],
        );

        // Example patterns
        patterns.insert(
            FileCategory::Example,
            vec![
                Pattern { pattern_type: PatternType::PathContains, value: "examples".to_string() },
                Pattern { pattern_type: PatternType::PathContains, value: "example".to_string() },
                Pattern { pattern_type: PatternType::Suffix, value: ".example".to_string() },
                Pattern { pattern_type: PatternType::Prefix, value: "example_".to_string() },
            ],
        );

        Self { patterns }
    }

    /// Classify a file based on its path
    pub fn classify(&self, path: &Path) -> FileCategory {
        let path_str = path.to_string_lossy().to_lowercase();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

        // Check each category in priority order
        let priority_order = [
            FileCategory::Test,           // Tests have highest priority
            FileCategory::Specification,  // Then specs
            FileCategory::Schema,         // Then schemas
            FileCategory::Example,        // Then examples
            FileCategory::Configuration,  // Then config
            FileCategory::Documentation,  // Then docs
            FileCategory::Implementation, // Then code
        ];

        for category in priority_order {
            if let Some(patterns) = self.patterns.get(&category) {
                for pattern in patterns {
                    if self.matches_pattern(pattern, &path_str, &file_name, &extension) {
                        return category;
                    }
                }
            }
        }

        FileCategory::Other
    }

    /// Check if a file matches a pattern
    fn matches_pattern(
        &self,
        pattern: &Pattern,
        path: &str,
        file_name: &str,
        extension: &str,
    ) -> bool {
        match pattern.pattern_type {
            PatternType::Extension => extension == pattern.value,
            PatternType::Filename => file_name == pattern.value,
            PatternType::PathContains => path.contains(&pattern.value),
            PatternType::Prefix => file_name.starts_with(&pattern.value),
            PatternType::Suffix => file_name.ends_with(&pattern.value),
            PatternType::Glob => {
                // Simple glob matching (can be enhanced with glob crate if needed)
                if pattern.value.contains('*') {
                    let parts: Vec<&str> = pattern.value.split('*').collect();
                    if parts.len() == 2 {
                        return path.starts_with(parts[0]) && path.ends_with(parts[1]);
                    }
                }
                false
            }
        }
    }

    /// Add a custom pattern for a category
    pub fn add_pattern(
        &mut self,
        category: FileCategory,
        pattern_type: PatternType,
        value: String,
    ) {
        self.patterns
            .entry(category)
            .or_insert_with(Vec::new)
            .push(Pattern { pattern_type, value });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_classify_specification() {
        let classifier = FileClassifier::new();

        assert_eq!(classifier.classify(Path::new("SPEC.md")), FileCategory::Specification);
        assert_eq!(
            classifier.classify(Path::new("requirements/auth.md")),
            FileCategory::Specification
        );
        assert_eq!(
            classifier.classify(Path::new("design/architecture.md")),
            FileCategory::Specification
        );
    }

    #[test]
    fn test_classify_test() {
        let classifier = FileClassifier::new();

        assert_eq!(classifier.classify(Path::new("auth_test.rs")), FileCategory::Test);
        assert_eq!(classifier.classify(Path::new("test_utils.rs")), FileCategory::Test);
        assert_eq!(classifier.classify(Path::new("tests/integration.rs")), FileCategory::Test);
        assert_eq!(classifier.classify(Path::new("app.test.js")), FileCategory::Test);
    }

    #[test]
    fn test_classify_implementation() {
        let classifier = FileClassifier::new();

        assert_eq!(classifier.classify(Path::new("main.rs")), FileCategory::Implementation);
        assert_eq!(classifier.classify(Path::new("app.py")), FileCategory::Implementation);
        assert_eq!(classifier.classify(Path::new("index.js")), FileCategory::Implementation);
    }

    #[test]
    fn test_priority_order() {
        let classifier = FileClassifier::new();

        // A file in tests/ directory with .rs extension should be classified as Test, not Implementation
        assert_eq!(classifier.classify(Path::new("tests/main.rs")), FileCategory::Test);

        // A file named test_spec.md should be classified as Test, not Documentation
        assert_eq!(classifier.classify(Path::new("test_spec.md")), FileCategory::Test);
    }
}
