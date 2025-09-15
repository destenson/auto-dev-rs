use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::context::analyzer::{CodePattern, CodingConventions, DependencyGraph, ProjectStructure};
use crate::context::embeddings::EmbeddingStore;
use crate::context::manager::ArchitectureDecision;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub metadata: ProjectMetadata,
    pub structure: ProjectStructure,
    pub patterns: Vec<CodePattern>,
    pub conventions: CodingConventions,
    pub dependencies: DependencyGraph,
    pub decisions: Vec<ArchitectureDecision>,
    #[serde(skip)]
    pub embeddings: Option<()>,
    pub history: ContextHistory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub name: String,
    pub languages: Vec<Language>,
    pub frameworks: Vec<Framework>,
    pub build_systems: Vec<BuildSystem>,
    pub team_size: Option<usize>,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    CSharp,
    Cpp,
    C,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Framework {
    pub name: String,
    pub version: Option<String>,
    pub language: Language,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildSystem {
    Cargo,
    Npm,
    Yarn,
    Pnpm,
    Maven,
    Gradle,
    Make,
    CMake,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextHistory {
    pub events: Vec<HistoryEvent>,
    pub max_events: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub description: String,
    pub metadata: Option<serde_json::Value>,
}

impl Default for ProjectContext {
    fn default() -> Self {
        Self {
            metadata: ProjectMetadata {
                name: String::from("unknown"),
                languages: Vec::new(),
                frameworks: Vec::new(),
                build_systems: Vec::new(),
                team_size: None,
                created_at: Utc::now(),
                last_updated: Utc::now(),
            },
            structure: ProjectStructure::default(),
            patterns: Vec::new(),
            conventions: CodingConventions::default(),
            dependencies: DependencyGraph::default(),
            decisions: Vec::new(),
            embeddings: None,
            history: ContextHistory { events: Vec::new(), max_events: 1000 },
        }
    }
}

#[derive(Debug)]
pub struct ContextStorage {
    base_path: PathBuf,
    context_dir: PathBuf,
}

impl ContextStorage {
    pub fn new(project_root: &Path) -> anyhow::Result<Self> {
        let context_dir = project_root.join(".auto-dev").join("context");

        Ok(Self { base_path: project_root.to_path_buf(), context_dir })
    }

    pub async fn load_or_create(&self) -> anyhow::Result<ProjectContext> {
        // Ensure directory exists
        fs::create_dir_all(&self.context_dir).await?;

        let context_file = self.context_dir.join("project.json");

        if context_file.exists() {
            self.load().await
        } else {
            let mut context = ProjectContext::default();

            // Try to infer project name from directory
            if let Some(name) = self.base_path.file_name() {
                context.metadata.name = name.to_string_lossy().to_string();
            }

            // Detect languages and build systems
            self.detect_project_metadata(&mut context).await?;

            Ok(context)
        }
    }

    pub async fn load(&self) -> anyhow::Result<ProjectContext> {
        let context_file = self.context_dir.join("project.json");
        let content = fs::read_to_string(&context_file).await?;
        let mut context: ProjectContext = serde_json::from_str(&content)?;

        // Load additional data files
        self.load_patterns(&mut context).await?;
        self.load_conventions(&mut context).await?;
        self.load_dependencies(&mut context).await?;
        self.load_decisions(&mut context).await?;

        Ok(context)
    }

    pub async fn save(&self, context: &ProjectContext) -> anyhow::Result<()> {
        // Ensure directory exists
        fs::create_dir_all(&self.context_dir).await?;

        // Save main context file
        let context_file = self.context_dir.join("project.json");
        let content = serde_json::to_string_pretty(&context)?;
        fs::write(&context_file, content).await?;

        // Save additional data files
        self.save_patterns(&context.patterns).await?;
        self.save_conventions(&context.conventions).await?;
        self.save_dependencies(&context.dependencies).await?;
        self.save_decisions(&context.decisions).await?;

        Ok(())
    }

    async fn detect_project_metadata(&self, context: &mut ProjectContext) -> anyhow::Result<()> {
        // Check for Cargo.toml
        if self.base_path.join("Cargo.toml").exists() {
            context.metadata.languages.push(Language::Rust);
            context.metadata.build_systems.push(BuildSystem::Cargo);
        }

        // Check for package.json
        if self.base_path.join("package.json").exists() {
            let package_json = self.base_path.join("package.json");
            if let Ok(content) = fs::read_to_string(&package_json).await {
                if content.contains("\"typescript\"") {
                    context.metadata.languages.push(Language::TypeScript);
                } else {
                    context.metadata.languages.push(Language::JavaScript);
                }

                if content.contains("\"yarn\"") {
                    context.metadata.build_systems.push(BuildSystem::Yarn);
                } else if content.contains("\"pnpm\"") {
                    context.metadata.build_systems.push(BuildSystem::Pnpm);
                } else {
                    context.metadata.build_systems.push(BuildSystem::Npm);
                }
            }
        }

        // Check for go.mod
        if self.base_path.join("go.mod").exists() {
            context.metadata.languages.push(Language::Go);
        }

        // Check for pom.xml
        if self.base_path.join("pom.xml").exists() {
            context.metadata.languages.push(Language::Java);
            context.metadata.build_systems.push(BuildSystem::Maven);
        }

        // Check for build.gradle
        if self.base_path.join("build.gradle").exists()
            || self.base_path.join("build.gradle.kts").exists()
        {
            context.metadata.languages.push(Language::Java);
            context.metadata.build_systems.push(BuildSystem::Gradle);
        }

        // Check for requirements.txt or setup.py
        if self.base_path.join("requirements.txt").exists()
            || self.base_path.join("setup.py").exists()
            || self.base_path.join("pyproject.toml").exists()
        {
            context.metadata.languages.push(Language::Python);
        }

        // Check for CMakeLists.txt
        if self.base_path.join("CMakeLists.txt").exists() {
            context.metadata.build_systems.push(BuildSystem::CMake);
            context.metadata.languages.push(Language::Cpp);
        }

        // Check for Makefile
        if self.base_path.join("Makefile").exists() {
            context.metadata.build_systems.push(BuildSystem::Make);
        }

        Ok(())
    }

    async fn load_patterns(&self, context: &mut ProjectContext) -> anyhow::Result<()> {
        let patterns_file = self.context_dir.join("patterns.json");
        if patterns_file.exists() {
            let content = fs::read_to_string(&patterns_file).await?;
            context.patterns = serde_json::from_str(&content)?;
        }
        Ok(())
    }

    async fn save_patterns(&self, patterns: &[CodePattern]) -> anyhow::Result<()> {
        let patterns_file = self.context_dir.join("patterns.json");
        let content = serde_json::to_string_pretty(patterns)?;
        fs::write(&patterns_file, content).await?;
        Ok(())
    }

    async fn load_conventions(&self, context: &mut ProjectContext) -> anyhow::Result<()> {
        let conventions_file = self.context_dir.join("conventions.json");
        if conventions_file.exists() {
            let content = fs::read_to_string(&conventions_file).await?;
            context.conventions = serde_json::from_str(&content)?;
        }
        Ok(())
    }

    async fn save_conventions(&self, conventions: &CodingConventions) -> anyhow::Result<()> {
        let conventions_file = self.context_dir.join("conventions.json");
        let content = serde_json::to_string_pretty(conventions)?;
        fs::write(&conventions_file, content).await?;
        Ok(())
    }

    async fn load_dependencies(&self, context: &mut ProjectContext) -> anyhow::Result<()> {
        let deps_file = self.context_dir.join("dependencies.json");
        if deps_file.exists() {
            let content = fs::read_to_string(&deps_file).await?;
            context.dependencies = serde_json::from_str(&content)?;
        }
        Ok(())
    }

    async fn save_dependencies(&self, deps: &DependencyGraph) -> anyhow::Result<()> {
        let deps_file = self.context_dir.join("dependencies.json");
        let content = serde_json::to_string_pretty(deps)?;
        fs::write(&deps_file, content).await?;
        Ok(())
    }

    async fn load_decisions(&self, context: &mut ProjectContext) -> anyhow::Result<()> {
        let decisions_dir = self.context_dir.join("decisions");
        if decisions_dir.exists() {
            let mut decisions = Vec::new();

            let mut entries = fs::read_dir(&decisions_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    let content = fs::read_to_string(entry.path()).await?;
                    if let Ok(decision) = serde_json::from_str::<ArchitectureDecision>(&content) {
                        decisions.push(decision);
                    }
                }
            }

            context.decisions = decisions;
        }
        Ok(())
    }

    async fn save_decisions(&self, decisions: &[ArchitectureDecision]) -> anyhow::Result<()> {
        let decisions_dir = self.context_dir.join("decisions");
        fs::create_dir_all(&decisions_dir).await?;

        for decision in decisions {
            let file_name = format!("{}.json", decision.id);
            let file_path = decisions_dir.join(file_name);
            let content = serde_json::to_string_pretty(decision)?;
            fs::write(&file_path, content).await?;
        }

        Ok(())
    }

    pub fn context_path(&self) -> &Path {
        &self.context_dir
    }
}
