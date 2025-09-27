//! Edge case test generation strategy

use super::{TestContext, TestStrategy};
use crate::test_gen::{Assertion, AssertionType, ExpectedOutcome, TestCase, TestInput, TestType};
use anyhow::Result;

/// Strategy for generating edge case tests
pub struct EdgeCaseStrategy {
    include_nulls: bool,
    include_bounds: bool,
    include_special_chars: bool,
    include_security: bool,
}

impl EdgeCaseStrategy {
    pub fn new() -> Self {
        Self {
            include_nulls: true,
            include_bounds: true,
            include_special_chars: true,
            include_security: true,
        }
    }

    fn generate_string_edge_cases(&self) -> Vec<TestInput> {
        let mut cases = Vec::new();

        // Basic edge cases
        cases.push(TestInput::Empty);
        cases.push(TestInput::Whitespace);

        // Special characters
        if self.include_special_chars {
            cases.push(TestInput::Unicode("🎉🎊😀".to_string()));
            cases.push(TestInput::Unicode("مرحبا".to_string())); // Arabic
            cases.push(TestInput::Unicode("你好".to_string())); // Chinese
            cases.push(TestInput::Unicode("\0\n\r\t".to_string())); // Control chars
        }

        // Boundary cases
        if self.include_bounds {
            cases.push(TestInput::VeryLong(10000));
            cases.push(TestInput::VeryLong(1_000_000));
        }

        // Security cases
        if self.include_security {
            cases.push(TestInput::SqlInjection("'; DROP TABLE users; --".to_string()));
            cases.push(TestInput::SqlInjection("' OR '1'='1".to_string()));
            cases.push(TestInput::Unicode("<script>alert('XSS')</script>".to_string()));
            cases.push(TestInput::Unicode("../../../etc/passwd".to_string()));
            cases.push(TestInput::Unicode("%00".to_string())); // Null byte
        }

        cases
    }

    fn generate_number_edge_cases(&self) -> Vec<TestInput> {
        let mut cases = Vec::new();

        // Basic edge cases
        cases.push(TestInput::Zero);

        if self.include_bounds {
            cases.push(TestInput::MaxValue);
            cases.push(TestInput::MinValue);
            cases.push(TestInput::Negative);
        }

        // Special values
        cases.push(TestInput::NaN);
        cases.push(TestInput::Custom(serde_json::Number::from_f64(f64::INFINITY).unwrap().to_value()));
        cases.push(TestInput::Custom(serde_json::Number::from_f64(f64::NEG_INFINITY).unwrap().to_value()));

        cases
    }

    fn generate_array_edge_cases(&self) -> Vec<TestInput> {
        let mut cases = Vec::new();

        // Basic cases
        cases.push(TestInput::EmptyArray);
        cases.push(TestInput::SingleElement);

        // Special cases
        cases.push(TestInput::Duplicates);

        if self.include_bounds {
            cases.push(TestInput::LargeArray(10000));
            cases.push(TestInput::LargeArray(1_000_000));
        }

        // Nested arrays
        cases.push(TestInput::Custom(serde_json::json!([[], [[]]])));

        // Mixed types (if applicable)
        cases.push(TestInput::Custom(serde_json::json!([1, "string", null, true])));

        cases
    }

    fn generate_object_edge_cases(&self) -> Vec<TestInput> {
        let mut cases = Vec::new();

        // Empty object
        cases.push(TestInput::Custom(serde_json::json!({})));

        // Null values
        if self.include_nulls {
            cases.push(TestInput::Custom(serde_json::json!({"key": null})));
        }

        // Deeply nested
        cases.push(TestInput::Custom(serde_json::json!({
            "{\"a\":{\"b\":{\"c\":{\"d\":{\"e\":\"deep\"}}}}}"
        })));

        // Circular reference simulation
        cases.push(TestInput::Custom(serde_json::json!({"self": "[Circular]"})));

        // Large object
        if self.include_bounds {
            let large_obj = format!(
                "{{{}}}",
                (0..1000)
                    .map(|i| format!("\"key{}\": \"value{}\"", i, i))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            cases.push(TestInput::Custom(serde_json::json!(large_obj)));
        }

        cases
    }

    fn create_edge_case_test(
        &self,
        context: &TestContext,
        edge_case: TestInput,
        index: usize,
    ) -> TestCase {
        let edge_case_name = match &edge_case {
            TestInput::Empty => "empty",
            TestInput::Whitespace => "whitespace",
            TestInput::Unicode(_) => "unicode",
            TestInput::VeryLong(_) => "very_long",
            TestInput::SqlInjection(_) => "sql_injection",
            TestInput::Zero => "zero",
            TestInput::Negative => "negative",
            TestInput::MaxValue => "max_value",
            TestInput::MinValue => "min_value",
            TestInput::NaN => "nan",
            TestInput::EmptyArray => "empty_array",
            TestInput::SingleElement => "single_element",
            TestInput::Duplicates => "duplicates",
            TestInput::LargeArray(_) => "large_array",
            TestInput::Custom(_) => "custom",
        };

        let mut test = TestCase::new(
            format!("test_{}_{}_edge_{}", context.function_name, edge_case_name, index),
            TestType::Unit,
        );

        test.description =
            format!("Edge case test for {}: {}", context.function_name, edge_case_name);

        test.inputs.push(edge_case);

        // Most edge cases should be handled gracefully
        test.expected = ExpectedOutcome {
            success: false, // Expect validation to catch edge cases
            value: None,
            error: Some("Edge case handled".to_string()),
        };

        // Add assertion for graceful handling
        test.assertions.push(Assertion {
            assertion_type: AssertionType::DoesNotThrow,
            expected: serde_json::json!(true),
            actual: "function_call".to_string(),
            message: Some(format!("Should handle {} gracefully", edge_case_name)),
        });

        test
    }
}

impl TestStrategy for EdgeCaseStrategy {
    fn generate(&self, context: &TestContext) -> Result<Vec<TestCase>> {
        let mut tests = Vec::new();
        let mut test_index = 0;

        for param in &context.parameters {
            let edge_cases = match param.param_type.as_str() {
                "string" | "String" | "&str" => self.generate_string_edge_cases(),
                "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "number" => {
                    self.generate_number_edge_cases()
                }
                "Vec" | "array" | "[]" => self.generate_array_edge_cases(),
                "object" | "HashMap" | "BTreeMap" => self.generate_object_edge_cases(),
                _ => Vec::new(),
            };

            for edge_case in edge_cases {
                tests.push(self.create_edge_case_test(context, edge_case, test_index));
                test_index += 1;
            }
        }

        Ok(tests)
    }

    fn test_type(&self) -> TestType {
        TestType::Unit
    }

    fn applies_to(&self, context: &TestContext) -> bool {
        // Edge case testing applies to all functions with parameters
        !context.parameters.is_empty()
    }
}

impl Default for EdgeCaseStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_gen::strategies::ParameterInfo;

    #[test]
    fn test_edge_case_strategy_creation() {
        let strategy = EdgeCaseStrategy::new();
        assert!(strategy.include_nulls);
        assert!(strategy.include_bounds);
        assert!(strategy.include_special_chars);
        assert!(strategy.include_security);
    }

    #[test]
    fn test_string_edge_cases() {
        let strategy = EdgeCaseStrategy::new();
        let cases = strategy.generate_string_edge_cases();

        assert!(cases.iter().any(|c| matches!(c, TestInput::Empty)));
        assert!(cases.iter().any(|c| matches!(c, TestInput::Whitespace)));
        assert!(cases.iter().any(|c| matches!(c, TestInput::Unicode(_))));
        assert!(cases.iter().any(|c| matches!(c, TestInput::SqlInjection(_))));
    }

    #[test]
    fn test_number_edge_cases() {
        let strategy = EdgeCaseStrategy::new();
        let cases = strategy.generate_number_edge_cases();

        assert!(cases.iter().any(|c| matches!(c, TestInput::Zero)));
        assert!(cases.iter().any(|c| matches!(c, TestInput::MaxValue)));
        assert!(cases.iter().any(|c| matches!(c, TestInput::MinValue)));
        assert!(cases.iter().any(|c| matches!(c, TestInput::NaN)));
    }

    #[test]
    fn test_edge_case_generation() {
        let strategy = EdgeCaseStrategy::new();
        let context = TestContext::new("validate_email").with_parameter(ParameterInfo {
            name: "email".to_string(),
            param_type: "string".to_string(),
            nullable: false,
            default_value: None,
            constraints: Vec::new(),
        });

        let tests = strategy.generate(&context).unwrap();
        assert!(!tests.is_empty());

        // Should have various edge cases for string parameter
        assert!(tests.iter().any(|t| t.name.contains("empty")));
        assert!(tests.iter().any(|t| t.name.contains("sql_injection")));
    }

    #[test]
    fn test_applies_to_functions_with_params() {
        let strategy = EdgeCaseStrategy::new();

        let context_with_params = TestContext::new("function").with_parameter(ParameterInfo {
            name: "param".to_string(),
            param_type: "string".to_string(),
            nullable: false,
            default_value: None,
            constraints: Vec::new(),
        });
        assert!(strategy.applies_to(&context_with_params));

        let context_without_params = TestContext::new("function");
        assert!(!strategy.applies_to(&context_without_params));
    }
}
