//! Analyze command - classify and analyze project files

use anyhow::Result;
use auto_dev_core::llm::classifier::HeuristicClassifier;
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

/// Analyze a project directory or file
pub async fn execute(path: String) -> Result<()> {
    let path = Path::new(&path);
    let classifier = HeuristicClassifier::new();
    
    if path.is_file() {
        // Analyze single file
        analyze_file(path, &classifier).await?;
    } else if path.is_dir() {
        // Analyze entire directory
        analyze_directory(path, &classifier).await?;
    } else {
        println!(" Path does not exist: {}", path.display());
    }
    
    Ok(())
}

/// Analyze a single file
async fn analyze_file(path: &Path, classifier: &HeuristicClassifier) -> Result<()> {
    println!(" Analyzing: {}", path.display());
    
    let content = fs::read_to_string(path).await?;
    let result = classifier.classify_content(&content);
    
    println!("\n Results:");
    println!("  Type: {}", if result.is_code { 
        "Code" 
    } else if result.is_documentation { 
        "Documentation" 
    } else if result.is_config { 
        "Configuration" 
    } else { 
        "Other" 
    });
    
    if let Some(lang) = &result.language {
        println!("  Language: {}", lang);
    }
    
    if result.is_test {
        println!("   Contains tests");
    }
    
    println!("  Confidence: {:.0}%", result.confidence * 100.0);
    
    Ok(())
}

/// Analyze a directory
async fn analyze_directory(dir: &Path, classifier: &HeuristicClassifier) -> Result<()> {
    println!(" Analyzing directory: {}", dir.display());
    
    let mut stats = ProjectStats::new();
    analyze_dir_recursive(dir, classifier, &mut stats).await?;
    
    // Display results
    println!("\n Project Analysis Results:");
    println!("  Total files: {}", stats.total_files);
    
    if !stats.languages.is_empty() {
        println!("\n   Languages detected:");
        let mut langs: Vec<_> = stats.languages.iter().collect();
        langs.sort_by(|a, b| b.1.cmp(a.1));
        
        for (lang, count) in langs.iter().take(10) {
            let percentage = (**count as f32 / stats.code_files as f32) * 100.0;
            println!("    {} {:<12} ({:.1}%)", 
                get_language_emoji(lang),
                lang, 
                percentage
            );
        }
    }
    
    println!("\n   File types:");
    println!("    Code files:     {}", stats.code_files);
    println!("    Test files:     {}", stats.test_files);
    println!("    Documentation:  {}", stats.doc_files);
    println!("    Configuration:  {}", stats.config_files);
    println!("    Other:          {}", stats.other_files);
    
    if stats.code_files > 0 {
        let test_coverage = (stats.test_files as f32 / stats.code_files as f32) * 100.0;
        println!("\n   Metrics:");
        println!("    Test coverage: {:.1}% of code files have tests", test_coverage);
    }
    
    Ok(())
}

/// Recursively analyze directory
async fn analyze_dir_recursive(
    dir: &Path, 
    classifier: &HeuristicClassifier,
    stats: &mut ProjectStats
) -> Result<()> {
    let mut entries = fs::read_dir(dir).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        
        // Skip hidden and common ignore directories
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') || 
               name_str == "target" || 
               name_str == "node_modules" ||
               name_str == "__pycache__" {
                continue;
            }
        }
        
        if path.is_dir() {
            Box::pin(analyze_dir_recursive(&path, classifier, stats)).await?;
        } else if path.is_file() {
            // Try to read and classify file
            if let Ok(content) = fs::read_to_string(&path).await {
                let result = classifier.classify_content(&content);
                stats.total_files += 1;
                
                if result.is_code {
                    stats.code_files += 1;
                    if let Some(lang) = result.language {
                        *stats.languages.entry(lang).or_insert(0) += 1;
                    }
                    if result.is_test {
                        stats.test_files += 1;
                    }
                } else if result.is_documentation {
                    stats.doc_files += 1;
                } else if result.is_config {
                    stats.config_files += 1;
                } else {
                    stats.other_files += 1;
                }
            }
        }
    }
    
    Ok(())
}

/// Project statistics
struct ProjectStats {
    total_files: usize,
    code_files: usize,
    test_files: usize,
    doc_files: usize,
    config_files: usize,
    other_files: usize,
    languages: HashMap<String, usize>,
}

impl ProjectStats {
    fn new() -> Self {
        Self {
            total_files: 0,
            code_files: 0,
            test_files: 0,
            doc_files: 0,
            config_files: 0,
            other_files: 0,
            languages: HashMap::new(),
        }
    }
}

/// Get emoji for language
fn get_language_emoji(_lang: &str) -> &'static str {
    ""
}
