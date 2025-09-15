use anyhow::Result;
use std::collections::HashMap;
use uuid::Uuid;

pub type Vector = Vec<f32>;

#[derive(Debug, Clone)]
pub struct EmbeddingGenerator {
    dimension: usize,
}

impl EmbeddingGenerator {
    pub async fn new() -> Result<Self> {
        Ok(Self { dimension: 384 })
    }

    pub fn new_sync() -> Self {
        Self { dimension: 384 }
    }

    pub fn generate(&self, text: &str) -> Result<Vector> {
        let mut embedding = vec![0.0f32; self.dimension];

        let bytes = text.as_bytes();
        for (i, byte) in bytes.iter().enumerate() {
            let idx = i % self.dimension;
            embedding[idx] += (*byte as f32) / 255.0;
        }

        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut embedding {
                *val /= norm;
            }
        }

        Ok(embedding)
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }
}

#[derive(Debug, Clone)]
pub struct EmbeddingIndex {
    pub embeddings: Vec<(String, Uuid, Vector)>,
    pub pattern_map: HashMap<String, Uuid>,
}

impl EmbeddingIndex {
    pub fn new() -> Self {
        Self { embeddings: Vec::new(), pattern_map: HashMap::new() }
    }

    pub fn insert(&mut self, embeddings: &[f32], key: String, pattern_id: Uuid) {
        if embeddings.len() == 384 {
            self.embeddings.push((key.clone(), pattern_id, embeddings.to_vec()));
            self.pattern_map.insert(key, pattern_id);
        }
    }

    pub fn search(&self, query: &[f32], limit: usize) -> Vec<(f32, String, Uuid)> {
        if query.len() != 384 {
            return Vec::new();
        }

        let mut results: Vec<(f32, String, Uuid)> = self
            .embeddings
            .iter()
            .map(|(key, id, emb)| {
                let similarity = 1.0 - cosine_distance(query, emb);
                (similarity, key.clone(), *id)
            })
            .collect();

        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    pub fn remove(&mut self, key: &str) {
        self.pattern_map.remove(key);
        self.embeddings.retain(|(k, _, _)| k != key);
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a > 0.0 && norm_b > 0.0 { dot_product / (norm_a * norm_b) } else { 0.0 }
}

pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    1.0 - cosine_similarity(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_generation() {
        let generator = EmbeddingGenerator::new_sync();
        let text = "fn main() { println!(\"Hello, world!\"); }";
        let embedding = generator.generate(text).unwrap();

        assert_eq!(embedding.len(), 384);

        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_embedding_index() {
        let mut index = EmbeddingIndex::new();
        let generator = EmbeddingGenerator::new_sync();

        let embedding1 = generator.generate("test1").unwrap();
        let embedding2 = generator.generate("test2").unwrap();

        index.insert(&embedding1, "key1".to_string(), Uuid::new_v4());
        index.insert(&embedding2, "key2".to_string(), Uuid::new_v4());

        let query = generator.generate("test1").unwrap();
        let results = index.search(&query, 2);

        assert_eq!(results.len(), 2);
        assert!(results[0].0 > results[1].0);
    }
}
