use anyhow::Context;
use clap::{Parser, Subcommand};
use serana::agent::HermesAgent;
use serana::core::{Agent, Config, LlmClient};
use serana::llm::{login_device_auth, OpenAiClient};
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
    Login {
        #[command(subcommand)]
        provider: LoginProvider,
    },
    Mora {
        file: Option<String>,
    },
}

#[derive(Subcommand)]
enum LoginProvider {
    /// Start Codex CLI's device-auth sign-in flow.
    Codex,
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
                println!("{}", serana::core::config::generate_sample_config());
            } else {
                println!("Config path: {:?}", Config::config_path());
                println!("\nResolved configuration:");
                println!("{:#?}", config);
            }
        }
        Some(Commands::Login { provider }) => match provider {
            LoginProvider::Codex => login_codex_device_auth()?,
        },
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

fn login_codex_device_auth() -> anyhow::Result<()> {
    println!("Starting Codex device sign-in...");
    println!("Delegating to: codex login --device-auth");
    login_device_auth()
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
        serana::tui::run(config.workspace.clone(), model, provider, config.clone())?;
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
    use mora_bin::mora::MoraEditor;
    use ratatui::backend::CrosstermBackend;
    use ratatui::Terminal;
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
            let (width, height) = terminal.size().map(|s| (s.width, s.height))?;
            let show_menu_bar = editor.show_menu_bar;
            let ui = mora_bin::mora::ui_node::build_ui(&mut editor, width, height, show_menu_bar);
            let buf = display_protocol::paint::paint(&ui, width, height);
            terminal.draw(|frame| {
                for y in 0..height {
                    for x in 0..width {
                        let cell = buf.get(x, y);
                        let fg = ratatui::style::Color::Rgb(cell.fg.r, cell.fg.g, cell.fg.b);
                        let bg = ratatui::style::Color::Rgb(cell.bg.r, cell.bg.g, cell.bg.b);
                        let mut style = ratatui::style::Style::default().fg(fg).bg(bg);
                        if cell.bold {
                            style = style.add_modifier(ratatui::style::Modifier::BOLD);
                        }
                        if cell.italic {
                            style = style.add_modifier(ratatui::style::Modifier::ITALIC);
                        }
                        if cell.underline {
                            style = style.add_modifier(ratatui::style::Modifier::UNDERLINED);
                        }
                        let buf_cell = frame.buffer_mut().cell_mut((x, y)).unwrap();
                        buf_cell.set_char(cell.ch);
                        buf_cell.set_style(style);
                    }
                }
            })?;

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        break;
                    }
                    editor.handle_key(display_tui::conversions::key_event_from_crossterm(key));
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_accepts_codex_login_command() {
        Cli::command().debug_assert();
        let cli = Cli::parse_from(["serana", "login", "codex"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Login {
                provider: LoginProvider::Codex
            })
        ));
    }
}
