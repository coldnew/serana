use clap::Parser as ClapParser;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{self, Color, Stylize},
    terminal::{self, ClearType},
};
use mishell_parser::{Parser, Command, CommandBody, SimpleCommand, Pipeline};
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, Stdio};

mod shell;
mod autocomplete;
mod history;
mod syntax;

use shell::Shell;
use autocomplete::AutoCompleter;
use history::History;
use syntax::SyntaxHighlighter;

#[derive(ClapParser)]
#[command(name = "mishell")]
#[command(about = "A powerful shell with bash syntax and fish features")]
struct Cli {
    /// Execute a command and exit
    #[arg(short = 'c', long)]
    command: Option<String>,

    /// Run in interactive mode (default if TTY)
    #[arg(short, long)]
    interactive: bool,

    /// Disable fish features
    #[arg(long)]
    no_fish: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        // Execute single command
        let mut shell = Shell::new(!cli.no_fish)?;
        shell.execute(&cmd)?;
        return Ok(());
    }

    // Interactive mode
    run_interactive(!cli.no_fish)
}

fn run_interactive(fish_features: bool) -> anyhow::Result<()> {
    let mut shell = Shell::new(fish_features)?;
    let mut history = History::new()?;
    let mut completer = AutoCompleter::new();
    let highlighter = SyntaxHighlighter::new();

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Show)?;

    let result = (|| -> anyhow::Result<()> {
        let mut input = String::new();
        let mut cursor_pos = 0;
        let mut history_idx = None;

        loop {
            // Display prompt
            display_prompt(&shell, &highlighter, &input, cursor_pos)?;

            // Handle input
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CTRL) => {
                        println!("^C");
                        input.clear();
                        cursor_pos = 0;
                        history_idx = None;
                        continue;
                    }
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CTRL) => {
                        if input.is_empty() {
                            println!("exit");
                            break;
                        }
                    }
                    KeyCode::Enter => {
                        println!();
                        if !input.trim().is_empty() {
                            history.add(&input)?;
                            shell.execute(&input)?;
                        }
                        input.clear();
                        cursor_pos = 0;
                        history_idx = None;
                        continue;
                    }
                    KeyCode::Tab => {
                        let completions = completer.complete(&input, cursor_pos, &shell);
                        if completions.len() == 1 {
                            let completion = &completions[0];
                            let before = &input[..cursor_pos];
                            let after = &input[cursor_pos..];
                            let last_word_start = before.rfind(' ').map(|i| i + 1).unwrap_or(0);
                            input = format!("{}{}{}", &before[..last_word_start], completion, after);
                            cursor_pos = last_word_start + completion.len();
                        } else if !completions.is_empty() {
                            println!();
                            for c in &completions {
                                println!("  {}", c);
                            }
                        }
                        continue;
                    }
                    KeyCode::Up => {
                        if let Some(entry) = history.prev() {
                            input = entry.to_string();
                            cursor_pos = input.len();
                            history_idx = Some(history.current_idx());
                        }
                        continue;
                    }
                    KeyCode::Down => {
                        if let Some(entry) = history.next() {
                            input = entry.to_string();
                            cursor_pos = input.len();
                            history_idx = Some(history.current_idx());
                        } else {
                            input.clear();
                            cursor_pos = 0;
                            history_idx = None;
                        }
                        continue;
                    }
                    KeyCode::Left => {
                        if cursor_pos > 0 {
                            cursor_pos -= 1;
                        }
                        continue;
                    }
                    KeyCode::Right => {
                        if cursor_pos < input.len() {
                            cursor_pos += 1;
                        }
                        continue;
                    }
                    KeyCode::Home => {
                        cursor_pos = 0;
                        continue;
                    }
                    KeyCode::End => {
                        cursor_pos = input.len();
                        continue;
                    }
                    KeyCode::Backspace => {
                        if cursor_pos > 0 {
                            input.remove(cursor_pos - 1);
                            cursor_pos -= 1;
                        }
                        continue;
                    }
                    KeyCode::Delete => {
                        if cursor_pos < input.len() {
                            input.remove(cursor_pos);
                        }
                        continue;
                    }
                    KeyCode::Char(c) => {
                        input.insert(cursor_pos, c);
                        cursor_pos += 1;
                        continue;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    })();

    execute!(stdout, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    result
}

fn display_prompt(
    shell: &Shell,
    highlighter: &SyntaxHighlighter,
    input: &str,
    cursor_pos: usize,
) -> anyhow::Result<()> {
    let mut stdout = io::stdout();

    // Get current directory
    let cwd = std::env::current_dir()?;
    let home = dirs::home_dir();
    let display_path = if let Some(home) = home {
        cwd.strip_prefix(&home)
            .map(|p| format!("~{}", p.display()))
            .unwrap_or_else(|_| cwd.display().to_string())
    } else {
        cwd.display().to_string()
    };

    // Prompt: user@host:path$
    let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "host".to_string());

    execute!(stdout, cursor::MoveToColumn(0), terminal::Clear(ClearType::CurrentLine))?;

    // Print prompt
    print!(
        "{}{}{}{}{} ",
        style::StyledContent::new(style::ContentStyle::new().with(Color::Green), user),
        "@".with(Color::White),
        style::StyledContent::new(style::ContentStyle::new().with(Color::Green), hostname),
        ":".with(Color::White),
        style::StyledContent::new(style::ContentStyle::new().with(Color::Blue), display_path),
    );

    // Highlight and display input
    let highlighted = highlighter.highlight(input);
    print!("{}", highlighted);

    // Move cursor to position
    let prompt_len = user.len() + 1 + hostname.len() + 1 + display_path.len() + 1;
    let total_len = prompt_len + cursor_pos;
    let (_, term_height) = terminal::size()?;
    let col = (total_len % terminal::size()?.0 as usize) as u16;
    let row = (total_len / terminal::size()?.0 as usize) as u16;
    execute!(stdout, cursor::MoveTo(col, term_height - 1 - row))?;

    stdout.flush()?;
    Ok(())
}
