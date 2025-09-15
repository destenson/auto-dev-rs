// Example Module Implementation
//
// This example shows how to create a module that can be loaded by the module system

use auto_dev_core::modules::interface::{
    ModuleInterface, ModuleMetadata, ModuleCapability, ModuleState, ModuleVersion, ModuleDependency
};
use async_trait::async_trait;
use serde_json::Value;
use anyhow::Result;
use std::collections::HashMap;

/// Example language parser module
pub struct ExampleParserModule {
    metadata: ModuleMetadata,
    state: ModuleState,
    parse_count: u64,
}

impl ExampleParserModule {
    pub fn new() -> Self {
        let metadata = ModuleMetadata {
            name: "example_parser".to_string(),
            version: ModuleVersion::new(1, 0, 0),
            author: "auto-dev".to_string(),
            description: "Example parser module for Python code".to_string(),
            capabilities: vec![
                ModuleCapability::Parser { 
                    language: "python".to_string() 
                },
            ],
            dependencies: vec![],
        };

        let state = ModuleState::new(metadata.version.clone());

        Self {
            metadata,
            state,
            parse_count: 0,
        }
    }

    fn parse_python(&self, code: &str) -> Result<Value> {
        // Simple mock parser
        let lines: Vec<&str> = code.lines().collect();
        let functions: Vec<String> = lines
            .iter()
            .filter(|line| line.trim().starts_with("def "))
            .map(|line| {
                line.trim()
                    .strip_prefix("def ")
                    .unwrap_or("")
                    .split('(')
                    .next()
                    .unwrap_or("")
                    .to_string()
            })
            .collect();

        Ok(serde_json::json!({
            "language": "python",
            "line_count": lines.len(),
            "functions": functions,
            "parse_time_ms": 10,
        }))
    }
}

#[async_trait]
impl ModuleInterface for ExampleParserModule {
    fn metadata(&self) -> ModuleMetadata {
        self.metadata.clone()
    }

    async fn initialize(&mut self) -> Result<()> {
        println!("Example parser module initialized");
        Ok(())
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        if let Some(code) = input.get("code").and_then(|v| v.as_str()) {
            self.parse_python(code)
        } else {
            anyhow::bail!("Expected 'code' field in input")
        }
    }

    fn get_capabilities(&self) -> Vec<ModuleCapability> {
        self.metadata.capabilities.clone()
    }

    async fn handle_message(&mut self, message: Value) -> Result<Option<Value>> {
        if let Some(msg_type) = message.get("type").and_then(|v| v.as_str()) {
            match msg_type {
                "get_stats" => {
                    Ok(Some(serde_json::json!({
                        "parse_count": self.parse_count,
                    })))
                }
                "reset_stats" => {
                    self.parse_count = 0;
                    Ok(Some(serde_json::json!({
                        "status": "reset",
                    })))
                }
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    async fn shutdown(&mut self) -> Result<()> {
        println!("Example parser module shutting down");
        Ok(())
    }

    fn get_state(&self) -> Result<ModuleState> {
        let mut state = self.state.clone();
        state.set(
            "parse_count".to_string(),
            Value::Number(self.parse_count.into()),
        );
        Ok(state)
    }

    fn restore_state(&mut self, state: ModuleState) -> Result<()> {
        if let Some(count) = state.get("parse_count").and_then(|v| v.as_u64()) {
            self.parse_count = count;
        }
        self.state = state;
        Ok(())
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}

/// Example synthesis strategy module
pub struct ExampleSynthesisModule {
    metadata: ModuleMetadata,
    state: ModuleState,
}

impl ExampleSynthesisModule {
    pub fn new() -> Self {
        let metadata = ModuleMetadata {
            name: "example_synthesis".to_string(),
            version: ModuleVersion::new(1, 0, 0),
            author: "auto-dev".to_string(),
            description: "Example synthesis strategy module".to_string(),
            capabilities: vec![
                ModuleCapability::SynthesisStrategy { 
                    name: "template_based".to_string() 
                },
            ],
            dependencies: vec![
                ModuleDependency {
                    name: "example_parser".to_string(),
                    version_requirement: "1.0.0".to_string(),
                    optional: false,
                },
            ],
        };

        let state = ModuleState::new(metadata.version.clone());

        Self {
            metadata,
            state,
        }
    }
}

#[async_trait]
impl ModuleInterface for ExampleSynthesisModule {
    fn metadata(&self) -> ModuleMetadata {
        self.metadata.clone()
    }

    async fn initialize(&mut self) -> Result<()> {
        println!("Example synthesis module initialized");
        Ok(())
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        // Simple template-based code generation
        if let Some(template) = input.get("template").and_then(|v| v.as_str()) {
            let variables = input.get("variables")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();

            let mut result = template.to_string();
            for (key, value) in variables {
                let placeholder = format!("{{{{{}}}}}", key);
                let replacement = value.as_str().unwrap_or("");
                result = result.replace(&placeholder, replacement);
            }

            Ok(serde_json::json!({
                "generated_code": result,
                "strategy": "template_based",
            }))
        } else {
            anyhow::bail!("Expected 'template' field in input")
        }
    }

    fn get_capabilities(&self) -> Vec<ModuleCapability> {
        self.metadata.capabilities.clone()
    }

    async fn handle_message(&mut self, _message: Value) -> Result<Option<Value>> {
        Ok(None)
    }

    async fn shutdown(&mut self) -> Result<()> {
        println!("Example synthesis module shutting down");
        Ok(())
    }

    fn get_state(&self) -> Result<ModuleState> {
        Ok(self.state.clone())
    }

    fn restore_state(&mut self, state: ModuleState) -> Result<()> {
        self.state = state;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    use auto_dev_core::modules::{ModuleSystem, ModuleFormat, ExecutionContext};
    
    println!("Example Module System Demo");
    println!("==========================");

    // Create module system
    let module_system = ModuleSystem::new()?;

    // In a real scenario, modules would be loaded from files
    // For this example, we'll demonstrate the API usage
    
    println!("\nModule system created successfully!");
    
    // Example of how to use the module system
    let context = ExecutionContext::new(serde_json::json!({
        "code": "def hello():\n    print('Hello')\n\ndef world():\n    pass"
    }));
    
    println!("\nExample execution context created");
    
    // List modules (should be empty initially)
    let modules = module_system.list_modules().await?;
    println!("\nCurrently loaded modules: {}", modules.len());
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_example_parser_module() {
        let mut module = ExampleParserModule::new();
        
        // Test initialization
        assert!(module.initialize().await.is_ok());
        
        // Test parsing
        let input = serde_json::json!({
            "code": "def test_function():\n    pass"
        });
        
        let result = module.execute(input).await.unwrap();
        assert_eq!(result["language"], "python");
        assert_eq!(result["functions"][0], "test_function");
    }

    #[tokio::test]
    async fn test_example_synthesis_module() {
        let mut module = ExampleSynthesisModule::new();
        
        // Test initialization
        assert!(module.initialize().await.is_ok());
        
        // Test template-based generation
        let input = serde_json::json!({
            "template": "class {{name}} {\n    constructor() {\n        this.type = '{{type}}';\n    }\n}",
            "variables": {
                "name": "TestClass",
                "type": "example"
            }
        });
        
        let result = module.execute(input).await.unwrap();
        let generated = result["generated_code"].as_str().unwrap();
        
        assert!(generated.contains("class TestClass"));
        assert!(generated.contains("this.type = 'example'"));
    }

    #[tokio::test]
    async fn test_module_state_management() {
        let mut module = ExampleParserModule::new();
        
        // Initialize
        module.initialize().await.unwrap();
        
        // Modify state
        module.parse_count = 42;
        
        // Get state
        let state = module.get_state().unwrap();
        assert_eq!(
            state.get("parse_count").and_then(|v| v.as_u64()),
            Some(42)
        );
        
        // Create new module and restore state
        let mut new_module = ExampleParserModule::new();
        new_module.restore_state(state).unwrap();
        
        assert_eq!(new_module.parse_count, 42);
    }
}