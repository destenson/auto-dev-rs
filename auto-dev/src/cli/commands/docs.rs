//! Documentation generation command handler

use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing::{error, info, warn};
use auto_dev_core::docs::{
    DocumentationSystem, DocConfig, DocFormat, ChangelogConfig, ChangelogBuilder,
};
use auto_dev_core::docs::changelog::ChangeCategory;

use crate::cli::app::DocsArgs;

/// Handle the docs command
pub async fn execute(args: DocsArgs) -> Result<()> {
    let command = args.command.as_ref().map(|s| s.as_str()).unwrap_or("generate");
    
    match command {
        "generate" => generate_docs(args).await,
        "check" | "validate" => check_docs(args).await,
        "coverage" => check_coverage(args).await,
        "changelog" => update_changelog(args).await,
        "api" => generate_api_docs(args).await,
        "readme" => update_readme(args).await,
        _ => {
            error!("Unknown docs command: {}", command);
            println!("Available commands: generate, check, coverage, changelog, api, readme");
            Ok(())
        }
    }
}

/// Generate documentation for the project
async fn generate_docs(args: DocsArgs) -> Result<()> {
    info!("Generating documentation...");
    
    let project_root = std::env::current_dir()
        .context("Failed to get current directory")?;
    
    // Configure output formats
    let formats = if let Some(format_str) = args.format {
        match format_str.as_str() {
            "markdown" | "md" => vec![DocFormat::Markdown],
            "html" => vec![DocFormat::Html],
            "json" => vec![DocFormat::Json],
            "all" => vec![DocFormat::Markdown, DocFormat::Html, DocFormat::Json],
            _ => {
                warn!("Unknown format: {}, using markdown", format_str);
                vec![DocFormat::Markdown]
            }
        }
    } else {
        vec![DocFormat::Markdown]
    };
    
    let config = DocConfig {
        output_dir: project_root.join("docs"),
        formats,
        include_private: false,
        generate_examples: true,
        update_readme: true,
        changelog: ChangelogConfig::default(),
    };
    
    let doc_system = DocumentationSystem::new(project_root.clone(), config)
        .context("Failed to create documentation system")?;
    
    let result = doc_system.generate_all().await
        .context("Failed to generate documentation")?;
    
    println!("✅ Documentation generated successfully!");
    println!("📁 Files created: {}", result.files.len());
    println!("📊 Statistics:");
    println!("   - Modules documented: {}", result.stats.modules);
    println!("   - Functions documented: {}", result.stats.functions);
    println!("   - Examples generated: {}", result.stats.examples);
    println!("   - Coverage: {:.1}%", result.stats.coverage);
    
    if !result.warnings.is_empty() {
        println!("\n⚠️  Warnings:");
        for warning in result.warnings {
            println!("   - {}", warning);
        }
    }
    
    Ok(())
}

/// Check documentation for issues
async fn check_docs(args: DocsArgs) -> Result<()> {
    info!("Checking documentation...");
    
    let project_root = std::env::current_dir()
        .context("Failed to get current directory")?;
    
    let config = DocConfig::default();
    let doc_system = DocumentationSystem::new(project_root, config)
        .context("Failed to create documentation system")?;
    
    let issues = doc_system.validate().await
        .context("Failed to validate documentation")?;
    
    if issues.is_empty() {
        println!("✅ No documentation issues found!");
    } else {
        println!("⚠️  Found {} documentation issues:", issues.len());
        for issue in issues {
            println!("   - {}", issue);
        }
    }
    
    Ok(())
}

/// Check documentation coverage
async fn check_coverage(args: DocsArgs) -> Result<()> {
    info!("Checking documentation coverage...");
    
    let project_root = std::env::current_dir()
        .context("Failed to get current directory")?;
    
    let config = DocConfig::default();
    let doc_system = DocumentationSystem::new(project_root, config)
        .context("Failed to create documentation system")?;
    
    let coverage = doc_system.check_coverage().await
        .context("Failed to check coverage")?;
    
    println!("📊 Documentation Coverage: {:.1}%", coverage);
    
    if coverage >= 80.0 {
        println!("✅ Good coverage!");
    } else if coverage >= 50.0 {
        println!("⚠️  Coverage could be improved");
    } else {
        println!("❌ Low coverage - consider adding more documentation");
    }
    
    Ok(())
}

/// Update or generate changelog
async fn update_changelog(args: DocsArgs) -> Result<()> {
    info!("Updating changelog...");
    
    let project_root = std::env::current_dir()
        .context("Failed to get current directory")?;
    
    let mut config = ChangelogConfig::default();
    config.file_path = project_root.join("CHANGELOG.md");
    
    let mut changelog = ChangelogBuilder::new(config);
    
    // Load existing changelog
    changelog.load()
        .context("Failed to load existing changelog")?;
    
    // Extract changes from git
    changelog.extract_from_git(None).await
        .context("Failed to extract changes from git")?;
    
    // Update the file
    changelog.update()
        .context("Failed to update changelog")?;
    
    println!("✅ Changelog updated successfully!");
    
    Ok(())
}

/// Generate API documentation
async fn generate_api_docs(args: DocsArgs) -> Result<()> {
    info!("Generating API documentation...");
    
    let project_root = std::env::current_dir()
        .context("Failed to get current directory")?;
    
    let config = DocConfig {
        output_dir: project_root.join("docs"),
        formats: vec![DocFormat::Markdown],
        include_private: false,
        generate_examples: true,
        update_readme: false,
        changelog: ChangelogConfig::default(),
    };
    
    let doc_system = DocumentationSystem::new(project_root, config)
        .context("Failed to create documentation system")?;
    
    let files = doc_system.generate_api_docs().await
        .context("Failed to generate API docs")?;
    
    println!("✅ API documentation generated!");
    println!("📁 Files created: {}", files.len());
    
    for file in files {
        if let Some(name) = file.file_name() {
            println!("   - {}", name.to_string_lossy());
        }
    }
    
    Ok(())
}

/// Update README with auto-generated sections
async fn update_readme(args: DocsArgs) -> Result<()> {
    info!("Updating README...");
    
    let project_root = std::env::current_dir()
        .context("Failed to get current directory")?;
    
    let config = DocConfig::default();
    let doc_system = DocumentationSystem::new(project_root, config)
        .context("Failed to create documentation system")?;
    
    doc_system.update_readme().await
        .context("Failed to update README")?;
    
    println!("✅ README updated successfully!");
    
    Ok(())
}
