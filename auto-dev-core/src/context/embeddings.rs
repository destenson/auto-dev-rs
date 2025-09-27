#![allow(unused)]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::context::manager::{CodeExample, SimilarCode};

// Vector type for embeddings
pub type Vector = Vec<f32>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingEntry {
    pub id: String,
    pub path: PathBuf,
    pub content_hash: String,
    pub embedding: Vector,
    pub metadata: EmbeddingMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingMetadata {
    pub language: Option<String>,
    pub file_type: String,
    pub size: usize,
    pub last_modified: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct EmbeddingStore {
    entries: Arc<RwLock<HashMap<PathBuf, EmbeddingEntry>>>,
    index: Arc<RwLock<VectorIndex>>,
    storage_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct VectorIndex {
    vectors: Vec<(String, Vector)>,
    dimension: usize,
}

impl EmbeddingStore {
    pub async fn new(project_root: &Path) -> anyhow::Result<Self> {
        let storage_path = project_root.join(".auto-dev").join("context").join("embeddings.db");
        let entries = Arc::new(RwLock::new(HashMap::new()));
        let index = Arc::new(RwLock::new(VectorIndex::new(384))); // Default dimension for small models

        let store = Self { entries, index, storage_path };

        // Load existing embeddings if available
        store.load_embeddings().await?;

        Ok(store)
    }

    pub async fn add_code(&self, path: &PathBuf, content: &str) -> anyhow::Result<()> {
        let embedding = self.generate_embedding(content).await?;
        let content_hash = self.hash_content(content);

        let entry = EmbeddingEntry {
            id: path.to_string_lossy().to_string(),
            path: path.clone(),
            content_hash,
            embedding: embedding.clone(),
            metadata: EmbeddingMetadata {
                language: detect_language_from_path(path),
                file_type: path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
                size: content.len(),
                last_modified: chrono::Utc::now(),
            },
        };

        // Update entries
        let mut entries = self.entries.write().await;
        entries.insert(path.clone(), entry);

        // Update index
        let mut index = self.index.write().await;
        index.add(path.to_string_lossy().to_string(), embedding);

        // Persist to disk
        self.save_embeddings().await?;

        Ok(())
    }

    pub async fn update_code(&self, path: &PathBuf, content: &str) -> anyhow::Result<()> {
        // Remove old embedding
        self.remove_code(path).await?;

        // Add new embedding
        self.add_code(path, content).await?;

        Ok(())
    }

    pub async fn remove_code(&self, path: &PathBuf) -> anyhow::Result<()> {
        let mut entries = self.entries.write().await;
        entries.remove(path);

        // Update index
        let mut index = self.index.write().await;
        index.remove(&path.to_string_lossy().to_string());

        // Persist changes
        self.save_embeddings().await?;

        Ok(())
    }

    pub async fn find_similar(&self, query: &str, k: usize) -> anyhow::Result<Vec<SimilarCode>> {
        let query_embedding = self.generate_embedding(query).await?;

        let index = self.index.read().await;
        let similar_ids = index.search(&query_embedding, k);

        let entries = self.entries.read().await;
        let mut results = Vec::new();

        for (id, similarity) in similar_ids {
            // Find the entry with this ID
            if let Some(entry) = entries.values().find(|e| e.id == id) {
                // Try to load the actual code snippet
                if let Ok(content) = tokio::fs::read_to_string(&entry.path).await {
                    let lines: Vec<&str> = content.lines().collect();
                    let snippet =
                        if lines.len() > 10 { lines[..10].join("\n") } else { content.clone() };

                    results.push(SimilarCode {
                        example: CodeExample {
                            file_path: entry.path.clone(),
                            line_start: 1,
                            line_end: lines.len().min(10),
                            code: snippet,
                            description: Some(format!(
                                "Similar code from {}",
                                entry.path.display()
                            )),
                        },
                        similarity,
                    });
                }
            }
        }

        Ok(results)
    }

    async fn generate_embedding(&self, text: &str) -> anyhow::Result<Vector> {
        // In a real implementation, this would use a local embedding model
        // For now, we'll create a simple hash-based embedding

        // This is a placeholder implementation
        // In production, you would use a model like:
        // - sentence-transformers/all-MiniLM-L6-v2
        // - BERT-based models
        // - CodeBERT for code-specific embeddings

        let mut embedding = vec![0.0f32; 384];

        // Simple hash-based embedding (placeholder)
        let bytes = text.as_bytes();
        for (i, byte) in bytes.iter().enumerate() {
            let idx = i % 384;
            embedding[idx] += (*byte as f32) / 255.0;
        }

        // Normalize
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut embedding {
                *val /= norm;
            }
        }

        Ok(embedding)
    }

    fn hash_content(&self, content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    async fn load_embeddings(&self) -> anyhow::Result<()> {
        if !self.storage_path.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&self.storage_path).await?;
        let stored_entries: Vec<EmbeddingEntry> = serde_json::from_str(&content)?;

        let mut entries = self.entries.write().await;
        let mut index = self.index.write().await;

        for entry in stored_entries {
            let id = entry.id.clone();
            let embedding = entry.embedding.clone();
            entries.insert(entry.path.clone(), entry);
            index.add(id, embedding);
        }

        Ok(())
    }

    async fn save_embeddings(&self) -> anyhow::Result<()> {
        // Ensure directory exists
        if let Some(parent) = self.storage_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let entries = self.entries.read().await;
        let entries_vec: Vec<EmbeddingEntry> = entries.values().cloned().collect();

        let content = serde_json::to_string_pretty(&entries_vec)?;
        tokio::fs::write(&self.storage_path, content).await?;

        Ok(())
    }
}

impl VectorIndex {
    fn new(dimension: usize) -> Self {
        Self { vectors: Vec::new(), dimension }
    }

    fn add(&mut self, id: String, vector: Vector) {
        // Remove old entry if exists
        self.vectors.retain(|(existing_id, _)| existing_id != &id);

        // Add new entry
        self.vectors.push((id, vector));
    }

    fn remove(&mut self, id: &str) {
        self.vectors.retain(|(existing_id, _)| existing_id != id);
    }

    fn search(&self, query: &Vector, k: usize) -> Vec<(String, f32)> {
        let mut similarities: Vec<(String, f32)> = self
            .vectors
            .iter()
            .map(|(id, vector)| {
                let similarity = cosine_similarity(query, vector);
                (id.clone(), similarity)
            })
            .collect();

        // Sort by similarity (descending)
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return top k
        similarities.into_iter().take(k).collect()
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a > 0.0 && norm_b > 0.0 { dot_product / (norm_a * norm_b) } else { 0.0 }
}

fn detect_language_from_path(path: &Path) -> Option<String> {
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

// Placeholder for actual embedding model integration
// In production, you would integrate with:
// - Candle for local model inference
// - ONNX Runtime for cross-platform model support
// - Hugging Face transformers
pub struct EmbeddingModel {
    model_path: PathBuf,
}

impl EmbeddingModel {
    pub fn new(model_path: PathBuf) -> Self {
        Self { model_path }
    }

    pub async fn embed(&self, _text: &str) -> anyhow::Result<Vector> {
        // Placeholder for actual model inference
        // This would load and run a local embedding model
        Ok(vec![0.0; 384])
    }
}
