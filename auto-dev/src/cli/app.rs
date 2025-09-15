use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "auto-dev",
    version,
    about = "Auto Dev - Streamline and automate your development process",
    long_about = "Auto Dev is a comprehensive development automation tool that helps you manage projects, generate code, run tests, and deploy applications with ease."
)]
pub struct Cli {
    /// Increase verbosity (-v for debug, -vv for trace)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Path to configuration file
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate code and boilerplate
    #[command(about = "Generate code, boilerplate, and project structures")]
    Generate(GenerateArgs),

    /// Manage projects and tasks
    #[command(about = "Manage projects, tasks, and workflows")]
    Manage(ManageArgs),

    /// Run tests and test suites
    #[command(about = "Run tests, test suites, and testing frameworks")]
    Test(TestArgs),

    /// Deploy applications
    #[command(about = "Deploy applications to various platforms")]
    Deploy(DeployArgs),

    /// Generate documentation
    #[command(about = "Generate and maintain project documentation")]
    Docs(DocsArgs),

    /// Parse specifications
    #[command(about = "Parse specifications and extract requirements")]
    Parse(ParseArgs),

    /// Analyze files and projects
    #[command(about = "Analyze and classify files or entire projects")]
    Analyze(AnalyzeArgs),
}

#[derive(Parser, Debug)]
pub struct GenerateArgs {
    /// Type of code to generate (e.g., rust, python, javascript)
    #[arg(help = "Language or framework for code generation")]
    pub target: Option<String>,

    /// Component to generate (e.g., struct, class, function)
    #[arg(help = "Type of component to generate")]
    pub component: Option<String>,

    /// Name of the generated item
    #[arg(help = "Name for the generated code")]
    pub name: Option<String>,
}

#[derive(Parser, Debug)]
pub struct ManageArgs {
    /// Project management action (e.g., init, status, task)
    #[arg(help = "Management action to perform")]
    pub action: Option<String>,

    /// Additional arguments for the action
    #[arg(help = "Additional arguments")]
    pub args: Vec<String>,
}

#[derive(Parser, Debug)]
pub struct TestArgs {
    /// Test command (e.g., run, watch, coverage)
    #[arg(help = "Test command to execute")]
    pub command: Option<String>,

    /// Test patterns or files
    #[arg(help = "Test patterns or files to run")]
    pub patterns: Vec<String>,
}

#[derive(Parser, Debug)]
pub struct DeployArgs {
    /// Deployment target (e.g., aws, docker, kubernetes)
    #[arg(help = "Deployment target platform")]
    pub target: Option<String>,

    /// Environment (e.g., dev, staging, production)
    #[arg(help = "Deployment environment")]
    pub environment: Option<String>,
}

#[derive(Parser, Debug)]
pub struct DocsArgs {
    /// Documentation command (e.g., generate, serve, check)
    #[arg(help = "Documentation command to execute")]
    pub command: Option<String>,

    /// Output format (e.g., markdown, html, pdf)
    #[arg(short, long, help = "Output format for documentation")]
    pub format: Option<String>,
}

#[derive(Parser, Debug)]
pub struct ParseArgs {
    /// Path to specification file or directory
    #[arg(help = "Path to specification file or directory to parse")]
    pub path: String,
}

#[derive(Parser, Debug)]
pub struct AnalyzeArgs {
    /// Path to file or directory to analyze
    #[arg(help = "Path to file or directory to analyze", default_value = ".")]
    pub path: String,
}