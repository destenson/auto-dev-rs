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

    /// Target auto-dev-rs's own codebase for analysis
    #[arg(long, global = true, help = "Use auto-dev-rs on its own codebase")]
    pub target_self: bool,

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

    /// Control the autonomous development loop
    #[command(about = "Manage the continuous monitoring and autonomous development loop")]
    Loop(super::commands::loop_control::LoopCommand),

    /// Start the autonomous development loop (alias for 'loop start --background')
    #[command(about = "Start the autonomous development loop in background")]
    Run,

    /// Start the autonomous development loop (alias for 'loop start --background')
    #[command(about = "Start the autonomous development loop in background")]
    Start,

    /// Initialize auto-dev in the current directory
    #[command(about = "Initialize auto-dev configuration and directory structure")]
    Init,

    /// Self-development integration control
    #[command(name = "self-dev", about = "Control self-development integration and orchestration")]
    SelfDev(super::commands::self_dev::SelfDevCommand),
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

    /// Include TODO comments as requirements
    #[arg(long, help = "Extract TODO comments and convert them to requirements")]
    pub include_todos: bool,

    /// Show priority breakdown of requirements
    #[arg(long, help = "Display requirements grouped by priority")]
    pub show_priorities: bool,

    /// Target self (parse auto-dev-rs's own source)
    #[arg(long, help = "Parse auto-dev-rs's own codebase for specifications")]
    pub target_self: bool,

    /// Validate extracted specifications
    #[arg(long, help = "Validate that extracted specifications are actionable")]
    pub validate: bool,
}

#[derive(Parser, Debug)]
pub struct AnalyzeArgs {
    /// Path to file or directory to analyze
    #[arg(help = "Path to file or directory to analyze", default_value = ".")]
    pub path: String,
}
