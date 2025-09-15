pub mod structure;
pub mod patterns;
pub mod conventions;
pub mod dependencies;

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

pub use structure::ProjectStructure;
pub use patterns::{CodePattern, PatternType, PatternDetector};
pub use conventions::{CodingConventions, NamingConventions, NamingStyle};
pub use dependencies::DependencyGraph;

use crate::context::manager::CodeExample;

pub struct ProjectAnalyzer {
    project_root: PathBuf,
    pattern_detector: PatternDetector,
}

impl ProjectAnalyzer {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root: project_root.clone(),
            pattern_detector: PatternDetector::new(project_root),
        }
    }

    pub async fn analyze_structure(&self) -> anyhow::Result<ProjectStructure> {
        structure::analyze_project_structure(&self.project_root).await
    }

    pub async fn detect_patterns(&self) -> anyhow::Result<Vec<CodePattern>> {
        self.pattern_detector.detect_all_patterns().await
    }

    pub async fn infer_conventions(&self) -> anyhow::Result<CodingConventions> {
        conventions::infer_conventions(&self.project_root).await
    }

    pub async fn analyze_dependencies(&self) -> anyhow::Result<DependencyGraph> {
        dependencies::analyze_dependencies(&self.project_root).await
    }

    pub async fn analyze_file(&self, path: &PathBuf) -> anyhow::Result<Vec<CodePattern>> {
        self.pattern_detector.analyze_file(path).await
    }
}