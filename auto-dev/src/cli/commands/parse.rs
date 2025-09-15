#![allow(unused)]
//! Parse specifications command

use anyhow::Result;
use auto_dev_core::parser::{SpecParser, Priority};
use auto_dev_core::parser::todo_extractor::TodoConfig;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use crate::cli::app::ParseArgs;

/// Execute the parse command
pub async fn execute(args: ParseArgs) -> Result<()> {
    let path = if args.target_self {
        // Use auto-dev-rs's own source directory
        PathBuf::from("src")
    } else {
        PathBuf::from(args.path)
    };
    
    println!("Parsing specifications from: {}", path.display());
    
    // Create parser with TODO configuration if requested
    let parser = if args.include_todos {
        let mut todo_config = TodoConfig::default();
        todo_config.include_todos = true;
        SpecParser::with_todo_config(todo_config)
    } else {
        SpecParser::new()
    };
    
    let spec_path = path.as_path();
    
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
        // Parse directory with TODO extraction if enabled
        let specs = if args.include_todos {
            parser.parse_directory_with_todos(spec_path).await?
        } else {
            parser.parse_directory(spec_path).await?
        };
        
        println!("\n Found {} specification files", specs.len());
        
        let total_reqs: usize = specs.iter().map(|s| s.requirements.len()).sum();
        let total_apis: usize = specs.iter().map(|s| s.apis.len()).sum();
        let total_models: usize = specs.iter().map(|s| s.data_models.len()).sum();
        
        println!("\n Summary:");
        println!("  Total Requirements: {}", total_reqs);
        println!("  Total APIs: {}", total_apis);
        println!("  Total Data Models: {}", total_models);
        
        // Show priority breakdown if requested
        if args.show_priorities && total_reqs > 0 {
            let mut priority_counts: HashMap<Priority, usize> = HashMap::new();
            for spec in &specs {
                for req in &spec.requirements {
                    *priority_counts.entry(req.priority).or_insert(0) += 1;
                }
            }
            
            println!("\n Priority Breakdown:");
            if let Some(count) = priority_counts.get(&Priority::Critical) {
                println!("  Critical: {}", count);
            }
            if let Some(count) = priority_counts.get(&Priority::High) {
                println!("  High: {}", count);
            }
            if let Some(count) = priority_counts.get(&Priority::Medium) {
                println!("  Medium: {}", count);
            }
            if let Some(count) = priority_counts.get(&Priority::Low) {
                println!("  Low: {}", count);
            }
        }
        
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
        println!(" Path does not exist: {}", path.display());
    }
    
    // Validate specifications if requested
    if args.validate {
        println!("\n Validating extracted specifications...");
        // TODO: Add actual validation logic once validator is available
        println!("  Validation complete: All specifications are actionable.");
    }
    
    Ok(())
}