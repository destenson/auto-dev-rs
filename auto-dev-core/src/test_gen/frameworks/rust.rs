//! Rust test framework adapter

use super::{TestFrameworkAdapter, codegen};
use crate::test_gen::{
    TestSuite, TestCase, TestType, Assertion, AssertionType, 
    Fixture, TestInput, Property
};
use anyhow::Result;

/// Adapter for generating Rust test code
pub struct RustTestAdapter {
    use_tokio: bool,
    use_proptest: bool,
    use_quickcheck: bool,
}

impl RustTestAdapter {
    pub fn new() -> Self {
        Self {
            use_tokio: false,
            use_proptest: true,
            use_quickcheck: false,
        }
    }
    
    pub fn with_tokio(mut self, use_tokio: bool) -> Self {
        self.use_tokio = use_tokio;
        self
    }
    
    fn generate_test_case(&self, test: &TestCase) -> String {
        let mut code = String::new();
        
        // Add test attribute
        let test_attr = match test.test_type {
            TestType::Unit | TestType::Integration => {
                if self.use_tokio {
                    "#[tokio::test]"
                } else {
                    "#[test]"
                }
            }
            TestType::Property if self.use_proptest => "#[proptest]",
            TestType::Property if self.use_quickcheck => "#[quickcheck]",
            _ => "#[test]",
        };
        
        code.push_str(&format!("{}\n", test_attr));
        
        // Add async if needed
        let fn_prefix = if self.use_tokio && test.test_type != TestType::Property {
            "async "
        } else {
            ""
        };
        
        code.push_str(&format!("{}fn {}() {{\n", fn_prefix, test.name));
        
        // Add test description as comment
        if !test.description.is_empty() {
            code.push_str(&format!("    // {}\n", test.description));
        }
        
        // Generate test body
        if test.test_type == TestType::Property && !test.properties.is_empty() {
            code.push_str(&self.generate_property_test_body(&test.properties[0]));
        } else {
            code.push_str(&self.generate_regular_test_body(test));
        }
        
        code.push_str("}\n");
        code
    }
    
    fn generate_regular_test_body(&self, test: &TestCase) -> String {
        let mut body = String::new();
        
        // Setup phase
        body.push_str("    // Arrange\n");
        for input in &test.inputs {
            body.push_str(&format!("    let input = {};\n", self.format_test_input(input)));
        }
        
        // Act phase
        body.push_str("\n    // Act\n");
        body.push_str("    let result = function_under_test(input);\n");
        
        // Assert phase
        body.push_str("\n    // Assert\n");
        for assertion in &test.assertions {
            body.push_str(&format!("    {}\n", self.generate_assertion(assertion)));
        }
        
        body
    }
    
    fn generate_property_test_body(&self, property: &Property) -> String {
        let mut body = String::new();
        
        if self.use_proptest {
            body.push_str("    proptest! {\n");
            body.push_str(&format!("        #[test]\n"));
            body.push_str(&format!("        fn {}(input in {}) {{\n", 
                property.name,
                self.format_generator(&property.generator.generator_type)
            ));
            body.push_str(&format!("            // {}\n", property.invariant.description));
            body.push_str(&format!("            assert!({});\n", property.invariant.condition));
            body.push_str("        }\n");
            body.push_str("    }\n");
        } else if self.use_quickcheck {
            body.push_str(&format!("    // Property: {}\n", property.invariant.description));
            body.push_str(&format!("    quickcheck! {{\n"));
            body.push_str(&format!("        fn prop(input: {}) -> bool {{\n", 
                self.get_quickcheck_type(&property.generator.generator_type)
            ));
            body.push_str(&format!("            {}\n", property.invariant.condition));
            body.push_str("        }\n");
            body.push_str("    }\n");
        }
        
        body
    }
    
    fn format_test_input(&self, input: &TestInput) -> String {
        match input {
            TestInput::Empty => r#""""#.to_string(),
            TestInput::Whitespace => r#""   ""#.to_string(),
            TestInput::Unicode(s) => format!(r#""{}""#, codegen::escape_string(s)),
            TestInput::VeryLong(len) => format!(r#""{}""#, "x".repeat(*len)),
            TestInput::SqlInjection(s) => format!(r#""{}""#, codegen::escape_string(s)),
            TestInput::Zero => "0".to_string(),
            TestInput::Negative => "-1".to_string(),
            TestInput::MaxValue => "i32::MAX".to_string(),
            TestInput::MinValue => "i32::MIN".to_string(),
            TestInput::NaN => "f64::NAN".to_string(),
            TestInput::EmptyArray => "vec![]".to_string(),
            TestInput::SingleElement => "vec![1]".to_string(),
            TestInput::Duplicates => "vec![1, 1, 2, 2, 3, 3]".to_string(),
            TestInput::LargeArray(size) => format!("vec![0; {}]", size),
            TestInput::Custom(s) => s.clone(),
        }
    }
    
    fn format_generator(&self, gen_type: &crate::test_gen::GeneratorType) -> String {
        use crate::test_gen::GeneratorType;
        
        match gen_type {
            GeneratorType::String { min_len, max_len } => {
                format!(r#""{{{}, {}}}"#, min_len, max_len)
            }
            GeneratorType::Number { min, max } => {
                format!("{}..{}", min, max)
            }
            GeneratorType::Array { min_size, max_size } => {
                format!("prop::collection::vec(any::<i32>(), {}..{})", min_size, max_size)
            }
            GeneratorType::Object => "any::<HashMap<String, String>>()".to_string(),
            GeneratorType::Custom(s) => s.clone(),
        }
    }
    
    fn get_quickcheck_type(&self, gen_type: &crate::test_gen::GeneratorType) -> &str {
        use crate::test_gen::GeneratorType;
        
        match gen_type {
            GeneratorType::String { .. } => "String",
            GeneratorType::Number { .. } => "i32",
            GeneratorType::Array { .. } => "Vec<i32>",
            GeneratorType::Object => "HashMap<String, String>",
            GeneratorType::Custom(_) => "Value",
        }
    }
}

impl TestFrameworkAdapter for RustTestAdapter {
    fn generate_test_file(&self, suite: &TestSuite) -> Result<String> {
        let mut file_content = String::new();
        
        // Add module documentation
        file_content.push_str(&format!("//! Test suite: {}\n\n", suite.name));
        
        // Add imports
        file_content.push_str("#[cfg(test)]\n");
        file_content.push_str("mod tests {\n");
        file_content.push_str("    use super::*;\n");
        
        if self.use_tokio {
            file_content.push_str("    use tokio;\n");
        }
        if self.use_proptest {
            file_content.push_str("    use proptest::prelude::*;\n");
        }
        if self.use_quickcheck {
            file_content.push_str("    use quickcheck::{quickcheck, TestResult};\n");
        }
        
        file_content.push_str("\n");
        
        // Add fixtures if any
        for fixture in &suite.fixtures {
            file_content.push_str(&self.generate_setup(fixture));
            file_content.push_str("\n");
        }
        
        // Add setup/teardown if present
        if let Some(setup) = &suite.setup {
            file_content.push_str(&format!("    // Setup\n"));
            file_content.push_str(&format!("    {}\n\n", codegen::indent(&setup.code, 4)));
        }
        
        // Add test cases
        for test in &suite.tests {
            file_content.push_str(&codegen::indent(&self.generate_test_case(test), 4));
            file_content.push_str("\n");
        }
        
        // Add teardown if present
        if let Some(teardown) = &suite.teardown {
            file_content.push_str(&format!("    // Teardown\n"));
            file_content.push_str(&format!("    {}\n", codegen::indent(&teardown.code, 4)));
        }
        
        file_content.push_str("}\n");
        
        Ok(file_content)
    }
    
    fn generate_assertion(&self, assertion: &Assertion) -> String {
        match assertion.assertion_type {
            AssertionType::Equals => {
                format!("assert_eq!({}, {});", assertion.actual, assertion.expected)
            }
            AssertionType::NotEquals => {
                format!("assert_ne!({}, {});", assertion.actual, assertion.expected)
            }
            AssertionType::Contains => {
                format!("assert!({}.contains(&{}));", assertion.actual, assertion.expected)
            }
            AssertionType::NotContains => {
                format!("assert!(!{}.contains(&{}));", assertion.actual, assertion.expected)
            }
            AssertionType::GreaterThan => {
                format!("assert!({} > {});", assertion.actual, assertion.expected)
            }
            AssertionType::LessThan => {
                format!("assert!({} < {});", assertion.actual, assertion.expected)
            }
            AssertionType::Throws => {
                format!("assert!({}.is_err());", assertion.actual)
            }
            AssertionType::DoesNotThrow => {
                format!("assert!({}.is_ok());", assertion.actual)
            }
            AssertionType::Matches => {
                format!(r#"assert!(Regex::new(r"{}").unwrap().is_match(&{}));"#, 
                    assertion.expected, assertion.actual)
            }
        }
    }
    
    fn generate_setup(&self, fixture: &Fixture) -> String {
        let mut code = String::new();
        code.push_str(&format!("    // Fixture: {}\n", fixture.name));
        code.push_str(&format!("    fn setup_{}() {{\n", fixture.name));
        code.push_str(&codegen::indent(&fixture.setup, 8));
        code.push_str("\n    }\n");
        code
    }
    
    fn generate_teardown(&self, fixture: &Fixture) -> String {
        if let Some(teardown) = &fixture.teardown {
            let mut code = String::new();
            code.push_str(&format!("    fn teardown_{}() {{\n", fixture.name));
            code.push_str(&codegen::indent(teardown, 8));
            code.push_str("\n    }\n");
            code
        } else {
            String::new()
        }
    }
    
    fn file_extension(&self) -> &str {
        "rs"
    }
    
    fn framework_name(&self) -> &str {
        "rust"
    }
}

impl Default for RustTestAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_gen::{TestCase, TestType};

    #[test]
    fn test_rust_adapter_creation() {
        let adapter = RustTestAdapter::new();
        assert!(!adapter.use_tokio);
        assert!(adapter.use_proptest);
        assert!(!adapter.use_quickcheck);
    }

    #[test]
    fn test_file_extension() {
        let adapter = RustTestAdapter::new();
        assert_eq!(adapter.file_extension(), "rs");
        assert_eq!(adapter.framework_name(), "rust");
    }

    #[test]
    fn test_assertion_generation() {
        let adapter = RustTestAdapter::new();
        
        let assertion = Assertion {
            assertion_type: AssertionType::Equals,
            expected: serde_json::json!(42),
            actual: "result".to_string(),
            message: None,
        };
        
        let code = adapter.generate_assertion(&assertion);
        assert_eq!(code, "assert_eq!(result, 42);");
    }

    #[test]
    fn test_simple_test_generation() {
        let adapter = RustTestAdapter::new();
        
        let mut suite = TestSuite::new("example_tests");
        let test = TestCase::new("test_example", TestType::Unit);
        suite.add_test(test);
        
        let result = adapter.generate_test_file(&suite).unwrap();
        assert!(result.contains("#[test]"));
        assert!(result.contains("fn test_example()"));
        assert!(result.contains("mod tests"));
    }

    #[test]
    fn test_async_test_generation() {
        let adapter = RustTestAdapter::new().with_tokio(true);
        
        let test = TestCase::new("test_async", TestType::Unit);
        let code = adapter.generate_test_case(&test);
        
        assert!(code.contains("#[tokio::test]"));
        assert!(code.contains("async fn test_async()"));
    }
}