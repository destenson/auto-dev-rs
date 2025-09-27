#![allow(unused)]
//! Test framework adapters for generating framework-specific test code

pub mod javascript;
pub mod python;
pub mod rust;

use crate::test_gen::{Assertion, Fixture, TestCase, TestSuite};
use anyhow::Result;

/// Trait for test framework adapters
pub trait TestFrameworkAdapter {
    /// Generate test file content from a test suite
    fn generate_test_file(&self, suite: &TestSuite) -> Result<String>;

    /// Generate assertion code for the framework
    fn generate_assertion(&self, assertion: &Assertion) -> String;

    /// Generate setup code for the framework
    fn generate_setup(&self, fixture: &Fixture) -> String;

    /// Generate teardown code for the framework
    fn generate_teardown(&self, fixture: &Fixture) -> String;

    /// Get the file extension for test files
    fn file_extension(&self) -> &str;

    /// Get the framework name
    fn framework_name(&self) -> &str;
}

/// Registry of available framework adapters
pub struct FrameworkRegistry {
    adapters: Vec<Box<dyn TestFrameworkAdapter>>,
}

impl FrameworkRegistry {
    pub fn new() -> Self {
        Self {
            adapters: vec![
                Box::new(rust::RustTestAdapter::new()),
                Box::new(python::PytestAdapter::new()),
                Box::new(javascript::JestAdapter::new()),
            ],
        }
    }

    /// Get adapter by language/framework name
    pub fn get_adapter(&self, name: &str) -> Option<&dyn TestFrameworkAdapter> {
        self.adapters
            .iter()
            .find(|a| a.framework_name().eq_ignore_ascii_case(name))
            .map(|a| a.as_ref())
    }

    /// List all available frameworks
    pub fn list_frameworks(&self) -> Vec<String> {
        self.adapters.iter().map(|a| a.framework_name().to_string()).collect()
    }
}

impl Default for FrameworkRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for code generation
pub mod codegen {
    /// Indent a block of code
    pub fn indent(code: &str, spaces: usize) -> String {
        let indent_str = " ".repeat(spaces);
        code.lines()
            .map(
                |line| {
                    if line.is_empty() {
                        line.to_string()
                    } else {
                        format!("{}{}", indent_str, line)
                    }
                },
            )
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Convert snake_case to camelCase
    pub fn to_camel_case(s: &str) -> String {
        let mut result = String::new();
        let mut capitalize_next = false;

        for c in s.chars() {
            if c == '_' {
                capitalize_next = true;
            } else if capitalize_next {
                result.push(c.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Convert snake_case to PascalCase
    pub fn to_pascal_case(s: &str) -> String {
        let camel = to_camel_case(s);
        if let Some(first) = camel.chars().next() {
            first.to_ascii_uppercase().to_string() + &camel[1..]
        } else {
            camel
        }
    }

    /// Escape string for inclusion in code
    pub fn escape_string(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_registry() {
        let registry = FrameworkRegistry::new();

        let frameworks = registry.list_frameworks();
        assert!(frameworks.contains(&"rust".to_string()));
        assert!(frameworks.contains(&"pytest".to_string()));
        assert!(frameworks.contains(&"jest".to_string()));

        assert!(registry.get_adapter("rust").is_some());
        assert!(registry.get_adapter("RUST").is_some()); // Case insensitive
        assert!(registry.get_adapter("unknown").is_none());
    }

    #[test]
    fn test_codegen_helpers() {
        use codegen::*;

        // Test indentation
        let code = "line1\nline2\nline3";
        let indented = indent(code, 4);
        assert_eq!(indented, "    line1\n    line2\n    line3");

        // Test camelCase conversion
        assert_eq!(to_camel_case("snake_case_name"), "snakeCaseName");
        assert_eq!(to_camel_case("already_camel"), "alreadyCamel");

        // Test PascalCase conversion
        assert_eq!(to_pascal_case("snake_case_name"), "SnakeCaseName");

        // Test string escaping
        assert_eq!(escape_string("Hello \"World\""), "Hello \\\"World\\\"");
        assert_eq!(escape_string("Line1\nLine2"), "Line1\\nLine2");
    }
}
