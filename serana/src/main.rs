use anyhow::Context;
use clap::{Parser, Subcommand};
use serana_agent::HermesAgent;
use serana_core::{Agent, Config, LlmClient};
use serana_llm::OpenAiClient;
use std::io::IsTerminal;

#[derive(Parser)]
#[command(name = "serana")]
#[command(about = "Personal Hermes agent", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, default_value = ".")]
    workspace: String,

    instruction: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    Interactive,
    Config {
        #[arg(short, long)]
        sample: bool,
    },
    Mora {
        file: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "serana=info".parse().expect("valid default filter")),
        )
        .init();

    let cli = Cli::parse();

    let mut config = Config::load().with_context(|| "Failed to load config")?;
    config.workspace = std::path::PathBuf::from(&cli.workspace);

    match cli.command {
        Some(Commands::Interactive) => {
            config.interactive = true;
            run_interactive(config).await?;
        }
        Some(Commands::Config { sample }) => {
            if sample {
                println!("{}", serana_core::config::generate_sample_config());
            } else {
                println!("Config path: {:?}", Config::config_path());
                println!("\nResolved configuration:");
                println!("{:#?}", config);
            }
        }
        Some(Commands::Mora { file }) => {
            run_mora(file)?;
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
    let agent = HermesAgent::hermes(llm);

    println!("Running: {}", instruction);
    let result = agent.execute(instruction).await?;
    println!("\n{}", result.response);

    if !result.tool_calls.is_empty() {
        println!("\n[Tool calls: {}]", result.tool_calls.len());
    }

    Ok(())
}

async fn run_interactive(config: Config) -> anyhow::Result<()> {
    if std::io::stdout().is_terminal() && std::io::stdin().is_terminal() {
        let model = config.model().to_string();
        let provider = config.provider.name.clone();
        serana_tui::run(config.workspace.clone(), model, provider, config.clone())?;
    } else {
        println!("Serana interactive mode (Ctrl+C to exit)");
        println!("(No TTY detected - using simple REPL)");
        loop {
            print!("> ");
            std::io::Write::flush(&mut std::io::stdout())?;
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
    }
    Ok(())
}

fn run_mora(file: Option<String>) -> anyhow::Result<()> {
    use crossterm::{
        event::{self, Event, KeyCode, KeyModifiers},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::backend::CrosstermBackend;
    use ratatui::Terminal;
    use mora_bin::mora::MoraEditor;
    use std::io;
    use std::path::Path;
    use std::time::Duration;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let size = terminal.size()?;
    let editor_height = size.height.saturating_sub(3) as usize;
    let mut editor = if let Some(ref path) = file {
        MoraEditor::open(Path::new(path), editor_height)
            .map_err(|e| anyhow::anyhow!("Failed to open {}: {}", path, e))?
    } else {
        MoraEditor::new(editor_height)
    };

    let result = (|| -> anyhow::Result<()> {
        loop {
            terminal.draw(|frame| {
                let widget = mora_bin::mora::ui::EditorWidget::new(&editor);
                frame.render_widget(widget, frame.area());
            })?;

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        break;
                    }
                    editor.handle_key(key);
                }
            }

            if event::poll(Duration::from_millis(0))? {
                if let Event::Resize(_w, h) = event::read()? {
                    editor.set_height(h.saturating_sub(3) as usize);
                }
            }

            if editor.quit_requested() {
                break;
            }
        }
        Ok(())
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
