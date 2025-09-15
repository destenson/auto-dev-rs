use auto_dev_core::llm::cli_tools::FabricProvider;
use auto_dev_core::llm::provider::{LLMProvider, Specification, ProjectContext, GenerationOptions};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let fabric = FabricProvider::new().await;
    
    if !fabric.is_available().await {
        println!("Fabric not available");
        return Ok(());
    }
    
    println!("Testing Fabric integration...\n");
    
    // Test simple question
    if let Ok(Some(answer)) = fabric.answer_question("What is a REST API?").await {
        println!("Q: What is a REST API?");
        println!("A: {}\n", answer.lines().take(3).collect::<Vec<_>>().join("\n"));
    }
    
    // Test code generation
    let spec = Specification {
        content: "Create a function that validates email addresses".to_string(),
        requirements: vec!["Must check for @ symbol".to_string(), "Must check for domain".to_string()],
        examples: vec![],
        acceptance_criteria: vec![],
    };
    
    let context = ProjectContext {
        language: "rust".to_string(),
        framework: None,
        dependencies: vec![],
        existing_files: vec![],
        patterns: vec![],
    };
    
    let options = GenerationOptions::default();
    
    if let Ok(code) = fabric.generate_code(&spec, &context, &options).await {
        println!("Generated code:");
        println!("{}", code.files[0].content.lines().take(10).collect::<Vec<_>>().join("\n"));
    }
    
    Ok(())
}