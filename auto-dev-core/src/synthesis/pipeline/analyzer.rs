//! Code analyzer for understanding existing code

use super::{PipelineContext, PipelineStage};
use crate::synthesis::{Result, SynthesisError};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Analyzes existing code to understand current implementation
pub struct CodeAnalyzer {
    parsers: HashMap<String, Box<dyn LanguageParser>>,
}

impl CodeAnalyzer {
    /// Create a new code analyzer
    pub fn new() -> Self {
        let mut parsers: HashMap<String, Box<dyn LanguageParser>> = HashMap::new();

        // Register language parsers
        parsers.insert("rs".to_string(), Box::new(RustParser));
        parsers.insert("py".to_string(), Box::new(PythonParser));
        parsers.insert("js".to_string(), Box::new(JavaScriptParser));
        parsers.insert("ts".to_string(), Box::new(TypeScriptParser));

        Self { parsers }
    }

    /// Analyze a file to extract structure
    async fn analyze_file(&self, path: &Path) -> Result<FileAnalysis> {
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        let content = fs::read_to_string(path).await?;

        let parser = self.parsers.get(extension).ok_or_else(|| {
            SynthesisError::AnalysisError(format!("No parser for extension: {}", extension))
        })?;

        Ok(parser.parse(&content))
    }

    /// Find implementation targets
    fn find_targets(&self, spec: &crate::parser::model::Specification) -> Vec<PathBuf> {
        let mut targets = Vec::new();

        // Based on spec, determine likely implementation files
        // This is a simplified version - real implementation would be more sophisticated

        // Look for main source files
        targets.push(PathBuf::from("src/lib.rs"));
        targets.push(PathBuf::from("src/main.rs"));

        // Look for module files based on spec
        for req in &spec.requirements {
            if req.description.to_lowercase().contains("api") {
                targets.push(PathBuf::from("src/api.rs"));
            }
            if req.description.to_lowercase().contains("model") {
                targets.push(PathBuf::from("src/models.rs"));
            }
        }

        targets
    }
}

#[async_trait]
impl PipelineStage for CodeAnalyzer {
    fn name(&self) -> &'static str {
        "CodeAnalyzer"
    }

    async fn execute(&self, mut context: PipelineContext) -> Result<PipelineContext> {
        tracing::info!(
            "Analyzing existing code for specification: {}",
            context.spec.source.display()
        );

        context.metadata.current_stage = self.name().to_string();

        // Find target files to analyze
        let targets = self.find_targets(&context.spec);

        // Analyze each target file that exists
        let mut analyses = Vec::new();
        for target in targets {
            if target.exists() {
                match self.analyze_file(&target).await {
                    Ok(analysis) => analyses.push(analysis),
                    Err(e) => {
                        context.add_warning(format!(
                            "Failed to analyze {}: {}",
                            target.display(),
                            e
                        ));
                    }
                }
            }
        }

        // Store analysis results in context for later stages
        // In a real implementation, this would be stored in a more structured way
        if analyses.is_empty() {
            context.add_warning("No existing code found to analyze".to_string());
        } else {
            tracing::debug!("Analyzed {} files", analyses.len());
        }

        Ok(context)
    }
}

/// Language-specific parser trait
trait LanguageParser: Send + Sync {
    fn parse(&self, content: &str) -> FileAnalysis;
}

/// Analysis result for a file
#[derive(Debug, Clone)]
struct FileAnalysis {
    pub functions: Vec<FunctionInfo>,
    pub structs: Vec<StructInfo>,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
}

#[derive(Debug, Clone)]
struct FunctionInfo {
    pub name: String,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone)]
struct StructInfo {
    pub name: String,
    pub fields: Vec<String>,
    pub methods: Vec<String>,
}

/// Rust language parser
struct RustParser;

impl LanguageParser for RustParser {
    fn parse(&self, content: &str) -> FileAnalysis {
        // Simplified parsing - real implementation would use syn or tree-sitter
        let mut analysis = FileAnalysis {
            functions: Vec::new(),
            structs: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
        };

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Parse functions
            if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
                if let Some(name) = extract_function_name(trimmed) {
                    analysis.functions.push(FunctionInfo {
                        name,
                        parameters: Vec::new(),
                        return_type: None,
                        line_start: line_num + 1,
                        line_end: line_num + 1,
                    });
                }
            }

            // Parse structs
            if trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ") {
                if let Some(name) = extract_struct_name(trimmed) {
                    analysis.structs.push(StructInfo {
                        name,
                        fields: Vec::new(),
                        methods: Vec::new(),
                    });
                }
            }

            // Parse imports
            if trimmed.starts_with("use ") {
                analysis.imports.push(trimmed.to_string());
            }
        }

        analysis
    }
}

/// Python language parser
struct PythonParser;

impl LanguageParser for PythonParser {
    fn parse(&self, content: &str) -> FileAnalysis {
        let mut analysis = FileAnalysis {
            functions: Vec::new(),
            structs: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
        };

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            if trimmed.starts_with("def ") {
                if let Some(name) = extract_python_function_name(trimmed) {
                    analysis.functions.push(FunctionInfo {
                        name,
                        parameters: Vec::new(),
                        return_type: None,
                        line_start: line_num + 1,
                        line_end: line_num + 1,
                    });
                }
            }

            if trimmed.starts_with("class ") {
                if let Some(name) = extract_python_class_name(trimmed) {
                    analysis.structs.push(StructInfo {
                        name,
                        fields: Vec::new(),
                        methods: Vec::new(),
                    });
                }
            }

            if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                analysis.imports.push(trimmed.to_string());
            }
        }

        analysis
    }
}

/// JavaScript parser
struct JavaScriptParser;

impl LanguageParser for JavaScriptParser {
    fn parse(&self, content: &str) -> FileAnalysis {
        FileAnalysis {
            functions: Vec::new(),
            structs: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
        }
    }
}

/// TypeScript parser
struct TypeScriptParser;

impl LanguageParser for TypeScriptParser {
    fn parse(&self, content: &str) -> FileAnalysis {
        FileAnalysis {
            functions: Vec::new(),
            structs: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
        }
    }
}

// Helper functions for parsing
fn extract_function_name(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    parts
        .iter()
        .position(|&p| p == "fn")
        .and_then(|i| parts.get(i + 1))
        .and_then(|&name| name.split('(').next())
        .map(|s| s.to_string())
}

fn extract_struct_name(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    parts
        .iter()
        .position(|&p| p == "struct")
        .and_then(|i| parts.get(i + 1))
        .map(|&s| s.trim_end_matches('{').to_string())
}

fn extract_python_function_name(line: &str) -> Option<String> {
    line.strip_prefix("def ").and_then(|s| s.split('(').next()).map(|s| s.trim().to_string())
}

fn extract_python_class_name(line: &str) -> Option<String> {
    line.strip_prefix("class ")
        .and_then(|s| s.split([':', '(']).next())
        .map(|s| s.trim().to_string())
}
