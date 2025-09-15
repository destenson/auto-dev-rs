//! Test coverage analysis

use crate::parser::model::{Specification, Requirement};
use super::{TestCase, TestSuite};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Coverage analyzer for test suites
pub struct CoverageAnalyzer;

impl CoverageAnalyzer {
    pub fn new() -> Self {
        Self
    }
    
    /// Analyze test coverage for a specification
    pub fn analyze_coverage(
        &self,
        spec: &Specification,
        test_suite: &TestSuite,
    ) -> Result<CoverageReport> {
        let mut report = CoverageReport::default();
        
        // Extract all requirements
        let all_requirements: HashSet<String> = spec.requirements
            .iter()
            .map(|r| r.id.clone())
            .collect();
        
        // Find covered requirements
        let covered_requirements: HashSet<String> = test_suite.tests
            .iter()
            .filter_map(|test| self.extract_requirement_id(&test.description))
            .collect();
        
        // Calculate coverage
        report.total_requirements = all_requirements.len();
        report.covered_requirements = covered_requirements.len();
        report.requirement_coverage = if report.total_requirements > 0 {
            (report.covered_requirements as f32 / report.total_requirements as f32) * 100.0
        } else {
            0.0
        };
        
        // Find uncovered requirements
        report.uncovered_requirements = all_requirements
            .difference(&covered_requirements)
            .cloned()
            .collect();
        
        // Count test types
        for test in &test_suite.tests {
            match test.test_type {
                super::TestType::Unit => report.unit_tests += 1,
                super::TestType::Integration => report.integration_tests += 1,
                super::TestType::Property => report.property_tests += 1,
                _ => report.other_tests += 1,
            }
        }
        
        report.total_tests = test_suite.tests.len();
        
        Ok(report)
    }
    
    /// Extract requirement ID from test description
    fn extract_requirement_id(&self, description: &str) -> Option<String> {
        // Look for patterns like "REQ-123" or "Requirement: REQ-123"
        if let Some(start) = description.find("REQ-") {
            let id_part = &description[start..];
            let end = id_part.find(|c: char| !c.is_alphanumeric() && c != '-')
                .unwrap_or(id_part.len());
            return Some(id_part[..end].to_string());
        }
        None
    }
    
    /// Generate coverage suggestions
    pub fn generate_suggestions(&self, report: &CoverageReport) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        if report.requirement_coverage < 80.0 {
            suggestions.push(format!(
                "Test coverage is below 80% ({:.1}%). Add tests for uncovered requirements.",
                report.requirement_coverage
            ));
        }
        
        if report.unit_tests == 0 {
            suggestions.push("No unit tests found. Add unit tests for individual functions.".to_string());
        }
        
        if report.integration_tests == 0 {
            suggestions.push("No integration tests found. Add tests for component interactions.".to_string());
        }
        
        if !report.uncovered_requirements.is_empty() {
            suggestions.push(format!(
                "Found {} uncovered requirements. Priority: {:?}",
                report.uncovered_requirements.len(),
                report.uncovered_requirements.iter().take(3).collect::<Vec<_>>()
            ));
        }
        
        suggestions
    }
}

/// Coverage report
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoverageReport {
    pub total_requirements: usize,
    pub covered_requirements: usize,
    pub requirement_coverage: f32,
    pub uncovered_requirements: Vec<String>,
    pub total_tests: usize,
    pub unit_tests: usize,
    pub integration_tests: usize,
    pub property_tests: usize,
    pub other_tests: usize,
}

impl CoverageReport {
    pub fn summary(&self) -> String {
        format!(
            "Coverage Report:\n\
             - Requirements: {}/{} ({:.1}%)\n\
             - Total Tests: {}\n\
             - Unit Tests: {}\n\
             - Integration Tests: {}\n\
             - Property Tests: {}\n\
             - Uncovered Requirements: {}",
            self.covered_requirements,
            self.total_requirements,
            self.requirement_coverage,
            self.total_tests,
            self.unit_tests,
            self.integration_tests,
            self.property_tests,
            self.uncovered_requirements.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::model::Priority;
    use std::collections::HashMap;

    #[test]
    fn test_coverage_analyzer_creation() {
        let analyzer = CoverageAnalyzer::new();
        let _ = analyzer; // Analyzer created successfully
    }

    #[test]
    fn test_requirement_id_extraction() {
        let analyzer = CoverageAnalyzer::new();
        
        assert_eq!(
            analyzer.extract_requirement_id("Test for REQ-123"),
            Some("REQ-123".to_string())
        );
        
        assert_eq!(
            analyzer.extract_requirement_id("Requirement: REQ-456 validation"),
            Some("REQ-456".to_string())
        );
        
        assert_eq!(
            analyzer.extract_requirement_id("No requirement here"),
            None
        );
    }

    #[test]
    fn test_coverage_calculation() {
        let analyzer = CoverageAnalyzer::new();
        
        let mut spec = Specification::new(std::path::PathBuf::from("test_spec.md"));
        spec.requirements.push(Requirement {
            id: "REQ-1".to_string(),
            description: "First requirement".to_string(),
            priority: Priority::High,
            category: crate::parser::model::RequirementType::Functional,
            acceptance_criteria: Vec::new(),
            source_location: Default::default(),
            related: Vec::new(),
            tags: Vec::new(),
        });
        spec.requirements.push(Requirement {
            id: "REQ-2".to_string(),
            description: "Second requirement".to_string(),
            priority: Priority::Medium,
            category: crate::parser::model::RequirementType::Functional,
            acceptance_criteria: Vec::new(),
            source_location: Default::default(),
            related: Vec::new(),
            tags: Vec::new(),
        });
        
        let mut suite = TestSuite::new("test_suite");
        let mut test1 = TestCase::new("test_1", crate::test_gen::TestType::Unit);
        test1.description = "Test for REQ-1".to_string();
        suite.add_test(test1);
        
        let report = analyzer.analyze_coverage(&spec, &suite).unwrap();
        
        assert_eq!(report.total_requirements, 2);
        assert_eq!(report.covered_requirements, 1);
        assert_eq!(report.requirement_coverage, 50.0);
        assert!(report.uncovered_requirements.contains(&"REQ-2".to_string()));
    }

    #[test]
    fn test_suggestion_generation() {
        let analyzer = CoverageAnalyzer::new();
        
        let mut report = CoverageReport::default();
        report.requirement_coverage = 60.0;
        report.unit_tests = 0;
        report.uncovered_requirements = vec!["REQ-1".to_string(), "REQ-2".to_string()];
        
        let suggestions = analyzer.generate_suggestions(&report);
        
        assert!(suggestions.iter().any(|s| s.contains("below 80%")));
        assert!(suggestions.iter().any(|s| s.contains("No unit tests")));
        assert!(suggestions.iter().any(|s| s.contains("uncovered requirements")));
    }
}