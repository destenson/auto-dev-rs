use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

pub mod init;

/// Handle code generation commands
pub async fn execute(args: crate::cli::app::GenerateArgs) -> Result<()> {
    info!("Starting project generation with args: {:?}", args);
    
    // Determine instructions source
    let (instruction_text, instruction_file) = if let Some(file) = args.file {
        // Explicit file provided
        (None, Some(file))
    } else if let Some(instructions) = args.instructions {
        // Check if it's a file path or raw text
        let path = PathBuf::from(&instructions);
        if path.exists() && path.is_file() {
            (None, Some(path))
        } else {
            (Some(instructions), None)
        }
    } else {
        return Err(anyhow::anyhow!(
            "No instructions provided. Use 'auto-dev generate \"instructions\"' or 'auto-dev generate -f file.md'"
        ));
    };
    
    if args.dry_run {
        println!("🔍 Dry run mode - no files will be created");
    }
    
    // Initialize project with instructions
    init::init_with_instructions(
        instruction_text,
        instruction_file,
        args.output,
    ).await?;
    
    // TODO: Implement remaining steps (PRPs 301-306)
    if !args.dry_run {
        println!("✨ Project generation started!");
        println!("📝 Next steps will be implemented in upcoming PRPs:");
        println!("  - Code implementation (PRP 302)");
        println!("  - Local model integration (PRP 303)"); 
        println!("  - Build-fix loop (PRP 304)");
    }
    
    Ok(())
}