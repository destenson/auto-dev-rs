//! Example generation for documentation

use anyhow::Result;
use std::path::Path;

use super::Example;

/// Generates usage examples for documentation
pub struct ExampleGenerator {
    language: String,
}

impl ExampleGenerator {
    /// Create new example generator
    pub fn new() -> Self {
        Self {
            language: "rust".to_string(),
        }
    }

    /// Generate example from function signature
    pub fn generate_from_signature(&self, signature: &str, name: &str) -> Result<Example> {
        let code = self.create_example_code(signature, name);
        
        Ok(Example {
            title: format!("Using {}", name),
            description: format!("Example usage of {}", name),
            code,
            output: None,
            language: self.language.clone(),
        })
    }

    /// Generate example from test code
    pub fn generate_from_test(&self, test_code: &str, test_name: &str) -> Result<Example> {
        // Extract the relevant parts of the test
        let cleaned_code = self.clean_test_code(test_code);
        
        Ok(Example {
            title: format!("Test: {}", test_name),
            description: "Example from test code".to_string(),
            code: cleaned_code,
            output: None,
            language: self.language.clone(),
        })
    }

    /// Generate examples for a module
    pub fn generate_module_examples(&self, module_path: &Path) -> Result<Vec<Example>> {
        let mut examples = Vec::new();
        
        // Basic usage example
        let module_name = module_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("module");
        
        examples.push(Example {
            title: "Basic Usage".to_string(),
            description: format!("How to use the {} module", module_name),
            code: format!(
                "use auto_dev::{};\n\nfn main() {{\n    // Your code here\n}}",
                module_name
            ),
            output: None,
            language: self.language.clone(),
        });
        
        Ok(examples)
    }

    /// Extract examples from doc comments
    pub fn extract_from_docs(&self, doc_content: &str) -> Vec<Example> {
        let mut examples = Vec::new();
        let mut in_example = false;
        let mut current_code = String::new();
        let mut example_count = 0;
        
        for line in doc_content.lines() {
            if line.contains("# Example") || line.contains("# Examples") {
                in_example = true;
                continue;
            }
            
            if in_example {
                if line.starts_with("```") {
                    if !current_code.is_empty() {
                        example_count += 1;
                        examples.push(Example {
                            title: format!("Example {}", example_count),
                            description: String::new(),
                            code: current_code.clone(),
                            output: None,
                            language: self.language.clone(),
                        });
                        current_code.clear();
                    }
                } else if line.starts_with("///") {
                    let content = line.trim_start_matches("///").trim();
                    if !content.is_empty() && !content.starts_with('#') {
                        current_code.push_str(content);
                        current_code.push('\n');
                    }
                }
            }
        }
        
        examples
    }

    fn create_example_code(&self, signature: &str, name: &str) -> String {
        // Parse signature to determine parameters
        let params = self.extract_parameters(signature);
        
        let mut code = String::new();
        
        // Add necessary imports
        code.push_str("use auto_dev_core::*;\n\n");
        
        // Create example based on function type
        if signature.contains("async") {
            code.push_str("#[tokio::main]\n");
            code.push_str("async fn main() -> Result<()> {\n");
            code.push_str(&format!("    let result = {}({}).await?;\n", name, params));
            code.push_str("    println!(\"Result: {:?}\", result);\n");
            code.push_str("    Ok(())\n");
            code.push_str("}\n");
        } else if signature.contains("->") && signature.contains("Result") {
            code.push_str("fn main() -> Result<()> {\n");
            code.push_str(&format!("    let result = {}({})?;\n", name, params));
            code.push_str("    println!(\"Result: {:?}\", result);\n");
            code.push_str("    Ok(())\n");
            code.push_str("}\n");
        } else {
            code.push_str("fn main() {\n");
            code.push_str(&format!("    {}({});\n", name, params));
            code.push_str("}\n");
        }
        
        code
    }

    fn extract_parameters(&self, signature: &str) -> String {
        // Simple parameter extraction (could be improved with proper parsing)
        if signature.contains("()") {
            return String::new();
        }
        
        // Generate example parameters based on types
        let mut params = Vec::new();
        
        if signature.contains("&str") {
            params.push("\"example\"");
        }
        if signature.contains("String") {
            params.push("String::from(\"example\")");
        }
        if signature.contains("usize") || signature.contains("u32") {
            params.push("42");
        }
        if signature.contains("bool") {
            params.push("true");
        }
        if signature.contains("Path") {
            params.push("Path::new(\"./example\")");
        }
        
        params.join(", ")
    }

    fn clean_test_code(&self, test_code: &str) -> String {
        let mut cleaned = String::new();
        let mut skip_assert = false;
        
        for line in test_code.lines() {
            // Skip test attributes and assertions for examples
            if line.trim().starts_with("#[test]") || line.trim().starts_with("#[tokio::test]") {
                continue;
            }
            
            if line.trim().starts_with("assert") {
                skip_assert = true;
            }
            
            if !skip_assert {
                cleaned.push_str(line);
                cleaned.push('\n');
            }
            
            if skip_assert && line.ends_with(';') {
                skip_assert = false;
            }
        }
        
        cleaned.trim().to_string()
    }
}

impl Default for ExampleGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_from_signature() {
        let generator = ExampleGenerator::new();
        let example = generator
            .generate_from_signature("pub fn process(input: &str) -> Result<String>", "process")
            .unwrap();
        
        assert!(example.code.contains("process("));
        assert_eq!(example.language, "rust");
    }

    #[test]
    fn test_extract_parameters() {
        let generator = ExampleGenerator::new();
        
        let params = generator.extract_parameters("fn test(s: &str, n: usize)");
        assert!(params.contains("\"example\""));
        assert!(params.contains("42"));
    }

    #[test]
    fn test_clean_test_code() {
        let generator = ExampleGenerator::new();
        
        let test_code = r#"
#[test]
fn test_example() {
    let value = compute();
    assert_eq!(value, 42);
}
"#;
        
        let cleaned = generator.clean_test_code(test_code);
        assert!(!cleaned.contains("#[test]"));
        assert!(!cleaned.contains("assert_eq"));
        assert!(cleaned.contains("let value = compute()"));
    }
}