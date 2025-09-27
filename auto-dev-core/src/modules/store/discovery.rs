//! Module Discovery for Local Store
//!
//! Provides search and discovery capabilities for modules in the local store.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::modules::store::manifest::{ModuleManifest, ManifestParser, TrustLevel};

/// Search index entry for a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Module ID
    pub id: String,
    /// Module name
    pub name: String,
    /// Module version
    pub version: String,
    /// Module description
    pub description: String,
    /// Module category
    pub category: String,
    /// Search keywords
    pub keywords: Vec<String>,
    /// Capabilities provided
    pub provides: Vec<String>,
    /// Trust level
    pub trust_level: TrustLevel,
    /// Manifest path
    pub manifest_path: PathBuf,
}

/// Module discovery service
pub struct ModuleDiscovery {
    /// Store root directory
    store_root: PathBuf,
    /// Search index
    index: HashMap<String, IndexEntry>,
    /// Index file path
    index_path: PathBuf,
    /// Inverted index for fast text search
    inverted_index: InvertedIndex,
}

/// Inverted index for text search
#[derive(Debug, Default)]
struct InvertedIndex {
    /// Map from terms to module IDs
    terms: HashMap<String, HashSet<String>>,
    /// Map from capabilities to module IDs
    capabilities: HashMap<String, HashSet<String>>,
    /// Map from categories to module IDs
    categories: HashMap<String, HashSet<String>>,
}

impl ModuleDiscovery {
    /// Create a new discovery service
    pub fn new(store_root: PathBuf) -> Self {
        let index_path = store_root.join("search.index");
        let (index, inverted_index) = if index_path.exists() {
            Self::load_index(&index_path).unwrap_or_default()
        } else {
            (HashMap::new(), InvertedIndex::default())
        };

        Self {
            store_root,
            index,
            index_path,
            inverted_index,
        }
    }

    /// Index a module for discovery
    pub fn index_module(&mut self, module_id: &str, manifest: &ModuleManifest) -> Result<()> {
        // Create index entry
        let mut keywords = manifest.module.keywords.clone().unwrap_or_default();
        keywords.push(manifest.module.name.clone());
        keywords.push(manifest.module.category.clone());
        
        let entry = IndexEntry {
            id: module_id.to_string(),
            name: manifest.module.name.clone(),
            version: manifest.module.version.clone(),
            description: manifest.module.description.clone(),
            category: manifest.module.category.clone(),
            keywords: keywords.clone(),
            provides: manifest.capabilities.provides.clone(),
            trust_level: manifest.verification
                .as_ref()
                .and_then(|v| v.trust_level.clone())
                .unwrap_or_default(),
            manifest_path: self.store_root.join(module_id).join("module.toml"),
        };

        // Add to main index
        self.index.insert(module_id.to_string(), entry.clone());

        // Update inverted index
        self.update_inverted_index(module_id, &entry);

        // Save index
        self.save_index()?;

        Ok(())
    }

    /// Search for modules
    pub fn search(&self, query: &str) -> Result<Vec<ModuleManifest>> {
        let query_lower = query.to_lowercase();
        let terms: Vec<&str> = query_lower.split_whitespace().collect();
        
        if terms.is_empty() {
            return Ok(Vec::new());
        }

        // Find matching module IDs
        let mut matching_ids = HashSet::new();
        
        for term in &terms {
            // Search in term index
            if let Some(ids) = self.inverted_index.terms.get(*term) {
                if matching_ids.is_empty() {
                    matching_ids = ids.clone();
                } else {
                    matching_ids = matching_ids.intersection(ids).cloned().collect();
                }
            }
            
            // Search in capabilities
            if let Some(ids) = self.inverted_index.capabilities.get(*term) {
                matching_ids.extend(ids.iter().cloned());
            }
        }

        // Also do substring matching on names and descriptions
        for (id, entry) in &self.index {
            if entry.name.to_lowercase().contains(&query_lower) ||
               entry.description.to_lowercase().contains(&query_lower) {
                matching_ids.insert(id.clone());
            }
        }

        // Load manifests for matching modules
        self.load_manifests(matching_ids)
    }

    /// List all modules
    pub fn list_all(&self) -> Result<Vec<ModuleManifest>> {
        let ids: HashSet<String> = self.index.keys().cloned().collect();
        self.load_manifests(ids)
    }

    /// List modules by category
    pub fn list_by_category(&self, category: &str) -> Result<Vec<ModuleManifest>> {
        let category_lower = category.to_lowercase();
        let ids = self.inverted_index.categories
            .get(&category_lower)
            .cloned()
            .unwrap_or_default();
        
        self.load_manifests(ids)
    }

    /// List modules by capability
    pub fn list_by_capability(&self, capability: &str) -> Result<Vec<ModuleManifest>> {
        let capability_lower = capability.to_lowercase();
        let ids = self.inverted_index.capabilities
            .get(&capability_lower)
            .cloned()
            .unwrap_or_default();
        
        self.load_manifests(ids)
    }

    /// List modules by trust level
    pub fn list_by_trust_level(&self, min_trust: TrustLevel) -> Result<Vec<ModuleManifest>> {
        let ids: HashSet<String> = self.index
            .iter()
            .filter(|(_, entry)| Self::trust_level_gte(&entry.trust_level, &min_trust))
            .map(|(id, _)| id.clone())
            .collect();
        
        self.load_manifests(ids)
    }

    /// Get recommended modules
    pub fn get_recommendations(&self, for_module: &str) -> Result<Vec<ModuleManifest>> {
        // Find the module
        let entry = self.index.get(for_module)
            .ok_or_else(|| anyhow::anyhow!("Module not found: {}", for_module))?;
        
        // Find similar modules based on category and keywords
        let mut scores: HashMap<String, f32> = HashMap::new();
        
        // Same category gives points
        if let Some(category_modules) = self.inverted_index.categories.get(&entry.category.to_lowercase()) {
            for id in category_modules {
                if id != for_module {
                    *scores.entry(id.clone()).or_default() += 1.0;
                }
            }
        }
        
        // Shared keywords give points
        for keyword in &entry.keywords {
            if let Some(keyword_modules) = self.inverted_index.terms.get(&keyword.to_lowercase()) {
                for id in keyword_modules {
                    if id != for_module {
                        *scores.entry(id.clone()).or_default() += 0.5;
                    }
                }
            }
        }
        
        // Sort by score and take top 10
        let mut sorted: Vec<(String, f32)> = scores.into_iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        sorted.truncate(10);
        
        let ids: HashSet<String> = sorted.into_iter().map(|(id, _)| id).collect();
        self.load_manifests(ids)
    }

    /// Remove module from index
    pub fn remove_from_index(&mut self, module_id: &str) -> Result<()> {
        if let Some(entry) = self.index.remove(module_id) {
            // Remove from inverted index
            self.remove_from_inverted_index(module_id, &entry);
            
            // Save index
            self.save_index()?;
        }
        
        Ok(())
    }

    /// Update inverted index with module entry
    fn update_inverted_index(&mut self, module_id: &str, entry: &IndexEntry) {
        // Index name and keywords
        let mut terms = HashSet::new();
        terms.insert(entry.name.to_lowercase());
        for keyword in &entry.keywords {
            terms.insert(keyword.to_lowercase());
        }
        
        // Split description into words
        for word in entry.description.split_whitespace() {
            let word = word.to_lowercase()
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_string();
            if !word.is_empty() && word.len() > 2 {
                terms.insert(word);
            }
        }
        
        for term in terms {
            self.inverted_index.terms
                .entry(term)
                .or_default()
                .insert(module_id.to_string());
        }
        
        // Index capabilities
        for capability in &entry.provides {
            self.inverted_index.capabilities
                .entry(capability.to_lowercase())
                .or_default()
                .insert(module_id.to_string());
        }
        
        // Index category
        self.inverted_index.categories
            .entry(entry.category.to_lowercase())
            .or_default()
            .insert(module_id.to_string());
    }

    /// Remove module from inverted index
    fn remove_from_inverted_index(&mut self, module_id: &str, entry: &IndexEntry) {
        // Remove from term index
        for term_set in self.inverted_index.terms.values_mut() {
            term_set.remove(module_id);
        }
        
        // Remove from capability index
        for cap_set in self.inverted_index.capabilities.values_mut() {
            cap_set.remove(module_id);
        }
        
        // Remove from category index
        if let Some(cat_set) = self.inverted_index.categories.get_mut(&entry.category.to_lowercase()) {
            cat_set.remove(module_id);
        }
    }

    /// Load manifests for given module IDs
    fn load_manifests(&self, ids: HashSet<String>) -> Result<Vec<ModuleManifest>> {
        let mut manifests = Vec::new();
        
        for id in ids {
            if let Some(entry) = self.index.get(&id) {
                if entry.manifest_path.exists() {
                    match ManifestParser::parse_from_path(&entry.manifest_path) {
                        Ok(manifest) => manifests.push(manifest),
                        Err(e) => eprintln!("Failed to load manifest for {}: {}", id, e),
                    }
                }
            }
        }
        
        Ok(manifests)
    }

    /// Compare trust levels
    fn trust_level_gte(level: &TrustLevel, min_level: &TrustLevel) -> bool {
        use TrustLevel::*;
        
        let level_value = match level {
            Core => 5,
            Verified => 4,
            Trusted => 3,
            Known => 2,
            Unknown => 1,
        };
        
        let min_value = match min_level {
            Core => 5,
            Verified => 4,
            Trusted => 3,
            Known => 2,
            Unknown => 1,
        };
        
        level_value >= min_value
    }

    /// Load index from file
    fn load_index(path: &Path) -> Result<(HashMap<String, IndexEntry>, InvertedIndex)> {
        let content = fs::read_to_string(path)
            .context("Failed to read index file")?;
        let index: HashMap<String, IndexEntry> = serde_json::from_str(&content)
            .context("Failed to parse index file")?;
        
        // Rebuild inverted index
        let mut inverted = InvertedIndex::default();
        let mut discovery = ModuleDiscovery {
            store_root: PathBuf::new(),
            index: HashMap::new(),
            index_path: PathBuf::new(),
            inverted_index: inverted,
        };
        
        for (id, entry) in &index {
            discovery.update_inverted_index(id, entry);
        }
        
        Ok((index, discovery.inverted_index))
    }

    /// Save index to file
    fn save_index(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.index)
            .context("Failed to serialize index")?;
        fs::write(&self.index_path, content)
            .context("Failed to write index file")?;
        Ok(())
    }

    /// Rebuild index from store directory
    pub fn rebuild_index(&mut self) -> Result<()> {
        self.index.clear();
        self.inverted_index = InvertedIndex::default();

        // Scan store directory for modules
        let entries = fs::read_dir(&self.store_root)
            .context("Failed to read store directory")?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                let manifest_path = path.join("module.toml");
                if manifest_path.exists() {
                    match ManifestParser::parse_from_path(&manifest_path) {
                        Ok(manifest) => {
                            let module_id = entry.file_name().to_string_lossy().to_string();
                            self.index_module(&module_id, &manifest)?;
                        }
                        Err(e) => {
                            eprintln!("Failed to parse manifest at {:?}: {}", manifest_path, e);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_discovery_creation() {
        let temp_dir = TempDir::new().unwrap();
        let discovery = ModuleDiscovery::new(temp_dir.path().to_path_buf());
        assert!(discovery.index.is_empty());
    }

    #[test]
    fn test_trust_level_comparison() {
        assert!(ModuleDiscovery::trust_level_gte(&TrustLevel::Core, &TrustLevel::Unknown));
        assert!(ModuleDiscovery::trust_level_gte(&TrustLevel::Verified, &TrustLevel::Trusted));
        assert!(!ModuleDiscovery::trust_level_gte(&TrustLevel::Unknown, &TrustLevel::Core));
    }
}