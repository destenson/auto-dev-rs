//! Specification coverage tracking

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

/// Coverage report for specifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub specifications: HashMap<PathBuf, SpecificationCoverage>,
    pub overall_coverage: f32,
    pub requirements_coverage: f32,
    pub api_coverage: f32,
    pub model_coverage: f32,
    pub behavior_coverage: f32,
    pub uncovered_items: Vec<UncoveredItem>,
}

impl CoverageReport {
    /// Create a new coverage report
    pub fn new() -> Self {
        Self {
            specifications: HashMap::new(),
            overall_coverage: 0.0,
            requirements_coverage: 0.0,
            api_coverage: 0.0,
            model_coverage: 0.0,
            behavior_coverage: 0.0,
            uncovered_items: Vec::new(),
        }
    }
    
    /// Update coverage for a specification
    pub fn update_specification(&mut self, path: PathBuf, coverage: SpecificationCoverage) {
        self.specifications.insert(path, coverage);
        self.recalculate();
    }
    
    /// Recalculate overall coverage metrics
    fn recalculate(&mut self) {
        if self.specifications.is_empty() {
            self.overall_coverage = 0.0;
            return;
        }
        
        let mut total_reqs = 0;
        let mut covered_reqs = 0;
        let mut total_apis = 0;
        let mut covered_apis = 0;
        let mut total_models = 0;
        let mut covered_models = 0;
        let mut total_behaviors = 0;
        let mut covered_behaviors = 0;
        
        self.uncovered_items.clear();
        
        for (path, spec_coverage) in &self.specifications {
            total_reqs += spec_coverage.total_requirements;
            covered_reqs += spec_coverage.implemented_requirements;
            total_apis += spec_coverage.total_apis;
            covered_apis += spec_coverage.implemented_apis;
            total_models += spec_coverage.total_models;
            covered_models += spec_coverage.implemented_models;
            total_behaviors += spec_coverage.total_behaviors;
            covered_behaviors += spec_coverage.implemented_behaviors;
            
            // Track uncovered items
            for req_id in &spec_coverage.uncovered_requirements {
                self.uncovered_items.push(UncoveredItem {
                    spec_path: path.clone(),
                    item_type: ItemType::Requirement,
                    item_id: req_id.clone(),
                });
            }
            
            for api_id in &spec_coverage.uncovered_apis {
                self.uncovered_items.push(UncoveredItem {
                    spec_path: path.clone(),
                    item_type: ItemType::Api,
                    item_id: api_id.clone(),
                });
            }
        }
        
        // Calculate coverage percentages
        self.requirements_coverage = if total_reqs > 0 {
            (covered_reqs as f32 / total_reqs as f32) * 100.0
        } else { 100.0 };
        
        self.api_coverage = if total_apis > 0 {
            (covered_apis as f32 / total_apis as f32) * 100.0
        } else { 100.0 };
        
        self.model_coverage = if total_models > 0 {
            (covered_models as f32 / total_models as f32) * 100.0
        } else { 100.0 };
        
        self.behavior_coverage = if total_behaviors > 0 {
            (covered_behaviors as f32 / total_behaviors as f32) * 100.0
        } else { 100.0 };
        
        // Overall coverage is weighted average
        let weights = [0.4, 0.3, 0.2, 0.1]; // requirements, apis, models, behaviors
        self.overall_coverage = 
            self.requirements_coverage * weights[0] +
            self.api_coverage * weights[1] +
            self.model_coverage * weights[2] +
            self.behavior_coverage * weights[3];
    }
    
    /// Check if coverage meets minimum threshold
    pub fn meets_threshold(&self, threshold: f32) -> bool {
        self.overall_coverage >= threshold
    }
    
    /// Get priority uncovered items
    pub fn get_priority_items(&self, limit: usize) -> Vec<&UncoveredItem> {
        self.uncovered_items.iter()
            .filter(|item| matches!(item.item_type, ItemType::Requirement))
            .take(limit)
            .collect()
    }
}

/// Coverage for a single specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecificationCoverage {
    pub total_requirements: usize,
    pub implemented_requirements: usize,
    pub total_apis: usize,
    pub implemented_apis: usize,
    pub total_models: usize,
    pub implemented_models: usize,
    pub total_behaviors: usize,
    pub implemented_behaviors: usize,
    pub uncovered_requirements: HashSet<String>,
    pub uncovered_apis: HashSet<String>,
    pub uncovered_models: HashSet<String>,
    pub uncovered_behaviors: HashSet<String>,
}

impl SpecificationCoverage {
    /// Create new specification coverage
    pub fn new() -> Self {
        Self {
            total_requirements: 0,
            implemented_requirements: 0,
            total_apis: 0,
            implemented_apis: 0,
            total_models: 0,
            implemented_models: 0,
            total_behaviors: 0,
            implemented_behaviors: 0,
            uncovered_requirements: HashSet::new(),
            uncovered_apis: HashSet::new(),
            uncovered_models: HashSet::new(),
            uncovered_behaviors: HashSet::new(),
        }
    }
    
    /// Calculate coverage percentage
    pub fn coverage_percentage(&self) -> f32 {
        let total = self.total_requirements + self.total_apis + 
                   self.total_models + self.total_behaviors;
        let implemented = self.implemented_requirements + self.implemented_apis +
                         self.implemented_models + self.implemented_behaviors;
        
        if total == 0 {
            100.0
        } else {
            (implemented as f32 / total as f32) * 100.0
        }
    }
    
    /// Mark a requirement as implemented
    pub fn mark_requirement_implemented(&mut self, req_id: &str) {
        self.uncovered_requirements.remove(req_id);
        self.implemented_requirements += 1;
    }
    
    /// Mark an API as implemented
    pub fn mark_api_implemented(&mut self, api_id: &str) {
        self.uncovered_apis.remove(api_id);
        self.implemented_apis += 1;
    }
    
    /// Mark a model as implemented
    pub fn mark_model_implemented(&mut self, model_id: &str) {
        self.uncovered_models.remove(model_id);
        self.implemented_models += 1;
    }
    
    /// Mark a behavior as implemented
    pub fn mark_behavior_implemented(&mut self, behavior_id: &str) {
        self.uncovered_behaviors.remove(behavior_id);
        self.implemented_behaviors += 1;
    }
}

/// Specification status for coverage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpecificationStatus {
    NotStarted,
    InProgress { percentage: f32 },
    Complete,
    Failed { reason: String },
}

impl SpecificationStatus {
    pub fn is_complete(&self) -> bool {
        matches!(self, SpecificationStatus::Complete)
    }
}

/// Uncovered specification item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UncoveredItem {
    pub spec_path: PathBuf,
    pub item_type: ItemType,
    pub item_id: String,
}

/// Type of specification item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ItemType {
    Requirement,
    Api,
    Model,
    Behavior,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_coverage_calculation() {
        let mut coverage = SpecificationCoverage::new();
        coverage.total_requirements = 10;
        coverage.implemented_requirements = 7;
        coverage.total_apis = 5;
        coverage.implemented_apis = 5;
        
        let percentage = coverage.coverage_percentage();
        assert!((percentage - 80.0).abs() < 0.1);
    }
    
    #[test]
    fn test_coverage_report() {
        let mut report = CoverageReport::new();
        
        let mut spec_coverage = SpecificationCoverage::new();
        spec_coverage.total_requirements = 10;
        spec_coverage.implemented_requirements = 8;
        
        report.update_specification(PathBuf::from("test.md"), spec_coverage);
        
        assert!(report.requirements_coverage > 0.0);
        assert_eq!(report.uncovered_items.len(), 0);
    }
}