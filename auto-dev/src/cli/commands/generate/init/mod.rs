//! Project initialization with native tool detection

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub mod detector;
pub mod executor;
pub mod instructions;

pub use detector::ProjectDetector;
pub use executor::ToolExecutor;
pub use instructions::InstructionParser;

/// Initialize a project based on instructions
pub async fn init_with_instructions(
    instruction: Option<String>,
    instruction_file: Option<PathBuf>,
    output_dir: Option<PathBuf>,
) -> Result<()> {
    // Parse instructions
    let instructions = if let Some(file) = instruction_file {
        InstructionParser::from_file(&file).await?
    } else if let Some(text) = instruction {
        InstructionParser::from_string(&text)?
    } else {
        return Err(anyhow::anyhow!("No instructions provided"));
    };

    // Detect project type
    let detector = ProjectDetector::new();
    let project_type = detector.detect(&instructions);

    // Determine output directory
    let output_dir = output_dir.unwrap_or_else(|| {
        instructions
            .metadata
            .project_name
            .as_ref()
            .map(|name| PathBuf::from(name))
            .unwrap_or_else(|| PathBuf::from("new-project"))
    });

    // Execute appropriate tool
    let executor = ToolExecutor::new();

    tracing::info!("Detected project type: {:?}", project_type);
    tracing::info!("Initializing project in: {:?}", output_dir);

    executor.execute(project_type, &output_dir, &instructions).await?;

    // Create .auto-dev directory with instructions
    create_auto_dev_dir(&output_dir, &instructions).await?;

    println!("✓ Project initialized successfully at {:?}", output_dir);

    Ok(())
}

/// Create .auto-dev directory with project instructions
async fn create_auto_dev_dir(
    project_dir: &Path,
    instructions: &instructions::InstructionDocument,
) -> Result<()> {
    let auto_dev_dir = project_dir.join(".auto-dev");
    tokio::fs::create_dir_all(&auto_dev_dir).await?;

    // Save original instructions
    let instructions_file = auto_dev_dir.join("instructions.md");
    tokio::fs::write(&instructions_file, &instructions.raw_content).await?;

    // Save parsed metadata
    let metadata_file = auto_dev_dir.join("metadata.json");
    let metadata_json = serde_json::to_string_pretty(&instructions.metadata)?;
    tokio::fs::write(&metadata_file, metadata_json).await?;

    // Create subdirectories
    let subdirs = ["cache", "history", "generated"];
    for subdir in &subdirs {
        let path = auto_dev_dir.join(subdir);
        tokio::fs::create_dir_all(&path).await?;
    }

    Ok(())
}
