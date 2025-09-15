//! Test generation module for automatically creating comprehensive test suites from specifications

pub mod generator;
pub mod strategies;
pub mod frameworks;
pub mod coverage;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

pub use generator::TestGenerator;
pub use coverage::CoverageAnalyzer;

/// Represents a complete test suite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuite {
    pub name: String,
    pub tests: Vec<TestCase>,
    pub fixtures: Vec<Fixture>,
    pub setup: Option<SetupCode>,
    pub teardown: Option<TeardownCode>,
}

/// Individual test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub name: String,
    pub description: String,
    pub test_type: TestType,
    pub inputs: Vec<TestInput>,
    pub expected: ExpectedOutcome,
    pub assertions: Vec<Assertion>,
    pub properties: Vec<Property>,
}

/// Type of test
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TestType {
    Unit,
    Integration,
    Property,
    Acceptance,
    Performance,
    Security,
}

/// Test input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestInput {
    Empty,
    Whitespace,
    Unicode(String),
    VeryLong(usize),
    SqlInjection(String),
    Zero,
    Negative,
    MaxValue,
    MinValue,
    NaN,
    EmptyArray,
    SingleElement,
    Duplicates,
    LargeArray(usize),
    Custom(String),
}

/// Expected outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedOutcome {
    pub success: bool,
    pub value: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Test assertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assertion {
    pub assertion_type: AssertionType,
    pub expected: serde_json::Value,
    pub actual: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssertionType {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    GreaterThan,
    LessThan,
    Throws,
    DoesNotThrow,
    Matches,
}

/// Property for property-based testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub generator: PropertyGenerator,
    pub invariant: Invariant,
    pub examples: Vec<Example>,
}

/// Property generator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyGenerator {
    pub generator_type: GeneratorType,
    pub constraints: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeneratorType {
    String { min_len: usize, max_len: usize },
    Number { min: i64, max: i64 },
    Array { min_size: usize, max_size: usize },
    Object,
    Custom(String),
}

/// Invariant to check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invariant {
    pub description: String,
    pub condition: String,
}

/// Example for property
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    pub input: serde_json::Value,
    pub expected: serde_json::Value,
}

/// Test fixture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixture {
    pub name: String,
    pub setup: String,
    pub teardown: Option<String>,
}

/// Setup code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupCode {
    pub code: String,
    pub language: String,
}

/// Teardown code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeardownCode {
    pub code: String,
    pub language: String,
}

/// Test quality metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestQualityMetrics {
    pub readability_score: f32,
    pub maintainability_index: f32,
    pub assertion_density: f32,
    pub setup_complexity: f32,
    pub execution_time: Duration,
}

impl TestSuite {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tests: Vec::new(),
            fixtures: Vec::new(),
            setup: None,
            teardown: None,
        }
    }

    pub fn add_test(&mut self, test: TestCase) {
        self.tests.push(test);
    }

    pub fn add_fixture(&mut self, fixture: Fixture) {
        self.fixtures.push(fixture);
    }
}

impl TestCase {
    pub fn new(name: impl Into<String>, test_type: TestType) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            test_type,
            inputs: Vec::new(),
            expected: ExpectedOutcome {
                success: true,
                value: None,
                error: None,
            },
            assertions: Vec::new(),
            properties: Vec::new(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn add_input(&mut self, input: TestInput) {
        self.inputs.push(input);
    }

    pub fn add_assertion(&mut self, assertion: Assertion) {
        self.assertions.push(assertion);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_suite_creation() {
        let mut suite = TestSuite::new("auth_tests");
        assert_eq!(suite.name, "auth_tests");
        assert!(suite.tests.is_empty());

        let test = TestCase::new("test_valid_login", TestType::Unit)
            .with_description("Tests valid login returns token");
        suite.add_test(test);
        assert_eq!(suite.tests.len(), 1);
    }

    #[test]
    fn test_test_case_builder() {
        let mut test = TestCase::new("test_edge_case", TestType::Unit);
        test.add_input(TestInput::Empty);
        test.add_input(TestInput::SqlInjection("'; DROP TABLE--".to_string()));
        assert_eq!(test.inputs.len(), 2);
    }
}