//! Test generation strategies for different types of tests

pub mod edge_case;
pub mod integration;
pub mod property;
pub mod unit;

pub use edge_case::EdgeCaseStrategy;
pub use integration::IntegrationTestStrategy;
pub use property::PropertyTestStrategy;
pub use unit::UnitTestStrategy;

use crate::test_gen::{TestCase, TestType};
use anyhow::Result;

/// Trait for test generation strategies
pub trait TestStrategy {
    /// Generate tests using this strategy
    fn generate(&self, context: &TestContext) -> Result<Vec<TestCase>>;

    /// Get the test type this strategy generates
    fn test_type(&self) -> TestType;

    /// Check if this strategy applies to the given context
    fn applies_to(&self, context: &TestContext) -> bool;
}

/// Context for test generation
#[derive(Debug, Clone)]
pub struct TestContext {
    pub function_name: String,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: String,
    pub description: String,
    pub constraints: Vec<String>,
    pub examples: Vec<Example>,
}

#[derive(Debug, Clone)]
pub struct ParameterInfo {
    pub name: String,
    pub param_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Example {
    pub input: serde_json::Value,
    pub output: serde_json::Value,
    pub description: Option<String>,
}

impl TestContext {
    pub fn new(function_name: impl Into<String>) -> Self {
        Self {
            function_name: function_name.into(),
            parameters: Vec::new(),
            return_type: String::new(),
            description: String::new(),
            constraints: Vec::new(),
            examples: Vec::new(),
        }
    }

    pub fn with_parameter(mut self, param: ParameterInfo) -> Self {
        self.parameters.push(param);
        self
    }

    pub fn with_return_type(mut self, return_type: impl Into<String>) -> Self {
        self.return_type = return_type.into();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let context = TestContext::new("calculate_sum").with_return_type("i32").with_parameter(
            ParameterInfo {
                name: "a".to_string(),
                param_type: "i32".to_string(),
                nullable: false,
                default_value: None,
                constraints: Vec::new(),
            },
        );

        assert_eq!(context.function_name, "calculate_sum");
        assert_eq!(context.return_type, "i32");
        assert_eq!(context.parameters.len(), 1);
    }
}
