//! Main test generator that orchestrates test creation from specifications

use crate::parser::model::{Specification, Requirement};
use crate::llm::provider::LLMProvider;
use super::{TestSuite, TestCase, TestType, TestInput, Assertion, Property};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for test generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestGenerationConfig {
    pub generate_unit_tests: bool,
    pub generate_integration_tests: bool,
    pub generate_property_tests: bool,
    pub generate_edge_cases: bool,
    pub target_coverage: f32,
    pub max_tests_per_function: usize,
}

impl Default for TestGenerationConfig {
    fn default() -> Self {
        Self {
            generate_unit_tests: true,
            generate_integration_tests: true,
            generate_property_tests: true,
            generate_edge_cases: true,
            target_coverage: 80.0,
            max_tests_per_function: 10,
        }
    }
}

/// Main test generator
pub struct TestGenerator {
    config: TestGenerationConfig,
    spec_analyzer: SpecAnalyzer,
    test_builder: TestBuilder,
    property_generator: PropertyGenerator,
}

impl TestGenerator {
    pub fn new(config: TestGenerationConfig) -> Self {
        Self {
            config,
            spec_analyzer: SpecAnalyzer::new(),
            test_builder: TestBuilder::new(),
            property_generator: PropertyGenerator::new(),
        }
    }

    /// Generate tests from a specification
    pub async fn generate_tests(
        &self, 
        spec: &Specification,
        llm: Option<&dyn LLMProvider>,
    ) -> Result<TestSuite> {
        let mut suite = TestSuite::new("generated_tests");
        
        // Extract test requirements from specification
        let requirements = self.spec_analyzer.extract_requirements(spec)?;
        
        // Generate different types of tests
        if self.config.generate_unit_tests {
            let unit_tests = self.generate_unit_tests(&requirements, llm).await?;
            for test in unit_tests {
                suite.add_test(test);
            }
        }
        
        if self.config.generate_property_tests {
            let property_tests = self.generate_property_tests(&requirements)?;
            for test in property_tests {
                suite.add_test(test);
            }
        }
        
        if self.config.generate_edge_cases {
            let edge_tests = self.generate_edge_case_tests(&requirements)?;
            for test in edge_tests {
                suite.add_test(test);
            }
        }
        
        if self.config.generate_integration_tests {
            let integration_tests = self.generate_integration_tests(&requirements, llm).await?;
            for test in integration_tests {
                suite.add_test(test);
            }
        }
        
        Ok(suite)
    }
    
    async fn generate_unit_tests(
        &self,
        requirements: &[TestRequirement],
        llm: Option<&dyn LLMProvider>,
    ) -> Result<Vec<TestCase>> {
        let mut tests = Vec::new();
        
        for req in requirements {
            if req.requirement_type == RequirementType::Functional {
                let test = self.test_builder.build_unit_test(req, llm).await?;
                tests.push(test);
            }
        }
        
        Ok(tests)
    }
    
    fn generate_property_tests(&self, requirements: &[TestRequirement]) -> Result<Vec<TestCase>> {
        let mut tests = Vec::new();
        
        for req in requirements {
            if let Some(properties) = self.property_generator.detect_properties(req) {
                for property in properties {
                    let test = self.test_builder.build_property_test(&property)?;
                    tests.push(test);
                }
            }
        }
        
        Ok(tests)
    }
    
    fn generate_edge_case_tests(&self, requirements: &[TestRequirement]) -> Result<Vec<TestCase>> {
        let mut tests = Vec::new();
        
        for req in requirements {
            let edge_cases = self.detect_edge_cases(req);
            for edge_case in edge_cases {
                let test = self.test_builder.build_edge_case_test(req, &edge_case)?;
                tests.push(test);
            }
        }
        
        Ok(tests)
    }
    
    async fn generate_integration_tests(
        &self,
        requirements: &[TestRequirement],
        llm: Option<&dyn LLMProvider>,
    ) -> Result<Vec<TestCase>> {
        let mut tests = Vec::new();
        
        // Find requirements that involve multiple components
        let integration_reqs: Vec<_> = requirements.iter()
            .filter(|r| r.involves_multiple_components())
            .collect();
        
        for req in integration_reqs {
            let test = self.test_builder.build_integration_test(req, llm).await?;
            tests.push(test);
        }
        
        Ok(tests)
    }
    
    fn detect_edge_cases(&self, req: &TestRequirement) -> Vec<TestInput> {
        let mut edge_cases = Vec::new();
        
        for param in &req.parameters {
            match param.param_type.as_str() {
                "string" => {
                    edge_cases.push(TestInput::Empty);
                    edge_cases.push(TestInput::Whitespace);
                    edge_cases.push(TestInput::Unicode("🎉".to_string()));
                    edge_cases.push(TestInput::VeryLong(10000));
                    edge_cases.push(TestInput::SqlInjection("'; DROP TABLE--".to_string()));
                }
                "number" | "integer" => {
                    edge_cases.push(TestInput::Zero);
                    edge_cases.push(TestInput::Negative);
                    edge_cases.push(TestInput::MaxValue);
                    edge_cases.push(TestInput::MinValue);
                    edge_cases.push(TestInput::NaN);
                }
                "array" => {
                    edge_cases.push(TestInput::EmptyArray);
                    edge_cases.push(TestInput::SingleElement);
                    edge_cases.push(TestInput::Duplicates);
                    edge_cases.push(TestInput::LargeArray(10000));
                }
                _ => {}
            }
        }
        
        edge_cases
    }
}

/// Analyzes specifications to extract test requirements
pub struct SpecAnalyzer {
    patterns: HashMap<String, TestPattern>,
}

impl SpecAnalyzer {
    pub fn new() -> Self {
        let mut patterns = HashMap::new();
        
        // Add common test patterns
        patterns.insert("given_when_then".to_string(), TestPattern::GivenWhenThen);
        patterns.insert("acceptance_criteria".to_string(), TestPattern::AcceptanceCriteria);
        patterns.insert("examples".to_string(), TestPattern::Examples);
        
        Self { patterns }
    }
    
    pub fn extract_requirements(&self, spec: &Specification) -> Result<Vec<TestRequirement>> {
        let mut requirements = Vec::new();
        
        // Extract from requirements
        for req in &spec.requirements {
            let test_req = self.convert_requirement(req)?;
            requirements.push(test_req);
        }
        
        // Extract from acceptance criteria if present
        // TODO: Add metadata field to Specification if needed
        
        Ok(requirements)
    }
    
    fn convert_requirement(&self, req: &Requirement) -> Result<TestRequirement> {
        Ok(TestRequirement {
            id: req.id.clone(),
            description: req.description.clone(),
            requirement_type: self.classify_requirement(req),
            parameters: self.extract_parameters(&req.description),
            expected_behavior: req.description.clone(),
            components: self.identify_components(&req.description),
        })
    }
    
    fn classify_requirement(&self, req: &Requirement) -> RequirementType {
        let desc_lower = req.description.to_lowercase();
        
        if desc_lower.contains("performance") || desc_lower.contains("speed") {
            RequirementType::Performance
        } else if desc_lower.contains("security") || desc_lower.contains("auth") {
            RequirementType::Security
        } else if desc_lower.contains("integrate") || desc_lower.contains("api") {
            RequirementType::Integration
        } else {
            RequirementType::Functional
        }
    }
    
    fn extract_parameters(&self, description: &str) -> Vec<Parameter> {
        // Simple parameter extraction - could be enhanced with NLP
        let mut params = Vec::new();
        
        if description.contains("email") {
            params.push(Parameter {
                name: "email".to_string(),
                param_type: "string".to_string(),
                constraints: vec!["contains @".to_string()],
            });
        }
        
        if description.contains("password") {
            params.push(Parameter {
                name: "password".to_string(),
                param_type: "string".to_string(),
                constraints: vec!["length >= 8".to_string()],
            });
        }
        
        params
    }
    
    fn identify_components(&self, description: &str) -> Vec<String> {
        let mut components = Vec::new();
        let desc_lower = description.to_lowercase();
        
        if desc_lower.contains("database") || desc_lower.contains("storage") {
            components.push("database".to_string());
        }
        if desc_lower.contains("api") || desc_lower.contains("endpoint") {
            components.push("api".to_string());
        }
        if desc_lower.contains("auth") || desc_lower.contains("login") {
            components.push("authentication".to_string());
        }
        
        components
    }
    
    fn extract_from_criteria(&self, criteria: &str) -> Result<Vec<TestRequirement>> {
        let mut requirements = Vec::new();
        
        // Parse Given-When-Then patterns
        let lines: Vec<&str> = criteria.lines().collect();
        for line in lines {
            if line.trim().starts_with("Given") || 
               line.trim().starts_with("When") || 
               line.trim().starts_with("Then") {
                // Extract as test requirement
                let req = TestRequirement {
                    id: format!("criteria_{}", requirements.len()),
                    description: line.trim().to_string(),
                    requirement_type: RequirementType::Acceptance,
                    parameters: Vec::new(),
                    expected_behavior: line.trim().to_string(),
                    components: Vec::new(),
                };
                requirements.push(req);
            }
        }
        
        Ok(requirements)
    }
}

/// Builds test cases
pub struct TestBuilder {
    templates: HashMap<TestType, String>,
}

impl TestBuilder {
    pub fn new() -> Self {
        let mut templates = HashMap::new();
        
        // Add test templates for different types
        templates.insert(TestType::Unit, "unit_test_template".to_string());
        templates.insert(TestType::Integration, "integration_test_template".to_string());
        templates.insert(TestType::Property, "property_test_template".to_string());
        
        Self { templates }
    }
    
    pub async fn build_unit_test(
        &self, 
        req: &TestRequirement,
        _llm: Option<&dyn LLMProvider>,
    ) -> Result<TestCase> {
        let mut test = TestCase::new(
            format!("test_{}", req.id),
            TestType::Unit,
        );
        
        test.description = format!("Unit test for: {}", req.description);
        
        // Add assertions based on requirement
        let assertion = Assertion {
            assertion_type: super::AssertionType::Equals,
            expected: serde_json::json!(true),
            actual: "result".to_string(),
            message: Some(format!("Requirement {} should be satisfied", req.id)),
        };
        test.add_assertion(assertion);
        
        Ok(test)
    }
    
    pub fn build_property_test(&self, property: &Property) -> Result<TestCase> {
        let mut test = TestCase::new(
            format!("prop_{}", property.name),
            TestType::Property,
        );
        
        test.description = format!("Property test: {}", property.invariant.description);
        test.properties.push(property.clone());
        
        Ok(test)
    }
    
    pub fn build_edge_case_test(
        &self,
        req: &TestRequirement,
        edge_case: &TestInput,
    ) -> Result<TestCase> {
        let mut test = TestCase::new(
            format!("test_edge_{}", req.id),
            TestType::Unit,
        );
        
        test.description = format!("Edge case test for: {}", req.description);
        test.add_input(edge_case.clone());
        
        Ok(test)
    }
    
    pub async fn build_integration_test(
        &self,
        req: &TestRequirement,
        _llm: Option<&dyn LLMProvider>,
    ) -> Result<TestCase> {
        let mut test = TestCase::new(
            format!("test_integration_{}", req.id),
            TestType::Integration,
        );
        
        test.description = format!("Integration test for: {}", req.description);
        
        Ok(test)
    }
}

/// Generates property-based tests
pub struct PropertyGenerator;

impl PropertyGenerator {
    pub fn new() -> Self {
        Self
    }
    
    pub fn detect_properties(&self, req: &TestRequirement) -> Option<Vec<Property>> {
        let mut properties = Vec::new();
        
        // Detect common properties
        if req.description.contains("hash") && req.description.contains("password") {
            properties.push(Property {
                name: "password_hash_security".to_string(),
                generator: super::PropertyGenerator {
                    generator_type: super::GeneratorType::String { min_len: 8, max_len: 128 },
                    constraints: HashMap::new(),
                },
                invariant: super::Invariant {
                    description: "Hash should never equal original password".to_string(),
                    condition: "hash(pwd) != pwd".to_string(),
                },
                examples: Vec::new(),
            });
        }
        
        if !properties.is_empty() {
            Some(properties)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestRequirement {
    pub id: String,
    pub description: String,
    pub requirement_type: RequirementType,
    pub parameters: Vec<Parameter>,
    pub expected_behavior: String,
    pub components: Vec<String>,
}

impl TestRequirement {
    pub fn involves_multiple_components(&self) -> bool {
        self.components.len() > 1
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RequirementType {
    Functional,
    Performance,
    Security,
    Integration,
    Acceptance,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub param_type: String,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum TestPattern {
    GivenWhenThen,
    AcceptanceCriteria,
    Examples,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::model::{Specification, Requirement, Priority};

    #[test]
    fn test_spec_analyzer_creation() {
        let analyzer = SpecAnalyzer::new();
        assert!(!analyzer.patterns.is_empty());
    }

    #[test]
    fn test_parameter_extraction() {
        let analyzer = SpecAnalyzer::new();
        let params = analyzer.extract_parameters("User login with email and password");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "email");
        assert_eq!(params[1].name, "password");
    }

    #[test]
    fn test_component_identification() {
        let analyzer = SpecAnalyzer::new();
        let components = analyzer.identify_components("API endpoint for database authentication");
        assert!(components.contains(&"database".to_string()));
        assert!(components.contains(&"api".to_string()));
        assert!(components.contains(&"authentication".to_string()));
    }

    #[test]
    fn test_requirement_classification() {
        let analyzer = SpecAnalyzer::new();
        
        let security_req = Requirement {
            id: "REQ-1".to_string(),
            description: "Secure authentication required".to_string(),
            priority: Priority::High,
            category: crate::parser::model::RequirementType::Security,
            acceptance_criteria: Vec::new(),
            source_location: Default::default(),
            related: Vec::new(),
            tags: Vec::new(),
        };
        
        let req_type = analyzer.classify_requirement(&security_req);
        assert_eq!(req_type, RequirementType::Security);
    }

    #[test]
    fn test_property_detection() {
        let property_gen = PropertyGenerator::new();
        let req = TestRequirement {
            id: "REQ-1".to_string(),
            description: "Password hash should be secure".to_string(),
            requirement_type: RequirementType::Security,
            parameters: Vec::new(),
            expected_behavior: String::new(),
            components: Vec::new(),
        };
        
        let properties = property_gen.detect_properties(&req);
        assert!(properties.is_some());
        assert_eq!(properties.unwrap().len(), 1);
    }
}