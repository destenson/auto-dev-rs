use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::context::storage::ProjectContext;
use crate::context::analyzer::{CodePattern, PatternType, CodingConventions};
use crate::context::manager::{ArchitectureDecision, CodeExample};

#[derive(Debug, Clone)]
pub struct ContextQuery {
    context: ProjectContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub patterns: Vec<CodePattern>,
    pub conventions: Option<CodingConventions>,
    pub decisions: Vec<ArchitectureDecision>,
    pub examples: Vec<CodeExample>,
    pub statistics: QueryStatistics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStatistics {
    pub total_patterns: usize,
    pub total_decisions: usize,
    pub total_modules: usize,
    pub total_dependencies: usize,
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
}

impl ContextQuery {
    pub fn new(context: ProjectContext) -> Self {
        Self { context }
    }

    pub fn find_patterns_by_type(&self, pattern_type: PatternType) -> Vec<CodePattern> {
        self.context
            .patterns
            .iter()
            .filter(|p| matches!(&p.pattern_type, pt if std::mem::discriminant(pt) == std::mem::discriminant(&pattern_type)))
            .cloned()
            .collect()
    }

    pub fn find_patterns_by_file(&self, file_path: &PathBuf) -> Vec<CodePattern> {
        self.context
            .patterns
            .iter()
            .filter(|p| p.locations.contains(file_path))
            .cloned()
            .collect()
    }

    pub fn find_patterns_by_frequency(&self, min_frequency: f32) -> Vec<CodePattern> {
        self.context
            .patterns
            .iter()
            .filter(|p| p.frequency >= min_frequency)
            .cloned()
            .collect()
    }

    pub fn get_most_common_patterns(&self, limit: usize) -> Vec<CodePattern> {
        let mut patterns = self.context.patterns.clone();
        patterns.sort_by(|a, b| b.frequency.partial_cmp(&a.frequency).unwrap());
        patterns.into_iter().take(limit).collect()
    }

    pub fn get_conventions(&self) -> &CodingConventions {
        &self.context.conventions
    }

    pub fn get_decisions_by_date(&self, after: chrono::DateTime<chrono::Utc>) -> Vec<ArchitectureDecision> {
        self.context
            .decisions
            .iter()
            .filter(|d| d.timestamp > after)
            .cloned()
            .collect()
    }

    pub fn get_recent_decisions(&self, limit: usize) -> Vec<ArchitectureDecision> {
        let mut decisions = self.context.decisions.clone();
        decisions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        decisions.into_iter().take(limit).collect()
    }

    pub fn search_decisions(&self, query: &str) -> Vec<ArchitectureDecision> {
        let query_lower = query.to_lowercase();
        self.context
            .decisions
            .iter()
            .filter(|d| {
                d.title.to_lowercase().contains(&query_lower) ||
                d.description.to_lowercase().contains(&query_lower) ||
                d.rationale.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect()
    }

    pub fn get_project_languages(&self) -> Vec<String> {
        self.context
            .metadata
            .languages
            .iter()
            .map(|l| format!("{:?}", l))
            .collect()
    }

    pub fn get_project_frameworks(&self) -> Vec<String> {
        self.context
            .metadata
            .frameworks
            .iter()
            .map(|f| f.name.clone())
            .collect()
    }

    pub fn get_external_dependencies(&self) -> Vec<String> {
        self.context
            .dependencies
            .external_dependencies
            .iter()
            .map(|d| d.name.clone())
            .collect()
    }

    pub fn find_circular_dependencies(&self) -> Vec<Vec<String>> {
        self.context
            .dependencies
            .circular_dependencies
            .iter()
            .map(|cd| cd.cycle.clone())
            .collect()
    }

    pub fn get_module_dependencies(&self, module_name: &str) -> Vec<String> {
        self.context
            .dependencies
            .edges
            .iter()
            .filter(|e| e.from == module_name)
            .map(|e| e.to.clone())
            .collect()
    }

    pub fn get_module_dependents(&self, module_name: &str) -> Vec<String> {
        self.context
            .dependencies
            .edges
            .iter()
            .filter(|e| e.to == module_name)
            .map(|e| e.from.clone())
            .collect()
    }

    pub fn get_statistics(&self) -> QueryStatistics {
        QueryStatistics {
            total_patterns: self.context.patterns.len(),
            total_decisions: self.context.decisions.len(),
            total_modules: self.context.dependencies.modules.len(),
            total_dependencies: self.context.dependencies.external_dependencies.len(),
            languages: self.get_project_languages(),
            frameworks: self.get_project_frameworks(),
        }
    }

    pub fn execute_complex_query(&self, query: ComplexQuery) -> QueryResult {
        let mut patterns = Vec::new();
        let mut decisions = Vec::new();
        let mut examples = Vec::new();

        // Apply pattern filters
        if let Some(pattern_type) = query.pattern_type {
            patterns.extend(self.find_patterns_by_type(pattern_type));
        }

        if let Some(min_frequency) = query.min_pattern_frequency {
            patterns.extend(self.find_patterns_by_frequency(min_frequency));
        }

        // Apply decision filters
        if let Some(decision_query) = query.decision_search {
            decisions.extend(self.search_decisions(&decision_query));
        }

        if let Some(after) = query.decisions_after {
            decisions.extend(self.get_decisions_by_date(after));
        }

        // Collect examples from patterns
        for pattern in &patterns {
            examples.extend(pattern.examples.clone());
        }

        // Apply limits
        if let Some(limit) = query.limit {
            patterns.truncate(limit);
            decisions.truncate(limit);
            examples.truncate(limit);
        }

        QueryResult {
            patterns,
            conventions: if query.include_conventions {
                Some(self.context.conventions.clone())
            } else {
                None
            },
            decisions,
            examples,
            statistics: self.get_statistics(),
        }
    }

    pub fn get_project_summary(&self) -> ProjectSummary {
        ProjectSummary {
            name: self.context.metadata.name.clone(),
            languages: self.get_project_languages(),
            frameworks: self.get_project_frameworks(),
            total_files: self.context.structure.files.len(),
            total_directories: self.context.structure.directories.len(),
            lines_of_code: self.context.structure.statistics.lines_of_code,
            test_coverage: self.calculate_test_coverage(),
            most_common_patterns: self.get_most_common_patterns(5),
            recent_decisions: self.get_recent_decisions(5),
            circular_dependencies: self.find_circular_dependencies(),
            external_dependencies_count: self.context.dependencies.external_dependencies.len(),
        }
    }

    fn calculate_test_coverage(&self) -> f32 {
        let total_modules = self.context.dependencies.modules.len();
        if total_modules == 0 {
            return 0.0;
        }

        let test_modules = self.context
            .dependencies
            .modules
            .iter()
            .filter(|m| matches!(m.module_type, crate::context::analyzer::dependencies::ModuleType::Test))
            .count();

        (test_modules as f32 / total_modules as f32) * 100.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexQuery {
    pub pattern_type: Option<PatternType>,
    pub min_pattern_frequency: Option<f32>,
    pub decision_search: Option<String>,
    pub decisions_after: Option<chrono::DateTime<chrono::Utc>>,
    pub include_conventions: bool,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub name: String,
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
    pub total_files: usize,
    pub total_directories: usize,
    pub lines_of_code: usize,
    pub test_coverage: f32,
    pub most_common_patterns: Vec<CodePattern>,
    pub recent_decisions: Vec<ArchitectureDecision>,
    pub circular_dependencies: Vec<Vec<String>>,
    pub external_dependencies_count: usize,
}