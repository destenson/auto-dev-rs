#![allow(unused)]
//! Parse specifications command

use anyhow::Result;
use auto_dev_core::parser::SpecParser;
use std::path::Path;

/// Execute the parse command
pub async fn execute(path: String) -> Result<()> {
    println!("Parsing specifications from: {}", path);
    
    let parser = SpecParser::new();
    let spec_path = Path::new(&path);
    
    if spec_path.is_file() {
        // Parse single file
        let spec = parser.parse_file(spec_path).await?;
        
        println!("\n Specification: {}", spec_path.display());
        println!("  Requirements: {}", spec.requirements.len());
        println!("  APIs: {}", spec.apis.len());
        println!("  Data Models: {}", spec.data_models.len());
        println!("  Behaviors: {}", spec.behaviors.len());
        
        // Show requirements
        if !spec.requirements.is_empty() {
            println!("\n Requirements:");
            for req in spec.requirements.iter().take(5) {
                println!("  - [{:?}] {}: {}", 
                    req.priority,
                    req.id,
                    req.description.chars().take(80).collect::<String>()
                );
            }
            if spec.requirements.len() > 5 {
                println!("  ... and {} more", spec.requirements.len() - 5);
            }
        }
        
        // Show APIs
        if !spec.apis.is_empty() {
            println!("\n🔌 APIs:");
            for api in spec.apis.iter().take(5) {
                println!("  - {} {}", api.method, api.endpoint);
            }
            if spec.apis.len() > 5 {
                println!("  ... and {} more", spec.apis.len() - 5);
            }
        }
    } else if spec_path.is_dir() {
        // Parse directory
        let specs = parser.parse_directory(spec_path).await?;
        
        println!("\n Found {} specification files", specs.len());
        
        let total_reqs: usize = specs.iter().map(|s| s.requirements.len()).sum();
        let total_apis: usize = specs.iter().map(|s| s.apis.len()).sum();
        let total_models: usize = specs.iter().map(|s| s.data_models.len()).sum();
        
        println!("\n Summary:");
        println!("  Total Requirements: {}", total_reqs);
        println!("  Total APIs: {}", total_apis);
        println!("  Total Data Models: {}", total_models);
        
        for spec in specs.iter().take(10) {
            println!("\n   {}", spec.source.display());
            println!("     Requirements: {}, APIs: {}, Models: {}", 
                spec.requirements.len(),
                spec.apis.len(),
                spec.data_models.len()
            );
        }
        
        if specs.len() > 10 {
            println!("\n  ... and {} more files", specs.len() - 10);
        }
    } else {
        println!(" Path does not exist: {}", path);
    }
    
    Ok(())
}
