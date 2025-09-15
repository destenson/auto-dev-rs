use std::collections::HashMap;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use anyhow::Result;

use crate::learning::pattern_extractor::{Pattern, PatternContext};
use crate::learning::failure_analyzer::AntiPattern;
use crate::parser::model::Specification;
use crate::incremental::Implementation;

pub type PatternId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBase {
    pub patterns: HashMap<PatternId, Pattern>,
    pub anti_patterns: HashMap<PatternId, AntiPattern>,
    #[serde(skip)]
    pub embedding_index: Option<()>,
    pub usage_stats: UsageStatistics,
    pub metadata: KnowledgeMetadata,
    pub storage_path: PathBuf,
}

impl KnowledgeBase {
    pub fn new(storage_path: PathBuf) -> Self {
        Self {
            patterns: HashMap::new(),
            anti_patterns: HashMap::new(),
            embedding_index: None,
            usage_stats: UsageStatistics::default(),
            metadata: KnowledgeMetadata::default(),
            storage_path,
        }
    }

    pub fn add_pattern(&mut self, mut pattern: Pattern) -> Result<PatternId> {
        let pattern_id = pattern.id;
        
        if pattern.embeddings.is_none() {
            pattern.embeddings = Some(self.generate_embeddings(&pattern.implementation)?);
        }
        
        // TODO: Implement embedding index when needed
        
        self.patterns.insert(pattern_id, pattern.clone());
        self.usage_stats.total_patterns += 1;
        self.metadata.last_updated = Utc::now();
        
        tracing::debug!("Added pattern {} to knowledge base", pattern.name);
        
        Ok(pattern_id)
    }

    pub fn add_anti_pattern(&mut self, anti_pattern: AntiPattern) -> Result<PatternId> {
        let anti_pattern_id = anti_pattern.id;
        
        self.anti_patterns.insert(anti_pattern_id, anti_pattern);
        self.usage_stats.total_anti_patterns += 1;
        self.metadata.last_updated = Utc::now();
        
        tracing::debug!("Added anti-pattern to knowledge base");
        
        Ok(anti_pattern_id)
    }

    pub fn find_similar_patterns(&self, spec: &Specification, limit: usize) -> Vec<Pattern> {
        let query_embedding = match self.generate_query_embedding(spec) {
            Ok(embedding) => embedding,
            Err(_) => return Vec::new(),
        };

        // TODO: Implement semantic search when embedding index is available
        // For now, return all patterns that match the context
        self.patterns
            .values()
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn find_matching_pattern(&self, implementation: &Implementation) -> Option<PatternId> {
        let impl_hash = hash_implementation(implementation);
        
        self.patterns
            .iter()
            .find(|(_, pattern)| {
                hash_code(&pattern.implementation) == impl_hash
            })
            .map(|(id, _)| *id)
    }

    pub fn apply_pattern(&self, pattern: &Pattern, spec: &Specification) -> Option<Implementation> {
        use crate::incremental::{FileChange, ChangeType};
        use std::path::PathBuf;
        
        let adapted_code = self.adapt_pattern_to_spec(&pattern.implementation, spec);
        
        let implementation = Implementation {
            files: vec![FileChange {
                path: PathBuf::from("generated.rs"),
                change_type: ChangeType::Create,
                content: adapted_code,
                line_range: None,
            }],
            estimated_complexity: crate::incremental::Complexity::Simple,
            approach: "Pattern-based generation".to_string(),
            language: pattern.context.language.clone(),
        };
        
        self.track_pattern_usage(pattern.id);
        
        Some(implementation)
    }

    pub fn reinforce_pattern(&mut self, pattern_id: PatternId) -> Result<()> {
        if let Some(pattern) = self.patterns.get_mut(&pattern_id) {
            pattern.usage_count += 1;
            pattern.success_rate = (pattern.success_rate * 0.95 + 1.0 * 0.05).min(1.0);
            
            self.usage_stats.pattern_hits += 1;
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Pattern not found"))
        }
    }

    pub fn get_pattern(&self, pattern_id: &PatternId) -> Option<&Pattern> {
        self.patterns.get(pattern_id)
    }

    pub fn get_anti_pattern(&self, anti_pattern_id: &PatternId) -> Option<&AntiPattern> {
        self.anti_patterns.get(anti_pattern_id)
    }

    pub fn search_patterns(&self, query: &str) -> Vec<Pattern> {
        self.patterns
            .values()
            .filter(|p| {
                p.name.contains(query) || 
                p.description.contains(query) ||
                p.tags.iter().any(|t| t.contains(query))
            })
            .cloned()
            .collect()
    }

    pub fn get_patterns_by_context(&self, context: &PatternContext) -> Vec<Pattern> {
        self.patterns
            .values()
            .filter(|p| p.context.matches(context))
            .cloned()
            .collect()
    }

    pub fn get_top_patterns(&self, limit: usize) -> Vec<Pattern> {
        let mut patterns: Vec<_> = self.patterns.values().cloned().collect();
        patterns.sort_by(|a, b| {
            let a_score = a.quality_score() * a.usage_count as f32;
            let b_score = b.quality_score() * b.usage_count as f32;
            b_score.partial_cmp(&a_score).unwrap()
        });
        patterns.truncate(limit);
        patterns
    }

    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    pub fn anti_pattern_count(&self) -> usize {
        self.anti_patterns.len()
    }

    pub fn size(&self) -> usize {
        self.patterns.len() + self.anti_patterns.len()
    }

    pub fn get_usage_stats(&self) -> &UsageStatistics {
        &self.usage_stats
    }

    pub fn export(&self) -> Result<KnowledgeExport> {
        Ok(KnowledgeExport {
            version: env!("CARGO_PKG_VERSION").to_string(),
            patterns: self.patterns.values().cloned().collect(),
            anti_patterns: self.anti_patterns.values().cloned().collect(),
            statistics: self.usage_stats.clone(),
            metadata: self.metadata.clone(),
            exported_at: Utc::now(),
        })
    }

    pub fn import(&mut self, export: KnowledgeExport) -> Result<()> {
        self.validate_version(&export.version)?;
        
        let pattern_count = export.patterns.len();
        let anti_pattern_count = export.anti_patterns.len();
        
        for pattern in export.patterns {
            self.merge_pattern(pattern)?;
        }
        
        for anti_pattern in export.anti_patterns {
            self.merge_anti_pattern(anti_pattern)?;
        }
        
        self.usage_stats.merge(&export.statistics);
        self.metadata.last_import = Some(Utc::now());
        
        self.rebuild_index()?;
        
        tracing::info!(
            "Imported {} patterns and {} anti-patterns",
            pattern_count,
            anti_pattern_count
        );
        
        Ok(())
    }

    fn validate_version(&self, version: &str) -> Result<()> {
        let current_version = env!("CARGO_PKG_VERSION");
        
        if !version_compatible(current_version, version) {
            return Err(anyhow::anyhow!(
                "Incompatible knowledge base version: {} (current: {})",
                version,
                current_version
            ));
        }
        
        Ok(())
    }

    fn merge_pattern(&mut self, pattern: Pattern) -> Result<()> {
        if let Some(existing) = self.patterns.get_mut(&pattern.id) {
            existing.usage_count += pattern.usage_count;
            existing.success_rate = (existing.success_rate + pattern.success_rate) / 2.0;
            
            if pattern.learned_at > existing.learned_at {
                existing.implementation = pattern.implementation;
                existing.embeddings = pattern.embeddings;
            }
        } else {
            self.patterns.insert(pattern.id, pattern);
        }
        
        Ok(())
    }

    fn merge_anti_pattern(&mut self, anti_pattern: AntiPattern) -> Result<()> {
        if let Some(existing) = self.anti_patterns.get_mut(&anti_pattern.id) {
            existing.occurrences += anti_pattern.occurrences;
        } else {
            self.anti_patterns.insert(anti_pattern.id, anti_pattern);
        }
        
        Ok(())
    }

    fn rebuild_index(&mut self) -> Result<()> {
        self.embedding_index = None;
        
        // TODO: Rebuild embedding index when needed
        
        Ok(())
    }

    fn generate_embeddings(&self, _text: &str) -> Result<Vec<f32>> {
        Ok(vec![0.0; 384])
    }

    fn generate_query_embedding(&self, spec: &Specification) -> Result<Vec<f32>> {
        let spec_text = format!("{:?}", spec);
        self.generate_embeddings(&spec_text)
    }

    fn adapt_pattern_to_spec(&self, pattern_code: &str, _spec: &Specification) -> String {
        pattern_code.to_string()
    }

    fn track_pattern_usage(&self, pattern_id: PatternId) {
        tracing::debug!("Pattern {} used", pattern_id);
    }

    pub async fn persist(&self) -> Result<()> {
        let export = self.export()?;
        let export_path = self.storage_path.join("knowledge_base.json");
        
        std::fs::create_dir_all(&self.storage_path)?;
        let json = serde_json::to_string_pretty(&export)?;
        tokio::fs::write(export_path, json).await?;
        
        Ok(())
    }

    pub async fn load(&mut self) -> Result<()> {
        let import_path = self.storage_path.join("knowledge_base.json");
        
        if !import_path.exists() {
            return Ok(());
        }
        
        let json = tokio::fs::read_to_string(import_path).await?;
        let export: KnowledgeExport = serde_json::from_str(&json)?;
        
        self.import(export)?;
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeExport {
    pub version: String,
    pub patterns: Vec<Pattern>,
    pub anti_patterns: Vec<AntiPattern>,
    pub statistics: UsageStatistics,
    pub metadata: KnowledgeMetadata,
    pub exported_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageStatistics {
    pub total_patterns: usize,
    pub total_anti_patterns: usize,
    pub total_uses: u32,
    pub pattern_hits: u32,
    pub cache_hits: u32,
    pub cache_misses: u32,
}

impl UsageStatistics {
    fn merge(&mut self, other: &UsageStatistics) {
        self.total_patterns = self.total_patterns.max(other.total_patterns);
        self.total_anti_patterns = self.total_anti_patterns.max(other.total_anti_patterns);
        self.total_uses += other.total_uses;
        self.pattern_hits += other.pattern_hits;
        self.cache_hits += other.cache_hits;
        self.cache_misses += other.cache_misses;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeMetadata {
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub last_export: Option<DateTime<Utc>>,
    pub last_import: Option<DateTime<Utc>>,
    pub version: String,
}

impl Default for KnowledgeMetadata {
    fn default() -> Self {
        Self {
            created_at: Utc::now(),
            last_updated: Utc::now(),
            last_export: None,
            last_import: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

fn hash_implementation(implementation: &Implementation) -> u64 {
    let code = implementation.files.iter()
        .map(|f| f.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    hash_code(&code)
}

fn hash_code(code: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    code.hash(&mut hasher);
    hasher.finish()
}

fn version_compatible(current: &str, other: &str) -> bool {
    let current_parts: Vec<_> = current.split('.').collect();
    let other_parts: Vec<_> = other.split('.').collect();
    
    if current_parts.is_empty() || other_parts.is_empty() {
        return false;
    }
    
    current_parts[0] == other_parts[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_base_creation() {
        let kb = KnowledgeBase::new(PathBuf::from("/tmp/test"));
        assert_eq!(kb.pattern_count(), 0);
        assert_eq!(kb.anti_pattern_count(), 0);
    }

    #[test]
    fn test_add_pattern() {
        let mut kb = KnowledgeBase::new(PathBuf::from("/tmp/test"));
        
        let pattern = Pattern {
            id: Uuid::new_v4(),
            name: "test_pattern".to_string(),
            description: "A test pattern".to_string(),
            pattern_type: crate::learning::pattern_extractor::PatternType::Structural,
            context: PatternContext::default(),
            implementation: "fn test() {}".to_string(),
            success_rate: 0.9,
            usage_count: 0,
            learned_at: Utc::now(),
            embeddings: None,
            tags: vec!["test".to_string()],
            complexity: 1,
            reusability_score: 0.8,
            test_coverage: 0.7,
        };
        
        let result = kb.add_pattern(pattern);
        assert!(result.is_ok());
        assert_eq!(kb.pattern_count(), 1);
    }

    #[test]
    fn test_search_patterns() {
        let mut kb = KnowledgeBase::new(PathBuf::from("/tmp/test"));
        
        let pattern = Pattern {
            id: Uuid::new_v4(),
            name: "error_handler".to_string(),
            description: "Error handling pattern".to_string(),
            pattern_type: crate::learning::pattern_extractor::PatternType::Behavioral,
            context: PatternContext::default(),
            implementation: "Result<T, E>".to_string(),
            success_rate: 0.95,
            usage_count: 10,
            learned_at: Utc::now(),
            embeddings: None,
            tags: vec!["error".to_string(), "result".to_string()],
            complexity: 2,
            reusability_score: 0.9,
            test_coverage: 0.8,
        };
        
        kb.add_pattern(pattern).unwrap();
        
        let results = kb.search_patterns("error");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "error_handler");
    }

    #[test]
    fn test_export_import() {
        let mut kb1 = KnowledgeBase::new(PathBuf::from("/tmp/test1"));
        
        let pattern = Pattern {
            id: Uuid::new_v4(),
            name: "test_pattern".to_string(),
            description: "Test".to_string(),
            pattern_type: crate::learning::pattern_extractor::PatternType::Structural,
            context: PatternContext::default(),
            implementation: "test".to_string(),
            success_rate: 1.0,
            usage_count: 5,
            learned_at: Utc::now(),
            embeddings: None,
            tags: vec![],
            complexity: 1,
            reusability_score: 0.5,
            test_coverage: 0.5,
        };
        
        kb1.add_pattern(pattern).unwrap();
        
        let export = kb1.export().unwrap();
        
        let mut kb2 = KnowledgeBase::new(PathBuf::from("/tmp/test2"));
        kb2.import(export).unwrap();
        
        assert_eq!(kb2.pattern_count(), 1);
    }
}