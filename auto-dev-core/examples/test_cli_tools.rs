use auto_dev_core::llm::cli_tools::{ClaudeCLIProvider, FabricProvider};
use auto_dev_core::llm::provider::LLMProvider;

#[tokio::main]
async fn main() {
    println!("Testing CLI Tool Integration\n");

    // Test Claude CLI
    println!("Checking Claude CLI...");
    let claude = ClaudeCLIProvider::new().await;
    if claude.is_available().await {
        println!("Claude CLI is available!");
        println!("  Name: {}", claude.name());
        println!("  Tier: {:?}", claude.tier());
    } else {
        println!("Claude CLI not found");
        println!("  Install with: pip install claude-cli");
    }

    // Test Fabric
    println!("\nChecking Fabric CLI...");
    let fabric = FabricProvider::new().await;
    if fabric.is_available().await {
        println!("Fabric CLI is available!");
        println!("  Name: {}", fabric.name());
        println!("  Tier: {:?}", fabric.tier());
    } else {
        println!("Fabric CLI not found");
        println!("  Install from: https://github.com/danielmiessler/fabric");
    }
}
