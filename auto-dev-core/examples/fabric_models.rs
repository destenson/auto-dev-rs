use auto_dev_core::llm::cli_tools::FabricProvider;
use auto_dev_core::llm::provider::{LLMProvider, ModelTier};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Testing Fabric Model Discovery\n");
    
    let mut fabric = FabricProvider::new().await;
    
    if !fabric.is_available().await {
        println!("Fabric not available");
        return Ok(());
    }
    
    println!("Fabric is available!");
    println!("  Default tier: {:?}", fabric.tier());
    println!("  Default model: {:?}", fabric.get_current_model());
    println!("  Total models available: {}", fabric.get_models().len());
    
    // Show some available models
    println!("\nSample of available models:");
    for (i, model) in fabric.get_models().iter().take(10).enumerate() {
        println!("  {}. {}", i + 1, model);
    }
    
    // Test model selection for different tiers
    println!("\nTesting model selection by tier:");
    
    for tier in [ModelTier::Tiny, ModelTier::Small, ModelTier::Medium, ModelTier::Large] {
        fabric.select_model_for_tier(tier);
        println!("  {:?} -> Selected: {:?}", tier, fabric.get_current_model());
    }
    
    // Test a simple question with the selected model
    println!("\nTesting with selected model:");
    if let Ok(Some(answer)) = fabric.answer_question("What is 2+2?").await {
        println!("Q: What is 2+2?");
        println!("A: {}", answer.lines().next().unwrap_or(""));
    }
    
    Ok(())
}