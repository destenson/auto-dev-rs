//! Unit test generation strategy

use super::{TestStrategy, TestContext};
use crate::test_gen::{TestCase, TestType, TestInput, Assertion, AssertionType, ExpectedOutcome};
use anyhow::Result;

/// Strategy for generating unit tests
pub struct UnitTestStrategy {
    include_happy_path: bool,
    include_error_cases: bool,
    include_boundary_values: bool,
}

impl UnitTestStrategy {
    pub fn new() -> Self {
        Self {
            include_happy_path: true,
            include_error_cases: true,
            include_boundary_values: true,
        }
    }
    
    pub fn with_happy_path(mut self, include: bool) -> Self {
        self.include_happy_path = include;
        self
    }
    
    pub fn with_error_cases(mut self, include: bool) -> Self {
        self.include_error_cases = include;
        self
    }
    
    fn generate_happy_path_test(&self, context: &TestContext) -> TestCase {
        let mut test = TestCase::new(
            format!("test_{}_happy_path", context.function_name),
            TestType::Unit,
        );
        
        test.description = format!(
            "Test {} with valid inputs returns expected output",
            context.function_name
        );
        
        // Add example-based inputs if available
        if let Some(example) = context.examples.first() {
            test.inputs.push(TestInput::Custom(example.input.to_string()));
            test.expected = ExpectedOutcome {
                success: true,
                value: Some(example.output.clone()),
                error: None,
            };
        }
        
        // Add assertion
        test.assertions.push(Assertion {
            assertion_type: AssertionType::Equals,
            expected: serde_json::json!(true),
            actual: "result.is_ok()".to_string(),
            message: Some("Function should succeed with valid inputs".to_string()),
        });
        
        test
    }
    
    fn generate_error_case_test(&self, context: &TestContext) -> TestCase {
        let mut test = TestCase::new(
            format!("test_{}_error_case", context.function_name),
            TestType::Unit,
        );
        
        test.description = format!(
            "Test {} handles invalid inputs gracefully",
            context.function_name
        );
        
        // Add invalid input based on parameter types
        for param in &context.parameters {
            if param.param_type == "string" && !param.nullable {
                test.inputs.push(TestInput::Empty);
                break;
            } else if param.param_type == "number" {
                test.inputs.push(TestInput::NaN);
                break;
            }
        }
        
        test.expected = ExpectedOutcome {
            success: false,
            value: None,
            error: Some("Invalid input".to_string()),
        };
        
        test.assertions.push(Assertion {
            assertion_type: AssertionType::Equals,
            expected: serde_json::json!(false),
            actual: "result.is_ok()".to_string(),
            message: Some("Function should fail with invalid inputs".to_string()),
        });
        
        test
    }
    
    fn generate_boundary_test(&self, context: &TestContext) -> TestCase {
        let mut test = TestCase::new(
            format!("test_{}_boundary", context.function_name),
            TestType::Unit,
        );
        
        test.description = format!(
            "Test {} with boundary values",
            context.function_name
        );
        
        // Add boundary inputs based on parameter types
        for param in &context.parameters {
            match param.param_type.as_str() {
                "number" | "i32" | "u32" | "f64" => {
                    test.inputs.push(TestInput::Zero);
                    test.inputs.push(TestInput::MaxValue);
                    test.inputs.push(TestInput::MinValue);
                }
                "string" => {
                    test.inputs.push(TestInput::Empty);
                    test.inputs.push(TestInput::VeryLong(1000));
                }
                "array" | "vec" => {
                    test.inputs.push(TestInput::EmptyArray);
                    test.inputs.push(TestInput::SingleElement);
                }
                _ => {}
            }
        }
        
        test
    }
}

impl TestStrategy for UnitTestStrategy {
    fn generate(&self, context: &TestContext) -> Result<Vec<TestCase>> {
        let mut tests = Vec::new();
        
        if self.include_happy_path {
            tests.push(self.generate_happy_path_test(context));
        }
        
        if self.include_error_cases {
            tests.push(self.generate_error_case_test(context));
        }
        
        if self.include_boundary_values {
            tests.push(self.generate_boundary_test(context));
        }
        
        Ok(tests)
    }
    
    fn test_type(&self) -> TestType {
        TestType::Unit
    }
    
    fn applies_to(&self, context: &TestContext) -> bool {
        // Unit tests apply to all functions
        !context.function_name.is_empty()
    }
}

impl Default for UnitTestStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_gen::strategies::ParameterInfo;

    #[test]
    fn test_unit_strategy_creation() {
        let strategy = UnitTestStrategy::new();
        assert!(strategy.include_happy_path);
        assert!(strategy.include_error_cases);
        assert!(strategy.include_boundary_values);
    }

    #[test]
    fn test_happy_path_generation() {
        let strategy = UnitTestStrategy::new();
        let context = TestContext::new("add_numbers")
            .with_parameter(ParameterInfo {
                name: "a".to_string(),
                param_type: "i32".to_string(),
                nullable: false,
                default_value: None,
                constraints: Vec::new(),
            });
        
        let tests = strategy.generate(&context).unwrap();
        assert!(!tests.is_empty());
        
        let happy_path = tests.iter()
            .find(|t| t.name.contains("happy_path"))
            .expect("Should have happy path test");
        
        assert_eq!(happy_path.test_type, TestType::Unit);
    }

    #[test]
    fn test_applies_to_all_functions() {
        let strategy = UnitTestStrategy::new();
        let context = TestContext::new("any_function");
        assert!(strategy.applies_to(&context));
    }
}