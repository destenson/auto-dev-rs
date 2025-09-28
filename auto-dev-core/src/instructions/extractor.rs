//! Metadata extraction from parsed instructions
//!
//! Extracts project metadata, dependencies, and other structured information.

use crate::instructions::parser::ParsedInstruction;
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::debug;

/// Extracted project metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectMetadata {
    /// Project name (if detected)
    pub name: Option<String>,
    /// Project type (web, cli, library, etc.)
    pub project_type: Option<ProjectType>,
    /// Programming language
    pub language: Option<String>,
    /// Detected frameworks
    pub frameworks: Vec<String>,
    /// Detected dependencies
    pub dependencies: Vec<String>,
    /// Feature requirements
    pub features: Vec<String>,
    /// Additional metadata
    pub custom: HashMap<String, String>,
}

/// Types of projects
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectType {
    WebApplication,
    CliTool,
    Library,
    Api,
    Desktop,
    Mobile,
    Game,
    SystemService,
    Unknown,
}

/// Metadata extractor
pub struct MetadataExtractor;

impl MetadataExtractor {
    /// Extract metadata from parsed instructions
    pub fn extract(instruction: &ParsedInstruction) -> Result<ProjectMetadata> {
        let mut metadata = ProjectMetadata::default();
        
        // Extract from explicit metadata
        metadata.extract_from_metadata(&instruction.metadata);
        
        // Extract from content using patterns
        metadata.extract_from_content(instruction.get_main_content());
        
        // Extract from sections
        metadata.extract_from_sections(&instruction.sections);
        
        // Infer project type if not explicitly set
        if metadata.project_type.is_none() {
            metadata.project_type = Some(Self::infer_project_type(&metadata, instruction));
        }
        
        debug!("Extracted metadata: {:?}", metadata);
        Ok(metadata)
    }
    
    /// Infer project type from available information
    fn infer_project_type(metadata: &ProjectMetadata, instruction: &ParsedInstruction) -> ProjectType {
        let content = instruction.get_main_content().to_lowercase();
        
        // Check for specific keywords
        let type_keywords = [
            ("web app", ProjectType::WebApplication),
            ("web server", ProjectType::WebApplication),
            ("website", ProjectType::WebApplication),
            ("api", ProjectType::Api),
            ("rest api", ProjectType::Api),
            ("graphql", ProjectType::Api),
            ("cli", ProjectType::CliTool),
            ("command line", ProjectType::CliTool),
            ("terminal", ProjectType::CliTool),
            ("library", ProjectType::Library),
            ("package", ProjectType::Library),
            ("crate", ProjectType::Library),
            ("desktop app", ProjectType::Desktop),
            ("gui", ProjectType::Desktop),
            ("mobile app", ProjectType::Mobile),
            ("ios", ProjectType::Mobile),
            ("android", ProjectType::Mobile),
            ("game", ProjectType::Game),
            ("service", ProjectType::SystemService),
            ("daemon", ProjectType::SystemService),
        ];
        
        for (keyword, project_type) in type_keywords {
            if content.contains(keyword) {
                return project_type;
            }
        }
        
        // Check frameworks for hints
        for framework in &metadata.frameworks {
            let framework_lower = framework.to_lowercase();
            if framework_lower.contains("actix") || framework_lower.contains("rocket") ||
               framework_lower.contains("express") || framework_lower.contains("flask") {
                return ProjectType::WebApplication;
            }
            if framework_lower.contains("clap") || framework_lower.contains("structopt") {
                return ProjectType::CliTool;
            }
        }
        
        ProjectType::Unknown
    }
}

impl ProjectMetadata {
    /// Extract from explicit metadata map
    fn extract_from_metadata(&mut self, metadata: &HashMap<String, String>) {
        // Direct mappings
        if let Some(name) = metadata.get("name").or_else(|| metadata.get("project_name")) {
            self.name = Some(name.clone());
        }
        
        if let Some(lang) = metadata.get("language").or_else(|| metadata.get("lang")) {
            self.language = Some(lang.clone());
        }
        
        // Project type
        if let Some(ptype) = metadata.get("type").or_else(|| metadata.get("project_type")) {
            self.project_type = Some(match ptype.to_lowercase().as_str() {
                "web" | "webapp" => ProjectType::WebApplication,
                "cli" | "command" => ProjectType::CliTool,
                "lib" | "library" => ProjectType::Library,
                "api" | "rest" => ProjectType::Api,
                "desktop" | "gui" => ProjectType::Desktop,
                "mobile" | "app" => ProjectType::Mobile,
                "game" => ProjectType::Game,
                "service" | "daemon" => ProjectType::SystemService,
                _ => ProjectType::Unknown,
            });
        }
        
        // Frameworks and dependencies
        if let Some(frameworks) = metadata.get("frameworks").or_else(|| metadata.get("framework")) {
            self.frameworks.extend(
                frameworks.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            );
        }
        
        if let Some(deps) = metadata.get("dependencies").or_else(|| metadata.get("deps")) {
            self.dependencies.extend(
                deps.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            );
        }
        
        // Store remaining as custom
        for (key, value) in metadata {
            if !matches!(key.as_str(), "name" | "project_name" | "language" | "lang" | 
                        "type" | "project_type" | "frameworks" | "framework" | 
                        "dependencies" | "deps" | "instruction" | "instructions") {
                self.custom.insert(key.clone(), value.clone());
            }
        }
    }
    
    /// Extract from instruction content using patterns
    fn extract_from_content(&mut self, content: &str) {
        // Language detection patterns
        let language_patterns = [
            (r"\b(in|using|with)\s+(Rust|rust)\b", "Rust"),
            (r"\b(in|using|with)\s+(Python|python)\b", "Python"),
            (r"\b(in|using|with)\s+(JavaScript|javascript|JS|js)\b", "JavaScript"),
            (r"\b(in|using|with)\s+(TypeScript|typescript|TS|ts)\b", "TypeScript"),
            (r"\b(in|using|with)\s+(Go|golang)\b", "Go"),
            (r"\b(in|using|with)\s+(Java|java)\b", "Java"),
            (r"\b(in|using|with)\s+(C\+\+|cpp|CPP)\b", "C++"),
            (r"\b(in|using|with)\s+(C#|csharp|CSharp)\b", "C#"),
        ];
        
        for (pattern, lang) in language_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(content) {
                    if self.language.is_none() {
                        self.language = Some(lang.to_string());
                    }
                    break;
                }
            }
        }
        
        // Framework detection
        let framework_keywords = [
            "actix", "rocket", "warp", "axum", "tokio",
            "express", "fastapi", "flask", "django",
            "react", "vue", "angular", "svelte",
            "spring", "asp.net", "rails",
        ];
        
        let content_lower = content.to_lowercase();
        for keyword in framework_keywords {
            if content_lower.contains(keyword) {
                self.frameworks.push(keyword.to_string());
            }
        }
        
        // Dependency extraction using common patterns
        let dep_patterns = [
            r"(?:use|import|require|depends on|dependency:?)\s+([a-zA-Z0-9_-]+)",
            r"(?:package|crate|library|module):?\s+([a-zA-Z0-9_-]+)",
        ];
        
        let mut found_deps = HashSet::new();
        for pattern in dep_patterns {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.captures_iter(content) {
                    if let Some(dep) = cap.get(1) {
                        found_deps.insert(dep.as_str().to_string());
                    }
                }
            }
        }
        self.dependencies.extend(found_deps);
        
        // Feature extraction
        let feature_patterns = [
            r"(?:feature|implement|support|include):?\s+([^,\.\n]+)",
            r"(?:should|must|need to)\s+([^,\.\n]+)",
            r"[-*]\s+([A-Z][^,\.\n]+)",  // Bullet points starting with capital
        ];
        
        for pattern in feature_patterns {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.captures_iter(content) {
                    if let Some(feature) = cap.get(1) {
                        let feature_text = feature.as_str().trim();
                        if feature_text.len() > 5 && feature_text.len() < 100 {
                            self.features.push(feature_text.to_string());
                        }
                    }
                }
            }
        }
        
        // Deduplicate
        self.frameworks.sort();
        self.frameworks.dedup();
        self.dependencies.sort();
        self.dependencies.dedup();
        self.features.sort();
        self.features.dedup();
    }
    
    /// Extract from instruction sections
    fn extract_from_sections(&mut self, sections: &[crate::instructions::parser::InstructionSection]) {
        for section in sections {
            let title_lower = section.title.to_lowercase();
            
            // Dependencies section
            if title_lower.contains("dependencies") || title_lower.contains("requirements") {
                for line in section.content.lines() {
                    let trimmed = line.trim().trim_start_matches('-').trim_start_matches('*').trim();
                    if !trimmed.is_empty() && trimmed.len() < 50 {
                        self.dependencies.push(trimmed.to_string());
                    }
                }
            }
            
            // Features section
            if title_lower.contains("features") || title_lower.contains("functionality") {
                for line in section.content.lines() {
                    let trimmed = line.trim().trim_start_matches('-').trim_start_matches('*').trim();
                    if !trimmed.is_empty() && trimmed.len() < 100 {
                        self.features.push(trimmed.to_string());
                    }
                }
            }
            
            // Technology/Stack section
            if title_lower.contains("technology") || title_lower.contains("stack") || 
               title_lower.contains("tech") {
                let content_lower = section.content.to_lowercase();
                
                // Look for language mentions
                if self.language.is_none() {
                    let languages = ["rust", "python", "javascript", "typescript", "go", "java", "c++", "c#"];
                    for lang in languages {
                        if content_lower.contains(lang) {
                            self.language = Some(lang.to_string());
                            break;
                        }
                    }
                }
                
                // Extract frameworks from bullet points
                for line in section.content.lines() {
                    let trimmed = line.trim().trim_start_matches('-').trim_start_matches('*').trim();
                    if !trimmed.is_empty() && trimmed.len() < 50 {
                        self.frameworks.push(trimmed.to_string());
                    }
                }
            }
        }
        
        // Final deduplication
        self.dependencies.sort();
        self.dependencies.dedup();
        self.features.sort();
        self.features.dedup();
        self.frameworks.sort();
        self.frameworks.dedup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instructions::parser::{ParsedInstruction, InstructionSection};
    use crate::instructions::formats::Format;
    
    #[test]
    fn test_extract_language() {
        let instruction = ParsedInstruction {
            raw_content: String::new(),
            format: Format::Text,
            sections: vec![],
            metadata: Default::default(),
            instruction_text: "Build a web server in Rust using Actix".to_string(),
        };
        
        let metadata = MetadataExtractor::extract(&instruction).unwrap();
        assert_eq!(metadata.language, Some("Rust".to_string()));
        assert!(metadata.frameworks.contains(&"actix".to_string()));
    }
    
    #[test]
    fn test_extract_project_type() {
        let instruction = ParsedInstruction {
            raw_content: String::new(),
            format: Format::Text,
            sections: vec![],
            metadata: Default::default(),
            instruction_text: "Create a CLI tool for file processing".to_string(),
        };
        
        let metadata = MetadataExtractor::extract(&instruction).unwrap();
        assert_eq!(metadata.project_type, Some(ProjectType::CliTool));
    }
    
    #[test]
    fn test_extract_from_metadata_map() {
        let mut meta_map = HashMap::new();
        meta_map.insert("name".to_string(), "my-project".to_string());
        meta_map.insert("language".to_string(), "Python".to_string());
        meta_map.insert("frameworks".to_string(), "FastAPI, SQLAlchemy".to_string());
        
        let instruction = ParsedInstruction {
            raw_content: String::new(),
            format: Format::Json,
            sections: vec![],
            metadata: meta_map,
            instruction_text: String::new(),
        };
        
        let metadata = MetadataExtractor::extract(&instruction).unwrap();
        assert_eq!(metadata.name, Some("my-project".to_string()));
        assert_eq!(metadata.language, Some("Python".to_string()));
        assert!(metadata.frameworks.contains(&"FastAPI".to_string()));
        assert!(metadata.frameworks.contains(&"SQLAlchemy".to_string()));
    }
    
    #[test]
    fn test_extract_from_sections() {
        let instruction = ParsedInstruction {
            raw_content: String::new(),
            format: Format::Markdown,
            sections: vec![
                InstructionSection {
                    title: "Dependencies".to_string(),
                    content: "- tokio\n- serde\n- reqwest".to_string(),
                    level: 2,
                },
                InstructionSection {
                    title: "Features".to_string(),
                    content: "- User authentication\n- File upload\n- Real-time updates".to_string(),
                    level: 2,
                },
            ],
            metadata: Default::default(),
            instruction_text: String::new(),
        };
        
        let metadata = MetadataExtractor::extract(&instruction).unwrap();
        assert!(metadata.dependencies.contains(&"tokio".to_string()));
        assert!(metadata.dependencies.contains(&"serde".to_string()));
        assert!(metadata.features.iter().any(|f| f.contains("authentication")));
    }
}