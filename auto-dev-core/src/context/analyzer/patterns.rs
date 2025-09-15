use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

use crate::context::manager::CodeExample;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodePattern {
    pub name: String,
    pub description: String,
    pub examples: Vec<CodeExample>,
    pub frequency: f32,
    pub locations: Vec<PathBuf>,
    pub pattern_type: PatternType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    Architectural, // MVC, Hexagonal, etc.
    Design,        // Factory, Observer, etc.
    Idiom,         // Language-specific idioms
    Convention,    // Naming, structure, etc.
}

#[derive(Debug)]
pub struct PatternDetector {
    project_root: PathBuf,
    pattern_library: PatternLibrary,
}

#[derive(Debug, Clone)]
struct PatternLibrary {
    architectural_patterns: Vec<ArchPattern>,
    design_patterns: Vec<DesignPattern>,
    idioms: HashMap<String, Vec<Idiom>>,
    anti_patterns: Vec<AntiPattern>,
}

#[derive(Debug, Clone)]
struct ArchPattern {
    name: String,
    indicators: Vec<String>,
    description: String,
}

#[derive(Debug, Clone)]
struct DesignPattern {
    name: String,
    regex_patterns: Vec<Regex>,
    description: String,
}

#[derive(Debug, Clone)]
struct Idiom {
    name: String,
    language: String,
    pattern: Regex,
    description: String,
}

#[derive(Debug, Clone)]
struct AntiPattern {
    name: String,
    pattern: Regex,
    description: String,
}

impl PatternDetector {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root, pattern_library: Self::initialize_pattern_library() }
    }

    fn initialize_pattern_library() -> PatternLibrary {
        let mut library = PatternLibrary {
            architectural_patterns: Vec::new(),
            design_patterns: Vec::new(),
            idioms: HashMap::new(),
            anti_patterns: Vec::new(),
        };

        // Architectural patterns
        library.architectural_patterns.push(ArchPattern {
            name: "MVC".to_string(),
            indicators: vec!["models".to_string(), "views".to_string(), "controllers".to_string()],
            description: "Model-View-Controller pattern".to_string(),
        });

        library.architectural_patterns.push(ArchPattern {
            name: "Layered Architecture".to_string(),
            indicators: vec![
                "presentation".to_string(),
                "business".to_string(),
                "data".to_string(),
                "domain".to_string(),
            ],
            description: "Layered architectural pattern".to_string(),
        });

        // Design patterns
        library.design_patterns.push(DesignPattern {
            name: "Singleton".to_string(),
            regex_patterns: vec![
                Regex::new(r"static\s+(?:mut\s+)?INSTANCE").unwrap(),
                Regex::new(r"fn\s+instance\s*\(\s*\)\s*->").unwrap(),
                Regex::new(r"lazy_static!").unwrap(),
            ],
            description: "Singleton pattern".to_string(),
        });

        library.design_patterns.push(DesignPattern {
            name: "Factory".to_string(),
            regex_patterns: vec![
                Regex::new(r"fn\s+create_\w+").unwrap(),
                Regex::new(r"fn\s+new_\w+").unwrap(),
                Regex::new(r"Factory(?:Trait)?").unwrap(),
            ],
            description: "Factory pattern".to_string(),
        });

        library.design_patterns.push(DesignPattern {
            name: "Builder".to_string(),
            regex_patterns: vec![
                Regex::new(r"struct\s+\w+Builder").unwrap(),
                Regex::new(r"fn\s+build\s*\(\s*.*\s*\)\s*->").unwrap(),
                Regex::new(r"fn\s+with_\w+").unwrap(),
            ],
            description: "Builder pattern".to_string(),
        });

        // Rust idioms
        let mut rust_idioms = Vec::new();
        rust_idioms.push(Idiom {
            name: "Result Type".to_string(),
            language: "Rust".to_string(),
            pattern: Regex::new(r"Result<.*>").unwrap(),
            description: "Rust Result type for error handling".to_string(),
        });

        rust_idioms.push(Idiom {
            name: "Option Type".to_string(),
            language: "Rust".to_string(),
            pattern: Regex::new(r"Option<.*>").unwrap(),
            description: "Rust Option type for nullable values".to_string(),
        });

        rust_idioms.push(Idiom {
            name: "Match Expression".to_string(),
            language: "Rust".to_string(),
            pattern: Regex::new(r"match\s+\w+\s*\{").unwrap(),
            description: "Rust pattern matching".to_string(),
        });

        library.idioms.insert("Rust".to_string(), rust_idioms);

        // Python idioms
        let mut python_idioms = Vec::new();
        python_idioms.push(Idiom {
            name: "List Comprehension".to_string(),
            language: "Python".to_string(),
            pattern: Regex::new(r"\[.*for.*in.*\]").unwrap(),
            description: "Python list comprehension".to_string(),
        });

        python_idioms.push(Idiom {
            name: "Context Manager".to_string(),
            language: "Python".to_string(),
            pattern: Regex::new(r"with\s+.*as\s+\w+:").unwrap(),
            description: "Python context manager".to_string(),
        });

        library.idioms.insert("Python".to_string(), python_idioms);

        // Anti-patterns
        library.anti_patterns.push(AntiPattern {
            name: "God Object".to_string(),
            pattern: Regex::new(r"impl\s+\w+\s*\{[\s\S]{5000,}\}").unwrap(),
            description: "Class or struct with too many responsibilities".to_string(),
        });

        library
    }

    pub async fn detect_all_patterns(&self) -> anyhow::Result<Vec<CodePattern>> {
        let mut patterns = Vec::new();
        let mut pattern_counts: HashMap<String, (Vec<PathBuf>, Vec<CodeExample>)> = HashMap::new();

        // Walk through source files
        for entry in walkdir::WalkDir::new(&self.project_root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !is_ignored(e.path()))
        {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && is_source_file(path) {
                let file_patterns = self.analyze_file(&path.to_path_buf()).await?;

                for pattern in file_patterns {
                    let entry = pattern_counts
                        .entry(pattern.name.clone())
                        .or_insert_with(|| (Vec::new(), Vec::new()));
                    entry.0.push(path.to_path_buf());
                    entry.1.extend(pattern.examples);
                }
            }
        }

        // Convert to CodePattern with frequency calculation
        let total_files = pattern_counts.values().map(|(locs, _)| locs.len()).sum::<usize>() as f32;

        for (name, (locations, examples)) in pattern_counts {
            let frequency = locations.len() as f32 / total_files.max(1.0);

            // Find pattern type
            let pattern_type = self.determine_pattern_type(&name);

            patterns.push(CodePattern {
                name: name.clone(),
                description: self.get_pattern_description(&name),
                examples: examples.into_iter().take(5).collect(), // Keep top 5 examples
                frequency,
                locations,
                pattern_type,
            });
        }

        Ok(patterns)
    }

    pub async fn analyze_file(&self, path: &PathBuf) -> anyhow::Result<Vec<CodePattern>> {
        let mut patterns = Vec::new();

        if let Ok(content) = fs::read_to_string(path).await {
            let language = detect_language_from_path(path);

            // Check architectural patterns
            for arch_pattern in &self.pattern_library.architectural_patterns {
                if path.to_string_lossy().contains(&arch_pattern.name.to_lowercase())
                    || arch_pattern
                        .indicators
                        .iter()
                        .any(|ind| path.to_string_lossy().contains(ind))
                {
                    patterns.push(CodePattern {
                        name: arch_pattern.name.clone(),
                        description: arch_pattern.description.clone(),
                        examples: vec![create_example(path, 0, 0, "")],
                        frequency: 0.0,
                        locations: vec![path.clone()],
                        pattern_type: PatternType::Architectural,
                    });
                }
            }

            // Check design patterns
            for design_pattern in &self.pattern_library.design_patterns {
                for regex in &design_pattern.regex_patterns {
                    if regex.is_match(&content) {
                        let example = extract_pattern_example(&content, regex, path);
                        patterns.push(CodePattern {
                            name: design_pattern.name.clone(),
                            description: design_pattern.description.clone(),
                            examples: vec![example],
                            frequency: 0.0,
                            locations: vec![path.clone()],
                            pattern_type: PatternType::Design,
                        });
                        break;
                    }
                }
            }

            // Check language-specific idioms
            if let Some(lang) = language {
                if let Some(idioms) = self.pattern_library.idioms.get(&lang) {
                    for idiom in idioms {
                        if idiom.pattern.is_match(&content) {
                            let example = extract_pattern_example(&content, &idiom.pattern, path);
                            patterns.push(CodePattern {
                                name: idiom.name.clone(),
                                description: idiom.description.clone(),
                                examples: vec![example],
                                frequency: 0.0,
                                locations: vec![path.clone()],
                                pattern_type: PatternType::Idiom,
                            });
                        }
                    }
                }
            }

            // Check anti-patterns
            for anti_pattern in &self.pattern_library.anti_patterns {
                if anti_pattern.pattern.is_match(&content) {
                    let example = extract_pattern_example(&content, &anti_pattern.pattern, path);
                    patterns.push(CodePattern {
                        name: format!("Anti-pattern: {}", anti_pattern.name),
                        description: anti_pattern.description.clone(),
                        examples: vec![example],
                        frequency: 0.0,
                        locations: vec![path.clone()],
                        pattern_type: PatternType::Convention,
                    });
                }
            }
        }

        Ok(patterns)
    }

    fn determine_pattern_type(&self, name: &str) -> PatternType {
        if self.pattern_library.architectural_patterns.iter().any(|p| p.name == name) {
            PatternType::Architectural
        } else if self.pattern_library.design_patterns.iter().any(|p| p.name == name) {
            PatternType::Design
        } else if name.starts_with("Anti-pattern:") {
            PatternType::Convention
        } else {
            PatternType::Idiom
        }
    }

    fn get_pattern_description(&self, name: &str) -> String {
        if let Some(pattern) =
            self.pattern_library.architectural_patterns.iter().find(|p| p.name == name)
        {
            return pattern.description.clone();
        }
        if let Some(pattern) = self.pattern_library.design_patterns.iter().find(|p| p.name == name)
        {
            return pattern.description.clone();
        }
        for idioms in self.pattern_library.idioms.values() {
            if let Some(idiom) = idioms.iter().find(|i| i.name == name) {
                return idiom.description.clone();
            }
        }
        if let Some(pattern) = self
            .pattern_library
            .anti_patterns
            .iter()
            .find(|p| format!("Anti-pattern: {}", p.name) == name)
        {
            return pattern.description.clone();
        }
        "Unknown pattern".to_string()
    }
}

fn detect_language_from_path(path: &PathBuf) -> Option<String> {
    path.extension().and_then(|ext| ext.to_str()).and_then(|ext| match ext {
        "rs" => Some("Rust".to_string()),
        "py" => Some("Python".to_string()),
        "js" | "mjs" => Some("JavaScript".to_string()),
        "ts" => Some("TypeScript".to_string()),
        "go" => Some("Go".to_string()),
        "java" => Some("Java".to_string()),
        "cpp" | "cc" | "cxx" => Some("C++".to_string()),
        "c" => Some("C".to_string()),
        "cs" => Some("C#".to_string()),
        _ => None,
    })
}

fn is_source_file(path: &std::path::Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        matches!(
            ext,
            "rs" | "py"
                | "js"
                | "ts"
                | "go"
                | "java"
                | "cpp"
                | "c"
                | "cs"
                | "rb"
                | "php"
                | "swift"
                | "kt"
        )
    } else {
        false
    }
}

fn is_ignored(path: &std::path::Path) -> bool {
    path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        name.starts_with('.') && name != "." && name != ".."
            || name == "node_modules"
            || name == "target"
            || name == "dist"
            || name == "__pycache__"
            || name == ".git"
            || name == "vendor"
    })
}

fn extract_pattern_example(content: &str, regex: &Regex, path: &PathBuf) -> CodeExample {
    if let Some(mat) = regex.find(content) {
        let start_byte = mat.start();
        let end_byte = mat.end();

        // Find line numbers
        let lines: Vec<&str> = content.lines().collect();
        let mut current_byte = 0;
        let mut start_line = 1;
        let mut end_line = 1;

        for (i, line) in lines.iter().enumerate() {
            let line_len = line.len() + 1; // +1 for newline
            if current_byte <= start_byte && start_byte < current_byte + line_len {
                start_line = i + 1;
            }
            if current_byte <= end_byte && end_byte <= current_byte + line_len {
                end_line = i + 1;
                break;
            }
            current_byte += line_len;
        }

        // Extract a bit more context
        let context_start = start_line.saturating_sub(2);
        let context_end = (end_line + 2).min(lines.len());
        let code_snippet = lines[context_start..context_end].join("\n");

        create_example(path, start_line, end_line, &code_snippet)
    } else {
        create_example(path, 0, 0, "")
    }
}

fn create_example(path: &PathBuf, start: usize, end: usize, code: &str) -> CodeExample {
    CodeExample {
        file_path: path.clone(),
        line_start: start,
        line_end: end,
        code: code.to_string(),
        description: None,
    }
}
