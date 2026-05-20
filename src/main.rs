use clap::{Parser, Subcommand};
use serana::agent::{Agent, coding::CodingAgent};
use serana::config::Config;
use serana::llm::{LlmClient, openai::OpenAiClient};
use serana::tools::ToolRegistry;
use anyhow::Context;

#[derive(Parser)]
#[command(name = "serana")]
#[command(about = "Personal coding agent", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Workspace directory path
    #[arg(short, long, default_value = ".")]
    workspace: String,

    /// Instruction to execute (non-interactive mode)
    instruction: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start interactive REPL mode
    Interactive,
    /// Show configuration
    Config {
        /// Show sample config.toml
        #[arg(short, long)]
        sample: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("serana=debug")
        .init();

    let cli = Cli::parse();

    let mut config = Config::load()
        .with_context(|| "Failed to load config")?;
    config.workspace = std::path::PathBuf::from(&cli.workspace);

    match cli.command {
        Some(Commands::Interactive) => {
            config.interactive = true;
            run_interactive(config).await?;
        }
        Some(Commands::Config { sample }) => {
            if sample {
                println!("{}", serana::config::generate_sample_config());
            } else {
                println!("Config path: {:?}", Config::config_path());
                println!("\nResolved configuration:");
                println!("{:#?}", config);
            }
        }
        None => {
            if let Some(instruction) = cli.instruction {
                run_once(config, &instruction).await?;
            } else {
                run_interactive(config).await?;
            }
        }
    }

    Ok(())
}

async fn run_once(config: Config, instruction: &str) -> anyhow::Result<()> {
    let llm: Box<dyn LlmClient> = Box::new(OpenAiClient::new(config));
    let tools = ToolRegistry::new();
    let agent = CodingAgent::new(llm, tools);

    println!("Running: {}", instruction);
    let result = agent.execute(instruction).await?;
    println!("\n{}", result.response);

    if !result.tool_calls.is_empty() {
        println!("\n[Tool calls: {}]", result.tool_calls.len());
    }

    Ok(())
}

async fn run_interactive(config: Config) -> anyhow::Result<()> {
    serana::tui::run(config.workspace)?;
    Ok(())
}
