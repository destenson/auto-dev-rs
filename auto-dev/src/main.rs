use anyhow::Result;
use clap::Parser;
use tracing::info;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

mod cli;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    // Parse CLI arguments first to get verbosity level
    let cli = Cli::parse();

    // Initialize tracing with appropriate verbosity
    let filter = if cli.verbose > 0 {
        match cli.verbose {
            1 => "debug",
            2.. => "trace",
            _ => "info",
        }
    } else {
        "info"
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)))
        .init();

    // Process commands
    match cli.command {
        Commands::Generate(args) => {
            info!("Generate command: {:?}", args);
            // Create async runtime for the command
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(cli::commands::generate::execute(args))?;
        }
        Commands::Manage(args) => {
            info!("Manage command: {:?}", args);
            println!("Project management feature coming soon!");
        }
        Commands::Test(args) => {
            info!("Test command: {:?}", args);
            println!("Testing framework integration coming soon!");
        }
        Commands::Deploy(args) => {
            info!("Deploy command: {:?}", args);
            println!("Deployment automation coming soon!");
        }
        Commands::Docs(args) => {
            info!("Docs command: {:?}", args);
            // Create async runtime for the command
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(cli::commands::docs::execute(args))?;
        }
        Commands::Parse(args) => {
            info!("Parse command: {:?}", args);
            // Create async runtime for the command
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(cli::commands::parse::execute(args))?;
        }
        Commands::Analyze(args) => {
            info!("Analyze command: {:?}", args);
            // Create async runtime for the command
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(cli::commands::analyze::execute(args.path, cli.target_self))?;
        }
        Commands::Loop(args) => {
            info!("Loop command: {:?}", args);
            // Create async runtime for the command
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(cli::commands::loop_control::execute(args, cli.target_self))?;
        }
        Commands::Run | Commands::Start => {
            info!("Starting autonomous development loop");
            // Create async runtime for the command
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(cli::commands::loop_control::run_default(cli.target_self))?;
        }
        Commands::Init => {
            info!("Initializing auto-dev");
            // Create async runtime for the command
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(cli::commands::loop_control::init_project())?;
        }
        Commands::SelfDev(command) => {
            info!("Self-dev command: {:?}", command);
            // Create async runtime for the command
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(cli::commands::self_dev::handle_self_dev_command(command))?;
        }
        Commands::Bootstrap(command) => {
            info!("Bootstrap command: {:?}", command);
            // Create async runtime for the command
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(cli::commands::bootstrap::handle_bootstrap_command(command))?;
        }
        Commands::Metrics(command) => {
            info!("Metrics command: {:?}", command);
            // Create async runtime for the command
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(cli::commands::metrics::handle_metrics_command(command))?;
        }
        Commands::Claude { command, args } => {
            info!("Claude command: {} with args: {:?}", command, args);
            // Create async runtime for the command
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(cli::claude_commands::handle_claude_command(command, args))?;
        }
    }

    Ok(())
}
