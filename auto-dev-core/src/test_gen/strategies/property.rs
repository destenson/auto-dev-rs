//! Property-based test generation strategy

use super::{TestContext, TestStrategy};
use crate::test_gen::{
    Example, GeneratorType, Invariant, Property, PropertyGenerator as PropGen, TestCase, TestType,
};
use anyhow::Result;
use std::collections::HashMap;

/// Strategy for generating property-based tests
pub struct PropertyTestStrategy {
    max_examples: usize,
    shrinking_enabled: bool,
}

impl PropertyTestStrategy {
    pub fn new() -> Self {
        Self { max_examples: 100, shrinking_enabled: true }
    }

    pub fn with_max_examples(mut self, max: usize) -> Self {
        self.max_examples = max;
        self
    }

    fn detect_invariants(&self, context: &TestContext) -> Vec<Invariant> {
        let mut invariants = Vec::new();
        let fn_name = &context.function_name;

        // Common invariants based on function names and types
        if fn_name.contains("sort") {
            invariants.push(Invariant {
                description: "Output should be sorted".to_string(),
                condition: "is_sorted(output)".to_string(),
            });
            invariants.push(Invariant {
                description: "Output length equals input length".to_string(),
                condition: "output.len() == input.len()".to_string(),
            });
        }

        if fn_name.contains("reverse") {
            invariants.push(Invariant {
                description: "Reversing twice returns original".to_string(),
                condition: "reverse(reverse(x)) == x".to_string(),
            });
        }

        if fn_name.contains("hash") {
            invariants.push(Invariant {
                description: "Hash is deterministic".to_string(),
                condition: "hash(x) == hash(x)".to_string(),
            });

            if fn_name.contains("password") {
                invariants.push(Invariant {
                    description: "Hash never equals input".to_string(),
                    condition: "hash(x) != x".to_string(),
                });
            }
        }

        if fn_name.contains("encode") && fn_name.contains("decode") {
            invariants.push(Invariant {
                description: "Decode reverses encode".to_string(),
                condition: "decode(encode(x)) == x".to_string(),
            });
        }

        if fn_name.contains("add") || fn_name.contains("sum") {
            invariants.push(Invariant {
                description: "Addition is commutative".to_string(),
                condition: "add(a, b) == add(b, a)".to_string(),
            });
            invariants.push(Invariant {
                description: "Adding zero is identity".to_string(),
                condition: "add(x, 0) == x".to_string(),
            });
        }

        if fn_name.contains("multiply") {
            invariants.push(Invariant {
                description: "Multiplication is commutative".to_string(),
                condition: "multiply(a, b) == multiply(b, a)".to_string(),
            });
            invariants.push(Invariant {
                description: "Multiplying by one is identity".to_string(),
                condition: "multiply(x, 1) == x".to_string(),
            });
        }

        invariants
    }

    fn create_generator_for_type(&self, type_name: &str) -> GeneratorType {
        match type_name {
            "string" | "String" | "&str" => GeneratorType::String { min_len: 0, max_len: 100 },
            "i32" | "i64" | "u32" | "u64" | "usize" => {
                GeneratorType::Number { min: -1000, max: 1000 }
            }
            "Vec" | "array" | "[]" => GeneratorType::Array { min_size: 0, max_size: 50 },
            _ => GeneratorType::Custom(type_name.to_string()),
        }
    }

    fn generate_property_test(&self, context: &TestContext, invariant: Invariant) -> TestCase {
        let mut test = TestCase::new(
            format!(
                "prop_{}_{}",
                context.function_name,
                invariant.description.to_lowercase().replace(' ', "_")
            ),
            TestType::Property,
        );

        test.description =
            format!("Property test: {} - {}", context.function_name, invariant.description);

        // Create property with generators for each parameter
        let mut constraints = HashMap::new();
        constraints.insert("max_examples".to_string(), serde_json::json!(self.max_examples));

        let generator = if !context.parameters.is_empty() {
            self.create_generator_for_type(&context.parameters[0].param_type)
        } else {
            GeneratorType::Custom("unknown".to_string())
        };

        let property = Property {
            name: invariant.description.clone(),
            generator: PropGen { generator_type: generator, constraints },
            invariant,
            examples: self.generate_examples(context),
        };

        test.properties.push(property);
        test
    }

    fn generate_examples(&self, context: &TestContext) -> Vec<Example> {
        let mut examples = Vec::new();

        // Use provided examples if available
        for ctx_example in &context.examples {
            examples.push(Example {
                input: ctx_example.input.clone(),
                expected: ctx_example.output.clone(),
            });
        }

        // Generate common examples based on parameter types
        if examples.is_empty() {
            for param in &context.parameters {
                match param.param_type.as_str() {
                    "string" => {
                        examples.push(Example {
                            input: serde_json::json!(""),
                            expected: serde_json::json!(""),
                        });
                        examples.push(Example {
                            input: serde_json::json!("hello"),
                            expected: serde_json::json!("processed"),
                        });
                    }
                    "i32" | "number" => {
                        examples.push(Example {
                            input: serde_json::json!(0),
                            expected: serde_json::json!(0),
                        });
                        examples.push(Example {
                            input: serde_json::json!(42),
                            expected: serde_json::json!(42),
                        });
                    }
                    _ => {}
                }
            }
        }

        examples
    }
}

impl TestStrategy for PropertyTestStrategy {
    fn generate(&self, context: &TestContext) -> Result<Vec<TestCase>> {
        let mut tests = Vec::new();

        // Detect invariants for this function
        let invariants = self.detect_invariants(context);

        // Generate a property test for each invariant
        for invariant in invariants {
            tests.push(self.generate_property_test(context, invariant));
        }

        // If no invariants detected, try to generate generic properties
        if tests.is_empty() && self.applies_to(context) {
            let generic_invariant = Invariant {
                description: "Function doesn't panic".to_string(),
                condition: "no_panic()".to_string(),
            };
            tests.push(self.generate_property_test(context, generic_invariant));
        }

        Ok(tests)
    }

    fn test_type(&self) -> TestType {
        TestType::Property
    }

    fn applies_to(&self, context: &TestContext) -> bool {
        // Property tests are useful for pure functions with clear invariants
        let fn_name = &context.function_name;

        // Functions that typically have good properties
        fn_name.contains("sort") ||
        fn_name.contains("reverse") ||
        fn_name.contains("hash") ||
        fn_name.contains("encode") ||
        fn_name.contains("decode") ||
        fn_name.contains("add") ||
        fn_name.contains("multiply") ||
        fn_name.contains("transform") ||
        fn_name.contains("parse") ||
        fn_name.contains("validate") ||
        // Or if constraints suggest properties
        !context.constraints.is_empty()
    }
}

impl Default for PropertyTestStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_gen::strategies::ParameterInfo;

    #[test]
    fn test_property_strategy_creation() {
        let strategy = PropertyTestStrategy::new();
        assert_eq!(strategy.max_examples, 100);
        assert!(strategy.shrinking_enabled);
    }

    #[test]
    fn test_invariant_detection_for_sort() {
        let strategy = PropertyTestStrategy::new();
        let context = TestContext::new("sort_array");

        let invariants = strategy.detect_invariants(&context);
        assert!(!invariants.is_empty());
        assert!(invariants.iter().any(|i| i.description.contains("sorted")));
        assert!(invariants.iter().any(|i| i.description.contains("length")));
    }

    #[test]
    fn test_invariant_detection_for_hash() {
        let strategy = PropertyTestStrategy::new();
        let context = TestContext::new("hash_password");

        let invariants = strategy.detect_invariants(&context);
        assert!(!invariants.is_empty());
        assert!(invariants.iter().any(|i| i.description.contains("deterministic")));
        assert!(invariants.iter().any(|i| i.description.contains("never equals")));
    }

    #[test]
    fn test_generator_creation() {
        let strategy = PropertyTestStrategy::new();

        match strategy.create_generator_for_type("string") {
            GeneratorType::String { min_len, max_len } => {
                assert_eq!(min_len, 0);
                assert_eq!(max_len, 100);
            }
            _ => panic!("Wrong generator type"),
        }

        match strategy.create_generator_for_type("i32") {
            GeneratorType::Number { min, max } => {
                assert_eq!(min, -1000);
                assert_eq!(max, 1000);
            }
            _ => panic!("Wrong generator type"),
        }
    }

    #[test]
    fn test_applies_to_appropriate_functions() {
        let strategy = PropertyTestStrategy::new();

        assert!(strategy.applies_to(&TestContext::new("sort_list")));
        assert!(strategy.applies_to(&TestContext::new("hash_data")));
        assert!(strategy.applies_to(&TestContext::new("encode_string")));
        assert!(!strategy.applies_to(&TestContext::new("print_message")));
    }
}
