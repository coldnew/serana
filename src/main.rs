use clap::{Parser, Subcommand};
use serana::config::Config;

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
    Config,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("serana=debug")
        .init();

    let cli = Cli::parse();

    let mut config = Config {
        workspace: std::path::PathBuf::from(&cli.workspace),
        ..Config::default()
    };

    match cli.command {
        Some(Commands::Interactive) => {
            config.interactive = true;
            run_interactive(config).await?;
        }
        Some(Commands::Config) => {
            println!("{:#?}", config);
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

async fn run_once(_config: Config, instruction: &str) -> anyhow::Result<()> {
    println!("Running instruction: {}", instruction);
    // TODO: Instantiate agent and execute
    Ok(())
}

async fn run_interactive(config: Config) -> anyhow::Result<()> {
    println!("Serana interactive mode (Ctrl+C to exit)");
    loop {
        println!("\n> ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        if input == "exit" || input == "quit" {
            break;
        }
        run_once(config.clone(), input).await?;
    }
    Ok(())
}
