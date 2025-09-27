use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::context::analyzer::{CodePattern, CodingConventions, ProjectAnalyzer};
use crate::context::embeddings::EmbeddingStore;
use crate::context::query::ContextQuery;
use crate::context::storage::{ContextStorage, ProjectContext};
use crate::{debug, info};

#[derive(Debug, Clone)]
pub struct ContextManager {
    context: Arc<RwLock<ProjectContext>>,
    storage: Arc<ContextStorage>,
    analyzer: Arc<ProjectAnalyzer>,
    embeddings: Arc<EmbeddingStore>,
    project_root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextUpdate {
    FileAdded(PathBuf),
    FileModified(PathBuf),
    FileDeleted(PathBuf),
    PatternDetected(CodePattern),
    DecisionMade(ArchitectureDecision),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureDecision {
    pub id: String,
    pub title: String,
    pub description: String,
    pub chosen_option: String,
    pub alternatives: Vec<String>,
    pub rationale: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExample {
    pub file_path: PathBuf,
    pub line_start: usize,
    pub line_end: usize,
    pub code: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarCode {
    pub example: CodeExample,
    pub similarity: f32,
}

impl ContextManager {
    pub async fn new(project_root: PathBuf) -> anyhow::Result<Self> {
        let storage = Arc::new(ContextStorage::new(&project_root)?);
        let context = Arc::new(RwLock::new(storage.load_or_create().await?));
        let analyzer = Arc::new(ProjectAnalyzer::new(project_root.clone()));
        let embeddings = Arc::new(EmbeddingStore::new(&project_root).await?);

        Ok(Self { context, storage, analyzer, embeddings, project_root })
    }

    pub async fn initialize(&self) -> anyhow::Result<()> {
        info!("Initializing project context for: {:?}", self.project_root);

        // Analyze project structure
        let structure = self.analyzer.analyze_structure().await?;

        // Detect patterns
        let patterns = self.analyzer.detect_patterns().await?;

        // Infer conventions
        let conventions = self.analyzer.infer_conventions().await?;

        // Build dependency graph
        let dependencies = self.analyzer.analyze_dependencies().await?;

        // Update context
        let mut ctx = self.context.write().await;
        ctx.structure = structure;
        ctx.patterns = patterns;
        ctx.conventions = conventions;
        ctx.dependencies = dependencies;
        ctx.metadata.last_updated = Utc::now();

        // Save to disk
        self.storage.save(&ctx).await?;

        info!("Project context initialized successfully");
        Ok(())
    }

    pub async fn update(&self, update: ContextUpdate) -> anyhow::Result<()> {
        debug!("Processing context update: {:?}", update);

        match update {
            ContextUpdate::FileAdded(path) => {
                self.handle_file_added(path).await?;
            }
            ContextUpdate::FileModified(path) => {
                self.handle_file_modified(path).await?;
            }
            ContextUpdate::FileDeleted(path) => {
                self.handle_file_deleted(path).await?;
            }
            ContextUpdate::PatternDetected(pattern) => {
                self.handle_pattern_detected(pattern).await?;
            }
            ContextUpdate::DecisionMade(decision) => {
                self.handle_decision_made(decision).await?;
            }
        }

        // Update timestamp
        let mut ctx = self.context.write().await;
        ctx.metadata.last_updated = Utc::now();

        // Persist changes
        self.storage.save(&ctx).await?;

        Ok(())
    }

    async fn handle_file_added(&self, path: PathBuf) -> anyhow::Result<()> {
        // Analyze new file
        let patterns = self.analyzer.analyze_file(&path).await?;

        // Update embeddings
        if let Ok(content) = tokio::fs::read_to_string(&path).await {
            self.embeddings.add_code(&path, &content).await?;
        }

        // Update context
        let mut ctx = self.context.write().await;
        for pattern in patterns {
            if !ctx.patterns.iter().any(|p| p.name == pattern.name) {
                ctx.patterns.push(pattern);
            }
        }

        Ok(())
    }

    async fn handle_file_modified(&self, path: PathBuf) -> anyhow::Result<()> {
        // Re-analyze file
        let patterns = self.analyzer.analyze_file(&path).await?;

        // Update embeddings
        if let Ok(content) = tokio::fs::read_to_string(&path).await {
            self.embeddings.update_code(&path, &content).await?;
        }

        // Update patterns
        let mut ctx = self.context.write().await;
        ctx.patterns.retain(|p| !p.locations.contains(&path));
        ctx.patterns.extend(patterns);

        Ok(())
    }

    async fn handle_file_deleted(&self, path: PathBuf) -> anyhow::Result<()> {
        // Remove from embeddings
        self.embeddings.remove_code(&path).await?;

        // Update context
        let mut ctx = self.context.write().await;
        ctx.patterns.retain(|p| !p.locations.contains(&path));

        Ok(())
    }

    async fn handle_pattern_detected(&self, pattern: CodePattern) -> anyhow::Result<()> {
        let mut ctx = self.context.write().await;

        // Check if pattern already exists
        if let Some(existing) = ctx.patterns.iter_mut().find(|p| p.name == pattern.name) {
            // Update frequency and locations
            existing.frequency = (existing.frequency + pattern.frequency) / 2.0;
            existing.locations.extend(pattern.locations);
            existing.examples.extend(pattern.examples);
        } else {
            ctx.patterns.push(pattern);
        }

        Ok(())
    }

    async fn handle_decision_made(&self, decision: ArchitectureDecision) -> anyhow::Result<()> {
        let mut ctx = self.context.write().await;
        ctx.decisions.push(decision);
        Ok(())
    }

    // Query methods
    pub async fn find_similar_code(&self, spec: &str) -> anyhow::Result<Vec<SimilarCode>> {
        self.embeddings.find_similar(spec, 5).await
    }

    pub async fn get_patterns_for(&self, file_type: &str) -> Vec<CodePattern> {
        let ctx = self.context.read().await;
        ctx.patterns
            .iter()
            .filter(|p| {
                p.locations.iter().any(|path| {
                    path.extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext == file_type)
                        .unwrap_or(false)
                })
            })
            .cloned()
            .collect()
    }

    pub async fn get_conventions(&self) -> CodingConventions {
        let ctx = self.context.read().await;
        ctx.conventions.clone()
    }

    pub async fn get_decisions_for(&self, component: &str) -> Vec<ArchitectureDecision> {
        let ctx = self.context.read().await;
        ctx.decisions
            .iter()
            .filter(|d| d.title.contains(component) || d.description.contains(component))
            .cloned()
            .collect()
    }

    pub async fn export_context(&self, format: &str) -> anyhow::Result<String> {
        let ctx = self.context.read().await;

        match format {
            "json" => Ok(serde_json::to_string_pretty(&*ctx)?),
            "toml" => Ok(toml::to_string_pretty(&*ctx)?),
            _ => anyhow::bail!("Unsupported export format: {}", format),
        }
    }

    pub async fn get_context_query(&self) -> ContextQuery {
        let ctx = self.context.read().await;
        ContextQuery::new(ctx.clone())
    }
}
