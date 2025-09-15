//! Change impact analysis to determine what actions to take

use crate::monitor::{ChangeType, FileCategory, FileChange};
use dashmap::DashMap;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info};

/// Represents the impact level of a change
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangeImpact {
    /// No action needed
    None,
    /// Minor update needed (e.g., comment update)
    Minor,
    /// Moderate changes needed (e.g., function update)
    Moderate,
    /// Major changes needed (e.g., API change)
    Major,
    /// Critical changes needed (e.g., spec change)
    Critical,
}

/// Represents an action to take based on a change
#[derive(Debug, Clone)]
pub enum ChangeAction {
    /// Update implementation to match specification
    UpdateImplementation { spec_file: PathBuf, target_files: Vec<PathBuf> },
    /// Generate code to pass new test
    ImplementTest { test_file: PathBuf, target_module: Option<PathBuf> },
    /// Update documentation
    UpdateDocumentation { doc_file: PathBuf, related_code: Vec<PathBuf> },
    /// Regenerate code from schema
    RegenerateFromSchema { schema_file: PathBuf, generated_files: Vec<PathBuf> },
    /// Create implementation from example
    ImplementFromExample { example_file: PathBuf, target_location: PathBuf },
    /// Revalidate implementation
    Revalidate { changed_files: Vec<PathBuf> },
}

/// Analyzes file changes to determine their impact
pub struct ChangeAnalyzer {
    /// Tracks dependencies between files
    dependencies: Arc<DashMap<PathBuf, HashSet<PathBuf>>>,
    /// Tracks which specs relate to which implementations
    spec_to_impl: Arc<DashMap<PathBuf, HashSet<PathBuf>>>,
    /// Tracks which tests relate to which implementations
    test_to_impl: Arc<DashMap<PathBuf, HashSet<PathBuf>>>,
}

impl ChangeAnalyzer {
    /// Create a new change analyzer
    pub fn new() -> Self {
        Self {
            dependencies: Arc::new(DashMap::new()),
            spec_to_impl: Arc::new(DashMap::new()),
            test_to_impl: Arc::new(DashMap::new()),
        }
    }

    /// Analyze a file change and determine its impact
    pub fn analyze(&self, change: &FileChange) -> ChangeImpact {
        match change.category {
            FileCategory::Specification => {
                // Spec changes have critical impact
                info!("Specification change detected: {:?}", change.path);
                ChangeImpact::Critical
            }
            FileCategory::Test => {
                // New tests need implementation
                match change.change_type {
                    ChangeType::Created => {
                        info!("New test detected: {:?}", change.path);
                        ChangeImpact::Major
                    }
                    ChangeType::Modified => {
                        info!("Test modified: {:?}", change.path);
                        ChangeImpact::Moderate
                    }
                    _ => ChangeImpact::Minor,
                }
            }
            FileCategory::Schema => {
                // Schema changes require regeneration
                info!("Schema change detected: {:?}", change.path);
                ChangeImpact::Major
            }
            FileCategory::Documentation => {
                // Documentation changes might need code updates
                debug!("Documentation change: {:?}", change.path);
                ChangeImpact::Minor
            }
            FileCategory::Example => {
                // New examples might inspire implementations
                if change.change_type == ChangeType::Created {
                    info!("New example detected: {:?}", change.path);
                    ChangeImpact::Moderate
                } else {
                    ChangeImpact::Minor
                }
            }
            FileCategory::Configuration => {
                // Config changes might affect build/runtime
                debug!("Configuration change: {:?}", change.path);
                ChangeImpact::Moderate
            }
            FileCategory::Implementation => {
                // Implementation changes need validation
                debug!("Implementation change: {:?}", change.path);
                ChangeImpact::Minor
            }
            FileCategory::Other => ChangeImpact::None,
        }
    }

    /// Determine what actions to take based on a change
    pub fn determine_actions(&self, change: &FileChange) -> Vec<ChangeAction> {
        let mut actions = Vec::new();

        match change.category {
            FileCategory::Specification => {
                // Find related implementation files
                let impl_files = self.get_related_implementations(&change.path);
                actions.push(ChangeAction::UpdateImplementation {
                    spec_file: change.path.clone(),
                    target_files: impl_files,
                });
            }
            FileCategory::Test => {
                if change.change_type == ChangeType::Created {
                    // Determine target module from test path
                    let target = self.infer_target_from_test(&change.path);
                    actions.push(ChangeAction::ImplementTest {
                        test_file: change.path.clone(),
                        target_module: target,
                    });
                }
            }
            FileCategory::Schema => {
                // Find generated files from this schema
                let generated = self.get_generated_from_schema(&change.path);
                actions.push(ChangeAction::RegenerateFromSchema {
                    schema_file: change.path.clone(),
                    generated_files: generated,
                });
            }
            FileCategory::Documentation => {
                // Find related code files
                let related = self.get_related_code(&change.path);
                if !related.is_empty() {
                    actions.push(ChangeAction::UpdateDocumentation {
                        doc_file: change.path.clone(),
                        related_code: related,
                    });
                }
            }
            FileCategory::Example => {
                if change.change_type == ChangeType::Created {
                    // Determine where to implement based on example
                    let target = self.infer_target_from_example(&change.path);
                    actions.push(ChangeAction::ImplementFromExample {
                        example_file: change.path.clone(),
                        target_location: target,
                    });
                }
            }
            FileCategory::Implementation | FileCategory::Configuration => {
                // Revalidate after changes
                actions.push(ChangeAction::Revalidate { changed_files: vec![change.path.clone()] });
            }
            _ => {}
        }

        actions
    }

    /// Add a dependency relationship between files
    pub fn add_dependency(&self, file: PathBuf, depends_on: PathBuf) {
        self.dependencies.entry(file).or_insert_with(HashSet::new).insert(depends_on);
    }

    /// Link a specification to implementation files
    pub fn link_spec_to_impl(&self, spec: PathBuf, impl_file: PathBuf) {
        self.spec_to_impl.entry(spec).or_insert_with(HashSet::new).insert(impl_file);
    }

    /// Link a test to implementation files
    pub fn link_test_to_impl(&self, test: PathBuf, impl_file: PathBuf) {
        self.test_to_impl.entry(test).or_insert_with(HashSet::new).insert(impl_file);
    }

    /// Get implementation files related to a specification
    fn get_related_implementations(&self, spec_path: &Path) -> Vec<PathBuf> {
        self.spec_to_impl
            .get(spec_path)
            .map(|entry| entry.value().iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get code files related to documentation
    fn get_related_code(&self, doc_path: &Path) -> Vec<PathBuf> {
        // Simple heuristic: if doc is named after a module, find that module
        if let Some(stem) = doc_path.file_stem() {
            let stem_str = stem.to_string_lossy().to_lowercase();

            // Look for matching implementation files
            let mut related = Vec::new();
            for entry in self.dependencies.iter() {
                let path = entry.key();
                if path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_lowercase() == stem_str)
                    .unwrap_or(false)
                {
                    related.push(path.clone());
                }
            }
            return related;
        }
        Vec::new()
    }

    /// Get generated files from a schema
    fn get_generated_from_schema(&self, schema_path: &Path) -> Vec<PathBuf> {
        // This would be populated by the code generation system
        // For now, use a simple heuristic
        let mut generated = Vec::new();

        if let Some(stem) = schema_path.file_stem() {
            let base_name = stem.to_string_lossy();
            // Common patterns for generated files
            generated.push(PathBuf::from(format!("src/generated/{}_types.rs", base_name)));
            generated.push(PathBuf::from(format!("src/generated/{}_client.rs", base_name)));
        }

        generated
    }

    /// Infer target module from test file path
    fn infer_target_from_test(&self, test_path: &Path) -> Option<PathBuf> {
        // Remove _test suffix or test_ prefix to find target
        if let Some(stem) = test_path.file_stem() {
            let stem_str = stem.to_string_lossy();

            let target_name = if stem_str.ends_with("_test") {
                stem_str.trim_end_matches("_test")
            } else if stem_str.starts_with("test_") {
                stem_str.trim_start_matches("test_")
            } else {
                return None;
            };

            // Look for matching implementation file
            let target_path = test_path.with_file_name(format!("{}.rs", target_name));
            if target_path.exists() {
                return Some(target_path);
            }

            // Try in src directory
            let src_path = PathBuf::from("src").join(format!("{}.rs", target_name));
            if src_path.exists() {
                return Some(src_path);
            }
        }

        None
    }

    /// Infer target location from example file
    fn infer_target_from_example(&self, example_path: &Path) -> PathBuf {
        // Place implementation in src/ with similar structure
        let relative = example_path.strip_prefix("examples/").unwrap_or(example_path);

        PathBuf::from("src").join(relative)
    }

    /// Get all files affected by a change (cascade effect)
    pub fn get_cascade_effect(&self, changed_file: &Path) -> HashSet<PathBuf> {
        let mut affected = HashSet::new();
        let mut to_check = vec![changed_file.to_path_buf()];

        while let Some(file) = to_check.pop() {
            // Find all files that depend on this file
            for entry in self.dependencies.iter() {
                if entry.value().contains(&file) && !affected.contains(entry.key()) {
                    affected.insert(entry.key().clone());
                    to_check.push(entry.key().clone());
                }
            }
        }

        affected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::ChangeType;
    use std::time::SystemTime;

    #[test]
    fn test_analyze_spec_change() {
        let analyzer = ChangeAnalyzer::new();

        let change = FileChange {
            path: PathBuf::from("SPEC.md"),
            category: FileCategory::Specification,
            change_type: ChangeType::Modified,
            timestamp: SystemTime::now(),
        };

        assert_eq!(analyzer.analyze(&change), ChangeImpact::Critical);
    }

    #[test]
    fn test_analyze_new_test() {
        let analyzer = ChangeAnalyzer::new();

        let change = FileChange {
            path: PathBuf::from("auth_test.rs"),
            category: FileCategory::Test,
            change_type: ChangeType::Created,
            timestamp: SystemTime::now(),
        };

        assert_eq!(analyzer.analyze(&change), ChangeImpact::Major);
    }

    #[test]
    fn test_determine_actions_for_spec() {
        let analyzer = ChangeAnalyzer::new();

        // Link spec to implementation
        analyzer.link_spec_to_impl(PathBuf::from("specs/auth.md"), PathBuf::from("src/auth.rs"));

        let change = FileChange {
            path: PathBuf::from("specs/auth.md"),
            category: FileCategory::Specification,
            change_type: ChangeType::Modified,
            timestamp: SystemTime::now(),
        };

        let actions = analyzer.determine_actions(&change);
        assert_eq!(actions.len(), 1);

        match &actions[0] {
            ChangeAction::UpdateImplementation { spec_file, target_files } => {
                assert_eq!(spec_file, &PathBuf::from("specs/auth.md"));
                assert_eq!(target_files.len(), 1);
                assert_eq!(target_files[0], PathBuf::from("src/auth.rs"));
            }
            _ => panic!("Wrong action type"),
        }
    }

    #[test]
    fn test_cascade_effect() {
        let analyzer = ChangeAnalyzer::new();

        // Set up dependencies: C depends on B, B depends on A
        analyzer.add_dependency(PathBuf::from("b.rs"), PathBuf::from("a.rs"));
        analyzer.add_dependency(PathBuf::from("c.rs"), PathBuf::from("b.rs"));

        let affected = analyzer.get_cascade_effect(Path::new("a.rs"));
        assert_eq!(affected.len(), 2);
        assert!(affected.contains(&PathBuf::from("b.rs")));
        assert!(affected.contains(&PathBuf::from("c.rs")));
    }
}
