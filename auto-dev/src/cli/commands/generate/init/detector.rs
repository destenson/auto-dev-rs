//! Project type detection from instructions

use super::instructions::InstructionDocument;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProjectType {
    Rust,
    Python,
    JavaScript,      // Node.js projects
    TypeScript,      // Node.js with TypeScript  
    Deno,           // JavaScript/TypeScript with Deno runtime
    DotNet,
    Go,
    Java,
    Generic,
}

pub struct ProjectDetector {
    keywords: HashMap<ProjectType, Vec<&'static str>>,
}

impl ProjectDetector {
    pub fn new() -> Self {
        let mut keywords = HashMap::new();
        
        // Rust keywords
        keywords.insert(ProjectType::Rust, vec![
            "rust", "cargo", "crate", "actix", "axum", "tokio", "async", "wasm",
            "rustc", "clippy", "rustfmt", "serde", "rocket", "bevy"
        ]);
        
        // Python keywords  
        keywords.insert(ProjectType::Python, vec![
            "python", "pip", "uv", "django", "flask", "fastapi", "pandas", "numpy",
            "pytest", "poetry", "virtualenv", "requirements.txt", "scikit", "tensorflow"
        ]);
        
        // JavaScript keywords
        keywords.insert(ProjectType::JavaScript, vec![
            "javascript", "js", "node", "npm", "express", "react", "vue", "angular",
            "webpack", "babel", "jest", "mocha", "package.json", "yarn"
        ]);
        
        // TypeScript keywords
        keywords.insert(ProjectType::TypeScript, vec![
            "typescript", "ts", "tsx", "tsc", "tsconfig", "nestjs", "next.js",
            "type-safe", "typed", "interface", "enum", "decorator"
        ]);
        
        // Deno keywords
        keywords.insert(ProjectType::Deno, vec![
            "deno", "deno.land", "fresh", "oak", "deno.json", "import_map"
        ]);
        
        // .NET keywords
        keywords.insert(ProjectType::DotNet, vec![
            "c#", "csharp", ".net", "dotnet", "asp.net", "blazor", "entity framework",
            "nuget", "visual studio", "msbuild", "xamarin", "maui"
        ]);
        
        // Go keywords
        keywords.insert(ProjectType::Go, vec![
            "go", "golang", "gin", "echo", "fiber", "gorilla", "go.mod", "gofmt",
            "goroutine", "channel", "interface{}", "package main"
        ]);
        
        // Java keywords
        keywords.insert(ProjectType::Java, vec![
            "java", "spring", "maven", "gradle", "junit", "hibernate", "servlet",
            "jvm", "jar", "classpath", "lombok", "jackson"
        ]);
        
        Self { keywords }
    }
    
    pub fn detect(&self, instructions: &InstructionDocument) -> ProjectType {
        let text = instructions.raw_content.to_lowercase();
        let mut scores: HashMap<ProjectType, usize> = HashMap::new();
        
        // Score each project type based on keyword matches
        for (project_type, keywords) in &self.keywords {
            let mut score = 0;
            for keyword in keywords {
                if text.contains(keyword) {
                    score += 1;
                    // Bonus points for exact matches in metadata
                    if let Some(lang) = &instructions.metadata.language {
                        if lang.to_lowercase() == *keyword {
                            score += 5;
                        }
                    }
                }
            }
            if score > 0 {
                scores.insert(project_type.clone(), score);
            }
        }
        
        // Check explicit language in metadata
        if let Some(lang) = &instructions.metadata.language {
            let lang_lower = lang.to_lowercase();
            if lang_lower.contains("rust") {
                return ProjectType::Rust;
            } else if lang_lower.contains("python") || lang_lower.contains("py") {
                return ProjectType::Python;
            } else if lang_lower.contains("deno") {
                // Deno is explicitly mentioned - use Deno runtime
                return ProjectType::Deno;
            } else if lang_lower.contains("typescript") || lang_lower.contains("ts") {
                // Check if Deno is mentioned in the full text
                if text.contains("deno") {
                    return ProjectType::Deno;
                }
                return ProjectType::TypeScript;
            } else if lang_lower.contains("javascript") || lang_lower.contains("js") {
                // Check if Deno is mentioned in the full text
                if text.contains("deno") {
                    return ProjectType::Deno;
                }
                return ProjectType::JavaScript;
            } else if lang_lower.contains("c#") || lang_lower.contains("csharp") || lang_lower.contains(".net") {
                return ProjectType::DotNet;
            } else if lang_lower.contains("go") {
                return ProjectType::Go;
            } else if lang_lower.contains("java") && !lang_lower.contains("javascript") {
                return ProjectType::Java;
            }
        }
        
        // Return highest scoring type, or Generic if no matches
        scores.into_iter()
            .max_by_key(|(_, score)| *score)
            .map(|(project_type, _)| project_type)
            .unwrap_or(ProjectType::Generic)
    }
}

impl Default for ProjectDetector {
    fn default() -> Self {
        Self::new()
    }
}