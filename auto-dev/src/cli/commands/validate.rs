#![allow(unused)]
//! Validation command for running code quality and correctness checks

use anyhow::Result;
use auto_dev_core::parser::model::Specification;
use auto_dev_core::validation::{
    GeneratedCode, SpecValidator, ToolRegistry, ValidationConfig, ValidationPipeline,
};
use clap::{Args, Subcommand};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Validation command arguments
#[derive(Debug, Args)]
pub struct ValidateArgs {
    #[command(subcommand)]
    pub command: Option<ValidateCommands>,

    /// Path to project to validate (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    pub path: PathBuf,

    /// Run all validation stages
    #[arg(short, long)]
    pub all: bool,

    /// Run only compilation checks
    #[arg(long)]
    pub compilation: bool,

    /// Run only tests
    #[arg(long)]
    pub tests: bool,

    /// Run only quality checks
    #[arg(long)]
    pub quality: bool,

    /// Run only security checks
    #[arg(long)]
    pub security: bool,

    /// Run only performance checks
    #[arg(long)]
    pub performance: bool,

    /// Output format (text, json, markdown)
    #[arg(short, long, default_value = "text")]
    pub format: String,

    /// Fail fast on first error
    #[arg(long)]
    pub fail_fast: bool,

    /// Run checks in parallel
    #[arg(long, default_value = "true")]
    pub parallel: bool,
}

#[derive(Debug, Subcommand)]
pub enum ValidateCommands {
    /// Validate code against specification
    Spec {
        /// Path to specification file
        #[arg(short, long)]
        spec: PathBuf,
    },

    /// Discover available validation tools
    Discover,

    /// Run specific validation tool
    Tool {
        /// Name of the tool to run
        name: String,

        /// Additional arguments to pass to the tool
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Generate validation report
    Report {
        /// Output file for the report
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

/// Execute validation command
pub async fn execute(args: ValidateArgs) -> Result<()> {
    match args.command {
        Some(ValidateCommands::Spec { spec }) => validate_specification(&args.path, &spec).await,
        Some(ValidateCommands::Discover) => discover_tools().await,
        Some(ValidateCommands::Tool { name, args: tool_args }) => {
            run_specific_tool(&args.path, &name, &tool_args).await
        }
        Some(ValidateCommands::Report { output }) => {
            generate_report(&args.path, output.as_deref()).await
        }
        None => {
            // Run validation based on flags
            run_validation(&args).await
        }
    }
}

/// Run validation based on command line arguments
async fn run_validation(args: &ValidateArgs) -> Result<()> {
    println!("Running validation checks...\n");

    // First check for essential tools
    check_essential_tools(&args.path).await?;

    // Create validation configuration
    let mut config = ValidationConfig::default();
    config.fail_fast = args.fail_fast;
    config.parallel = args.parallel;

    // Enable stages based on flags
    if args.all {
        // All stages are enabled by default
    } else {
        // Disable all stages first
        for stage in &mut config.stages {
            stage.enabled = false;
        }

        // Enable specific stages
        for stage in &mut config.stages {
            use auto_dev_core::validation::ValidationStage;

            stage.enabled = match stage.stage {
                ValidationStage::Compilation => args.compilation,
                ValidationStage::UnitTests => args.tests,
                ValidationStage::Linting => args.quality,
                ValidationStage::Security => args.security,
                ValidationStage::Performance => args.performance,
                _ => false,
            };
        }
    }

    // Create validation pipeline
    let pipeline = ValidationPipeline::new(config, &args.path);

    // Run validation
    let code = GeneratedCode {
        file_path: args.path.to_string_lossy().to_string(),
        content: String::new(),
        language: "rust".to_string(),
    };

    let result = pipeline.validate(&code).await?;

    // Format and display results
    match args.format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&result)?;
            println!("{}", json);
        }
        "markdown" => {
            print_markdown_report(&result);
        }
        _ => {
            print_text_report(&result);
        }
    }

    // Exit with error code if validation failed
    if !result.passed {
        std::process::exit(1);
    }

    Ok(())
}

/// Validate code against specification
async fn validate_specification(project_path: &Path, spec_path: &Path) -> Result<()> {
    println!("Validating against specification: {}\n", spec_path.display());

    // Load specification
    let spec = Specification::new(spec_path.to_path_buf());

    // Create validator
    let validator = SpecValidator::new(spec, project_path);

    // Run validation
    let result = validator.validate_compliance(&project_path.to_string_lossy()).await?;

    // Print results
    print_text_report(&result);

    if !result.passed {
        std::process::exit(1);
    }

    Ok(())
}

/// Discover available validation tools
async fn discover_tools() -> Result<()> {
    println!("Discovering available validation tools...\n");

    let mut registry = ToolRegistry::new();
    let report = registry.discover_tools().await?;

    println!("Tool Discovery Report");
    println!("========================\n");

    println!("Available Tools ({}):", report.available_tools.len());
    for tool in &report.available_tools {
        println!("   - {}", tool);
    }

    if !report.missing_tools.is_empty() {
        println!("\nMissing Tools ({}):", report.missing_tools.len());
        for missing in &report.missing_tools {
            print!("   - {}", missing.name);
            if missing.required {
                print!(" (REQUIRED)");
            }
            println!();
            if let Some(instructions) = &missing.install_instructions {
                println!("     Install: {}", instructions);
            }
        }
    }

    println!("\nSummary:");
    println!("   Total registered: {}", report.total_registered);
    println!("   Available: {}", report.available_tools.len());
    println!("   Missing: {}", report.missing_tools.len());

    // Get recommendations
    let recommendations = registry.get_tool_recommendations(&report);
    if !recommendations.is_empty() {
        println!("\nRecommendations:");
        for rec in recommendations {
            println!("   - {}", rec.description);
        }
    }

    Ok(())
}

/// Run a specific validation tool
async fn run_specific_tool(project_path: &Path, name: &str, args: &[String]) -> Result<()> {
    println!("Running tool: {} {}\n", name, args.join(" "));

    // For now, just run the tool directly
    // In a real implementation, would use the ToolRegistry
    use std::process::Command;

    let output = Command::new(name).args(args).current_dir(project_path).output()?;

    if output.status.success() {
        println!("Tool completed successfully");
        println!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        println!("Tool failed");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(1);
    }

    Ok(())
}

/// Generate validation report
async fn generate_report(project_path: &Path, output: Option<&Path>) -> Result<()> {
    println!("Generating validation report...\n");

    // Run full validation
    let config = ValidationConfig::default();
    let pipeline = ValidationPipeline::new(config, project_path);

    let code = GeneratedCode {
        file_path: project_path.to_string_lossy().to_string(),
        content: String::new(),
        language: "rust".to_string(),
    };

    let result = pipeline.validate(&code).await?;

    // Generate report
    let report = generate_full_report(&result);

    // Write or print report
    if let Some(output_path) = output {
        tokio::fs::write(output_path, &report).await?;
        println!("Report written to: {}", output_path.display());
    } else {
        println!("{}", report);
    }

    Ok(())
}

/// Print text format report
fn print_text_report(result: &auto_dev_core::validation::ValidationResult) {
    use auto_dev_core::validation::Severity;

    println!("Validation Results");
    println!("====================\n");

    if result.passed {
        println!("Validation PASSED\n");
    } else {
        println!("Validation FAILED\n");
    }

    // Print errors
    if !result.errors.is_empty() {
        println!("Errors ({}):", result.errors.len());
        for error in &result.errors {
            let icon = match error.severity {
                Severity::Critical => "[CRITICAL]",
                Severity::Error => "[ERROR]",
                Severity::Warning => "[WARNING]",
                Severity::Info => "[INFO]",
            };

            println!("  {} {}", icon, error.message);
            if let Some(location) = &error.location {
                println!("     Location: {}:{}", location.file, location.line.unwrap_or(0));
            }
            if let Some(fix) = &error.fix_suggestion {
                println!("     Fix: {}", fix);
            }
        }
        println!();
    }

    // Print warnings
    if !result.warnings.is_empty() {
        println!("Warnings ({}):", result.warnings.len());
        for warning in &result.warnings {
            println!("  - {}", warning.message);
        }
        println!();
    }

    // Print suggestions
    if !result.suggestions.is_empty() {
        println!("Suggestions:");
        for suggestion in &result.suggestions {
            println!("  - {}", suggestion.description);
        }
        println!();
    }

    // Print metrics
    if let Some(coverage) = result.metrics.test_coverage {
        println!("Metrics:");
        println!("  Test coverage: {:.1}%", coverage);
        if let Some(doc) = result.metrics.documentation_coverage {
            println!("  Documentation: {:.1}%", doc);
        }
        if let Some(complexity) = result.metrics.cyclomatic_complexity {
            println!("  Complexity: {:.1}", complexity);
        }
        println!();
    }

    // Print summary
    println!("Summary: {}", result.summary());
}

/// Print markdown format report
fn print_markdown_report(result: &auto_dev_core::validation::ValidationResult) {
    println!("# Validation Report\n");

    if result.passed {
        println!("**Status:** PASSED\n");
    } else {
        println!("**Status:** FAILED\n");
    }

    if !result.errors.is_empty() {
        println!("## Errors\n");
        for error in &result.errors {
            println!("- **{:?}**: {}", error.category, error.message);
            if let Some(fix) = &error.fix_suggestion {
                println!("  - *Fix:* {}", fix);
            }
        }
        println!();
    }

    if !result.warnings.is_empty() {
        println!("## Warnings\n");
        for warning in &result.warnings {
            println!("- {}", warning.message);
        }
        println!();
    }

    if !result.suggestions.is_empty() {
        println!("## Suggestions\n");
        for suggestion in &result.suggestions {
            println!("- {}", suggestion.description);
        }
        println!();
    }

    println!("## Summary\n");
    println!("{}", result.summary());
}

/// Check for essential tools based on project type
async fn check_essential_tools(project_path: &Path) -> Result<()> {
    println!("Checking for essential tools...\n");

    // Detect project type
    let project_type = detect_project_type(project_path).await?;

    // Define essential tools based on project type
    let essential_tools = match project_type {
        ProjectType::Rust => vec![
            EssentialTool {
                name: "cargo",
                check_command: vec!["cargo", "--version"],
                install_instructions: "Install Rust from https://rustup.rs/",
                required: true,
            },
            EssentialTool {
                name: "rustc",
                check_command: vec!["rustc", "--version"],
                install_instructions: "Install Rust from https://rustup.rs/",
                required: true,
            },
        ],
        ProjectType::JavaScript => vec![
            EssentialTool {
                name: "node",
                check_command: vec!["node", "--version"],
                install_instructions: "Install Node.js from https://nodejs.org/",
                required: true,
            },
            EssentialTool {
                name: "npm",
                check_command: vec!["npm", "--version"],
                install_instructions: "npm comes with Node.js installation",
                required: true,
            },
        ],
        ProjectType::Python => vec![
            EssentialTool {
                name: "python",
                check_command: vec!["python", "--version"],
                install_instructions: "Install Python from https://python.org/",
                required: true,
            },
            EssentialTool {
                name: "pip",
                check_command: vec!["pip", "--version"],
                install_instructions: "pip comes with Python installation",
                required: true,
            },
        ],
        ProjectType::Unknown => vec![],
    };

    let mut missing_tools = Vec::new();
    let mut optional_missing = Vec::new();

    // Check each essential tool
    for tool in &essential_tools {
        let is_available = Command::new(tool.check_command[0])
            .args(&tool.check_command[1..])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);

        if !is_available {
            if tool.required {
                missing_tools.push(tool);
            } else {
                optional_missing.push(tool);
            }
        }
    }

    // Check for recommended tools (non-blocking)
    let recommended_tools = get_recommended_tools(&project_type);
    for tool in &recommended_tools {
        let is_available = Command::new(tool.check_command[0])
            .args(&tool.check_command[1..])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);

        if !is_available {
            optional_missing.push(tool);
        }
    }

    // Handle missing required tools
    if !missing_tools.is_empty() {
        println!("Required tools are missing:\n");
        for tool in &missing_tools {
            println!("   - {} (REQUIRED)", tool.name);
            println!("     Install: {}", tool.install_instructions);
        }
        println!();

        // Offer to install or exit
        print!("Would you like to:\n");
        print!("  1) Get installation instructions\n");
        print!("  2) Try to auto-install (if supported)\n");
        print!("  3) Exit\n");
        print!("\nChoice (1-3): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim() {
            "1" => {
                print_installation_instructions(&missing_tools);
                return Err(anyhow::anyhow!("Please install required tools and try again"));
            }
            "2" => {
                attempt_auto_install(&missing_tools).await?;
                // Re-check after installation - need to avoid infinite recursion
                println!("Please restart the validation after installing tools.");
                return Err(anyhow::anyhow!("Tools installed. Please run validation again."));
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Required tools missing. Cannot proceed with validation."
                ));
            }
        }
    }

    // Show optional missing tools as suggestions
    if !optional_missing.is_empty() {
        println!("Optional tools that could enhance validation:\n");
        for tool in &optional_missing {
            println!("   - {}", tool.name);
            println!("     Install: {}", tool.install_instructions);
        }
        println!();
    }

    println!("Essential tools check passed\n");
    Ok(())
}

/// Detect project type based on files in the project
async fn detect_project_type(project_path: &Path) -> Result<ProjectType> {
    if project_path.join("Cargo.toml").exists() {
        return Ok(ProjectType::Rust);
    }

    if project_path.join("package.json").exists() {
        return Ok(ProjectType::JavaScript);
    }

    if project_path.join("requirements.txt").exists()
        || project_path.join("setup.py").exists()
        || project_path.join("pyproject.toml").exists()
    {
        return Ok(ProjectType::Python);
    }

    if project_path.join("go.mod").exists() {
        return Ok(ProjectType::Unknown); // Could add Go support
    }

    Ok(ProjectType::Unknown)
}

#[derive(Debug)]
enum ProjectType {
    Rust,
    JavaScript,
    Python,
    Unknown,
}

struct EssentialTool {
    name: &'static str,
    check_command: Vec<&'static str>,
    install_instructions: &'static str,
    required: bool,
}

/// Get recommended tools based on project type
fn get_recommended_tools(project_type: &ProjectType) -> Vec<EssentialTool> {
    match project_type {
        ProjectType::Rust => vec![
            EssentialTool {
                name: "clippy",
                check_command: vec!["cargo", "clippy", "--version"],
                install_instructions: "rustup component add clippy",
                required: false,
            },
            EssentialTool {
                name: "rustfmt",
                check_command: vec!["cargo", "fmt", "--version"],
                install_instructions: "rustup component add rustfmt",
                required: false,
            },
        ],
        ProjectType::JavaScript => vec![EssentialTool {
            name: "eslint",
            check_command: vec!["npx", "eslint", "--version"],
            install_instructions: "npm install -D eslint",
            required: false,
        }],
        ProjectType::Python => vec![
            EssentialTool {
                name: "black",
                check_command: vec!["black", "--version"],
                install_instructions: "pip install black",
                required: false,
            },
            EssentialTool {
                name: "mypy",
                check_command: vec!["mypy", "--version"],
                install_instructions: "pip install mypy",
                required: false,
            },
        ],
        ProjectType::Unknown => vec![],
    }
}

/// Print installation instructions for missing tools
fn print_installation_instructions(tools: &[&EssentialTool]) {
    println!("\nInstallation Instructions:\n");
    for tool in tools {
        println!("To install {}:", tool.name);
        println!("   {}\n", tool.install_instructions);
    }
}

/// Attempt to auto-install missing tools
async fn attempt_auto_install(tools: &[&EssentialTool]) -> Result<()> {
    println!("\nAttempting to auto-install tools...\n");

    for tool in tools {
        println!("Installing {}...", tool.name);

        // Handle specific auto-installation cases
        match tool.name {
            "clippy" | "rustfmt" => {
                let output =
                    Command::new("rustup").args(&["component", "add", tool.name]).output()?;

                if output.status.success() {
                    println!("   [OK] {} installed successfully", tool.name);
                } else {
                    println!("   [FAILED] Failed to install {}", tool.name);
                    println!("      Please install manually: {}", tool.install_instructions);
                }
            }
            _ => {
                println!("   [WARNING] Auto-installation not supported for {}", tool.name);
                println!("      Please install manually: {}", tool.install_instructions);
            }
        }
    }

    println!();
    Ok(())
}

/// Generate full validation report
fn generate_full_report(result: &auto_dev_core::validation::ValidationResult) -> String {
    let mut report = String::new();

    report.push_str("# Comprehensive Validation Report\n\n");
    report
        .push_str(&format!("Generated: {}\n\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));

    // Status
    report.push_str("## Overall Status\n\n");
    if result.passed {
        report.push_str("**PASSED** - All validation checks completed successfully\n\n");
    } else {
        report.push_str("**FAILED** - Validation issues detected\n\n");
    }

    // Statistics
    report.push_str("## Statistics\n\n");
    report.push_str(&format!("- Errors: {}\n", result.errors.len()));
    report.push_str(&format!("- Warnings: {}\n", result.warnings.len()));
    report.push_str(&format!("- Suggestions: {}\n", result.suggestions.len()));
    report.push_str("\n");

    // Detailed findings
    if !result.errors.is_empty() {
        report.push_str("## Errors\n\n");
        for (i, error) in result.errors.iter().enumerate() {
            report.push_str(&format!("### Error #{}\n", i + 1));
            report.push_str(&format!("- **Severity:** {:?}\n", error.severity));
            report.push_str(&format!("- **Category:** {:?}\n", error.category));
            report.push_str(&format!("- **Message:** {}\n", error.message));
            if let Some(location) = &error.location {
                report.push_str(&format!(
                    "- **Location:** {}:{}\n",
                    location.file,
                    location.line.unwrap_or(0)
                ));
            }
            if let Some(fix) = &error.fix_suggestion {
                report.push_str(&format!("- **Suggested Fix:** {}\n", fix));
            }
            report.push_str("\n");
        }
    }

    if !result.warnings.is_empty() {
        report.push_str("## Warnings\n\n");
        for warning in &result.warnings {
            report.push_str(&format!("- {}\n", warning.message));
        }
        report.push_str("\n");
    }

    // Metrics
    report.push_str("## Code Metrics\n\n");
    if let Some(coverage) = result.metrics.test_coverage {
        report.push_str(&format!("- **Test Coverage:** {:.1}%\n", coverage));
    }
    if let Some(doc) = result.metrics.documentation_coverage {
        report.push_str(&format!("- **Documentation Coverage:** {:.1}%\n", doc));
    }
    if let Some(complexity) = result.metrics.cyclomatic_complexity {
        report.push_str(&format!("- **Cyclomatic Complexity:** {:.1}\n", complexity));
    }
    if let Some(cognitive) = result.metrics.cognitive_complexity {
        report.push_str(&format!("- **Cognitive Complexity:** {:.1}\n", cognitive));
    }
    report.push_str("\n");

    // Recommendations
    if !result.suggestions.is_empty() {
        report.push_str("## Recommendations\n\n");
        for suggestion in &result.suggestions {
            report.push_str(&format!(
                "- **{}** (Priority: {:?}): {}\n",
                suggestion.category, suggestion.priority, suggestion.description
            ));
        }
        report.push_str("\n");
    }

    // Summary
    report.push_str("## Summary\n\n");
    report.push_str(&result.summary());
    report.push_str("\n");

    report
}
