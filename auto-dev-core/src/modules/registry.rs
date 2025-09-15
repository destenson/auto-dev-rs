// Module Registry
//
// Tracks loaded modules, manages lifecycle, and handles dependencies

use std::collections::{HashMap, HashSet};
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::modules::interface::{ModuleInterface, ModuleCapability, ModuleVersion, ModuleMetadata};
use crate::modules::loader::LoadedModule;

/// Module status in the registry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModuleStatus {
    Loading,
    Ready,
    Running,
    Stopping,
    Stopped,
    Failed(String),
}

/// Information about a registered module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub id: String,
    pub metadata: ModuleMetadata,
    pub status: ModuleStatus,
    pub loaded_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub execution_count: u64,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
}

/// Module registry entry
struct RegistryEntry {
    module: LoadedModule,
    info: ModuleInfo,
}

/// Module registry that tracks all loaded modules
pub struct ModuleRegistry {
    modules: HashMap<String, RegistryEntry>,
    dependency_graph: HashMap<String, HashSet<String>>,
}

impl ModuleRegistry {
    /// Create a new module registry
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            dependency_graph: HashMap::new(),
        }
    }

    /// Register a new module
    pub async fn register(&mut self, mut module: LoadedModule) -> Result<String> {
        // Initialize the module
        module.as_interface_mut().initialize().await
            .context("Failed to initialize module")?;

        let metadata = module.metadata();
        let module_id = metadata.name.clone();

        // Check for duplicate
        if self.modules.contains_key(&module_id) {
            anyhow::bail!("Module already registered: {}", module_id);
        }

        // Resolve dependencies
        let dependencies = self.resolve_dependencies(&metadata)?;

        // Create module info
        let info = ModuleInfo {
            id: module_id.clone(),
            metadata,
            status: ModuleStatus::Ready,
            loaded_at: Utc::now(),
            last_accessed: Utc::now(),
            execution_count: 0,
            dependencies: dependencies.clone(),
            dependents: Vec::new(),
        };

        // Update dependency graph
        for dep in &dependencies {
            self.dependency_graph
                .entry(dep.clone())
                .or_insert_with(HashSet::new)
                .insert(module_id.clone());
        }

        // Register the module
        let entry = RegistryEntry { module, info };
        self.modules.insert(module_id.clone(), entry);

        Ok(module_id)
    }

    /// Unregister a module
    pub async fn unregister(&mut self, module_id: &str) -> Result<()> {
        // Check if module exists
        let entry = self.modules.get(module_id)
            .ok_or_else(|| anyhow::anyhow!("Module not found: {}", module_id))?;

        // Check for dependents
        if let Some(dependents) = self.dependency_graph.get(module_id) {
            if !dependents.is_empty() {
                let deps: Vec<String> = dependents.iter().cloned().collect();
                anyhow::bail!(
                    "Cannot unregister module '{}' - required by: {:?}",
                    module_id,
                    deps
                );
            }
        }

        // Remove from dependency graph
        for dep in &entry.info.dependencies {
            if let Some(dependents) = self.dependency_graph.get_mut(dep) {
                dependents.remove(module_id);
            }
        }
        self.dependency_graph.remove(module_id);

        // Remove the module
        self.modules.remove(module_id);

        Ok(())
    }

    /// Update a module (for hot-reload)
    pub async fn update(&mut self, module_id: &str, mut new_module: LoadedModule) -> Result<()> {
        // Get existing entry
        let old_entry = self.modules.get_mut(module_id)
            .ok_or_else(|| anyhow::anyhow!("Module not found: {}", module_id))?;

        // Initialize new module
        new_module.as_interface_mut().initialize().await?;

        // Update the module
        old_entry.module = new_module;
        old_entry.info.metadata = old_entry.module.metadata();
        old_entry.info.last_accessed = Utc::now();

        Ok(())
    }

    /// Get a module by ID
    pub fn get(&self, module_id: &str) -> Option<&LoadedModule> {
        self.modules.get(module_id).map(|entry| &entry.module)
    }

    /// Get a mutable reference to a module
    pub fn get_mut(&mut self, module_id: &str) -> Option<&mut LoadedModule> {
        self.modules.get_mut(module_id).map(|entry| {
            entry.info.last_accessed = Utc::now();
            &mut entry.module
        })
    }

    /// Get module info
    pub fn get_info(&self, module_id: &str) -> Option<&ModuleInfo> {
        self.modules.get(module_id).map(|entry| &entry.info)
    }

    /// Update module status
    pub fn set_status(&mut self, module_id: &str, status: ModuleStatus) -> Result<()> {
        let entry = self.modules.get_mut(module_id)
            .ok_or_else(|| anyhow::anyhow!("Module not found: {}", module_id))?;
        
        entry.info.status = status;
        entry.info.last_accessed = Utc::now();
        
        Ok(())
    }

    /// Increment execution count
    pub fn increment_execution_count(&mut self, module_id: &str) -> Result<()> {
        let entry = self.modules.get_mut(module_id)
            .ok_or_else(|| anyhow::anyhow!("Module not found: {}", module_id))?;
        
        entry.info.execution_count += 1;
        entry.info.last_accessed = Utc::now();
        
        Ok(())
    }

    /// Get capabilities provided by a module
    pub fn get_capabilities(&self, module_id: &str) -> Result<Vec<ModuleCapability>> {
        self.modules
            .get(module_id)
            .map(|entry| entry.module.as_interface().get_capabilities())
            .ok_or_else(|| anyhow::anyhow!("Module not found: {}", module_id))
    }

    /// List all modules
    pub fn list_all(&self) -> Vec<ModuleInfo> {
        self.modules
            .values()
            .map(|entry| entry.info.clone())
            .collect()
    }

    /// List modules by status
    pub fn list_by_status(&self, status: ModuleStatus) -> Vec<ModuleInfo> {
        self.modules
            .values()
            .filter(|entry| entry.info.status == status)
            .map(|entry| entry.info.clone())
            .collect()
    }

    /// List modules providing a specific capability
    pub fn list_by_capability(&self, capability: &ModuleCapability) -> Vec<ModuleInfo> {
        self.modules
            .values()
            .filter(|entry| {
                entry.module.as_interface()
                    .get_capabilities()
                    .iter()
                    .any(|cap| matches!(cap, capability))
            })
            .map(|entry| entry.info.clone())
            .collect()
    }

    /// Resolve module dependencies
    fn resolve_dependencies(&self, metadata: &ModuleMetadata) -> Result<Vec<String>> {
        let mut resolved = Vec::new();
        
        for dep in &metadata.dependencies {
            if dep.optional {
                // Skip optional dependencies for now
                continue;
            }

            // Find matching module
            let found = self.modules.iter().find(|(id, entry)| {
                **id == dep.name && Self::version_matches(&entry.info.metadata.version, &dep.version_requirement)
            });

            if let Some((id, _)) = found {
                resolved.push(id.clone());
            } else {
                anyhow::bail!(
                    "Dependency not satisfied: {} {}",
                    dep.name,
                    dep.version_requirement
                );
            }
        }

        Ok(resolved)
    }

    /// Check if a version matches a requirement
    fn version_matches(version: &ModuleVersion, requirement: &str) -> bool {
        // Simple version matching for now
        // TODO: Implement proper semver matching
        if requirement == "*" {
            return true;
        }

        if let Some(req_version) = Self::parse_version(requirement) {
            version.is_compatible_with(&req_version)
        } else {
            false
        }
    }

    /// Parse a version string
    fn parse_version(s: &str) -> Option<ModuleVersion> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() >= 3 {
            let major = parts[0].parse().ok()?;
            let minor = parts[1].parse().ok()?;
            let patch = parts[2].parse().ok()?;
            Some(ModuleVersion::new(major, minor, patch))
        } else {
            None
        }
    }

    /// Get dependency graph for visualization
    pub fn get_dependency_graph(&self) -> HashMap<String, Vec<String>> {
        self.dependency_graph
            .iter()
            .map(|(k, v)| (k.clone(), v.iter().cloned().collect()))
            .collect()
    }

    /// Check for circular dependencies
    pub fn check_circular_dependencies(&self) -> Result<()> {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        for module_id in self.modules.keys() {
            if !visited.contains(module_id) {
                self.dfs_check_circular(module_id, &mut visited, &mut stack)?;
            }
        }

        Ok(())
    }

    fn dfs_check_circular(
        &self,
        module_id: &str,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
    ) -> Result<()> {
        visited.insert(module_id.to_string());
        stack.insert(module_id.to_string());

        if let Some(entry) = self.modules.get(module_id) {
            for dep in &entry.info.dependencies {
                if !visited.contains(dep) {
                    self.dfs_check_circular(dep, visited, stack)?;
                } else if stack.contains(dep) {
                    anyhow::bail!("Circular dependency detected: {} -> {}", module_id, dep);
                }
            }
        }

        stack.remove(module_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = ModuleRegistry::new();
        assert_eq!(registry.list_all().len(), 0);
    }

    #[test]
    fn test_version_matching() {
        let v1 = ModuleVersion::new(1, 0, 0);
        assert!(ModuleRegistry::version_matches(&v1, "*"));
        assert!(ModuleRegistry::version_matches(&v1, "1.0.0"));
        assert!(!ModuleRegistry::version_matches(&v1, "2.0.0"));
    }
}