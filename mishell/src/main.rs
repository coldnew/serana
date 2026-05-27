use clap::Parser as ClapParser;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    style::{self, Color, Stylize},
    terminal::{self, ClearType},
};
use std::io::{self, Write};

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

enum InputMode {
    Normal,
    ReverseSearch { query: String, match_idx: usize },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        let mut shell = Shell::new(!cli.no_fish)?;
        shell.set_interactive(false);
        shell.execute(&cmd)?;
        return Ok(());
    }

    run_interactive(!cli.no_fish)
}

fn run_interactive(fish_features: bool) -> anyhow::Result<()> {
    let mut shell = Shell::new(fish_features)?;
    let mut hist = History::new()?;
    let completer = AutoCompleter::new();
    let highlighter = SyntaxHighlighter::new();

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();

    let result = (|| -> anyhow::Result<()> {
        let mut input = String::new();
        let mut cursor_pos: usize = 0;
        let mut mode = InputMode::Normal;
        let mut last_completion_prefix = String::new();
        let mut last_completions: Vec<String> = Vec::new();
        let mut tab_count = 0;

        loop {
            match &mode {
                InputMode::Normal => {
                    display_prompt(&shell, &highlighter, &input, cursor_pos, fish_features, &hist)?;
                }
                InputMode::ReverseSearch { query, match_idx: _ } => {
                    display_reverse_prompt(&shell, query)?;
                }
            }

            if let Event::Key(key) = event::read()? {
                match &mut mode {
                    InputMode::ReverseSearch { query, match_idx } => {
                        match key.code {
                            KeyCode::Esc => {
                                mode = InputMode::Normal;
                                // Restore original input
                            }
                            KeyCode::Enter => {
                                if let Some(entry) = hist.search(query).get(*match_idx).cloned() {
                                    input = entry.to_string();
                                    cursor_pos = input.len();
                                }
                                mode = InputMode::Normal;
                                println!();
                            }
                            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Cycle to next match
                                let matches = hist.search(query);
                                if !matches.is_empty() {
                                    *match_idx = (*match_idx + 1) % matches.len();
                                }
                            }
                            KeyCode::Backspace => {
                                query.pop();
                                *match_idx = 0;
                                if query.is_empty() {
                                    mode = InputMode::Normal;
                                }
                            }
                            KeyCode::Char(c) => {
                                query.push(c);
                                *match_idx = 0;
                            }
                            _ => {}
                        }
                        continue;
                    }
                    InputMode::Normal => {}
                }

                match key.code {
                    // Ctrl+C - cancel
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        execute!(stdout, cursor::MoveToColumn(0), terminal::Clear(ClearType::CurrentLine))?;
                        print!("{}^C", style::StyledContent::new(style::ContentStyle::new().with(Color::Red), "^C"));
                        println!();
                        input.clear();
                        cursor_pos = 0;
                        last_completion_prefix.clear();
                        last_completions.clear();
                        tab_count = 0;
                        continue;
                    }
                    // Ctrl+D - exit if empty
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) && input.is_empty() => {
                        println!();
                        break;
                    }
                    // Ctrl+Z - suspend (limited support)
                    KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        execute!(stdout, cursor::MoveToColumn(0), terminal::Clear(ClearType::CurrentLine))?;
                        print!("{}^Z", style::StyledContent::new(style::ContentStyle::new().with(Color::Yellow), "^Z"));
                        println!();
                        input.clear();
                        cursor_pos = 0;
                        continue;
                    }
                    // Ctrl+R - reverse history search
                    KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        mode = InputMode::ReverseSearch {
                            query: String::new(),
                            match_idx: 0,
                        };
                        continue;
                    }
                    // Ctrl+A - beginning of line
                    KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        cursor_pos = 0;
                        continue;
                    }
                    // Ctrl+E - end of line
                    KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        cursor_pos = input.len();
                        continue;
                    }
                    // Ctrl+K - kill to end of line
                    KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        input.truncate(cursor_pos);
                        continue;
                    }
                    // Ctrl+U - kill to beginning of line
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        input = input[cursor_pos..].to_string();
                        cursor_pos = 0;
                        continue;
                    }
                    // Ctrl+W - delete word backwards
                    KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if cursor_pos > 0 {
                            let before = &input[..cursor_pos];
                            let trimmed = before.trim_end();
                            let new_pos = trimmed.rfind(' ').map(|i| i + 1).unwrap_or(0);
                            input = format!("{}{}", &input[..new_pos], &input[cursor_pos..]);
                            cursor_pos = new_pos;
                        }
                        continue;
                    }
                    // Ctrl+L - clear screen
                    KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        execute!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                        continue;
                    }
                    // Enter - execute
                    KeyCode::Enter => {
                        execute!(stdout, cursor::MoveToColumn(0), terminal::Clear(ClearType::CurrentLine))?;
                        // Print prompt + input (final, colored)
                        print_prompt(&shell, &input);
                        println!();

                        if !input.trim().is_empty() {
                            hist.add(&input)?;
                            shell.execute(&input)?;
                        }
                        input.clear();
                        cursor_pos = 0;
                        last_completion_prefix.clear();
                        last_completions.clear();
                        tab_count = 0;
                        continue;
                    }
                    // Tab - completion
                    KeyCode::Tab => {
                        let word_start = input[..cursor_pos].rfind(' ').map(|i| i + 1).unwrap_or(0);
                        let current_word = &input[word_start..cursor_pos];

                        if current_word != last_completion_prefix || last_completions.is_empty() {
                            last_completions = completer.complete(&input, cursor_pos, &shell);
                            last_completion_prefix = current_word.to_string();
                            tab_count = 0;
                        }

                        if last_completions.is_empty() {
                            continue;
                        }

                        if last_completions.len() == 1 {
                            let completion = &last_completions[0];
                            input = format!("{}{}{}", &input[..word_start], completion, &input[cursor_pos..]);
                            cursor_pos = word_start + completion.len();
                            last_completion_prefix.clear();
                            last_completions.clear();
                            tab_count = 0;
                        } else {
                            tab_count += 1;
                            if tab_count == 1 {
                                // First tab: complete common prefix
                                let common = common_prefix(&last_completions);
                                if common.len() > current_word.len() {
                                    input = format!("{}{}{}", &input[..word_start], common, &input[cursor_pos..]);
                                    cursor_pos = word_start + common.len();
                                    last_completion_prefix = common;
                                }
                            } else {
                                // Second tab: show all completions
                                execute!(stdout, cursor::MoveToColumn(0), terminal::Clear(ClearType::CurrentLine))?;
                                print_prompt(&shell, &input);
                                println!();
                                // Print completions in columns
                                let term_width = terminal::size()?.0 as usize;
                                let max_len = last_completions.iter().map(|c| c.len()).max().unwrap_or(0) + 2;
                                let cols = (term_width / max_len).max(1);
                                for (i, c) in last_completions.iter().enumerate() {
                                    print!("{:<width$}", c, width = max_len);
                                    if (i + 1) % cols == 0 {
                                        println!();
                                    }
                                }
                                if !last_completions.len().is_multiple_of(cols) {
                                    println!();
                                }
                                tab_count = 0;
                            }
                        }
                        continue;
                    }
                    // Up - history prev
                    KeyCode::Up => {
                        if let Some(entry) = hist.prev() {
                            input = entry.to_string();
                            cursor_pos = input.len();
                        }
                        last_completion_prefix.clear();
                        last_completions.clear();
                        tab_count = 0;
                        continue;
                    }
                    // Down - history next
                    KeyCode::Down => {
                        if let Some(entry) = hist.next_entry() {
                            input = entry.to_string();
                            cursor_pos = input.len();
                        } else {
                            input.clear();
                            cursor_pos = 0;
                        }
                        last_completion_prefix.clear();
                        last_completions.clear();
                        tab_count = 0;
                        continue;
                    }
                    // Right arrow - accept autosuggestion OR move cursor
                    KeyCode::Right => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            // Ctrl+Right - move by word
                            let after = &input[cursor_pos..];
                            if let Some(pos) = after.find(|c: char| c.is_whitespace()) {
                                cursor_pos += pos + 1;
                            } else {
                                cursor_pos = input.len();
                            }
                        } else if cursor_pos == input.len() && fish_features {
                            // Accept autosuggestion from history
                            if let Some(suggestion) = highlighter.get_autosuggestion(&input, hist.entries()) {
                                input = suggestion;
                                cursor_pos = input.len();
                            }
                        } else if cursor_pos < input.len() {
                            cursor_pos += 1;
                        }
                        continue;
                    }
                    // Left - move cursor
                    KeyCode::Left => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            // Ctrl+Left - move by word
                            let before = &input[..cursor_pos];
                            let trimmed = before.trim_end();
                            cursor_pos = trimmed.rfind(' ').unwrap_or(0);                        } else {
                            cursor_pos = cursor_pos.saturating_sub(1);
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
                            last_completion_prefix.clear();
                            last_completions.clear();
                            tab_count = 0;
                        }
                        continue;
                    }
                    KeyCode::Delete => {
                        if cursor_pos < input.len() {
                            input.remove(cursor_pos);
                            last_completion_prefix.clear();
                            last_completions.clear();
                            tab_count = 0;
                        }
                        continue;
                    }
                    KeyCode::Char(c) => {
                        input.insert(cursor_pos, c);
                        cursor_pos += 1;
                        last_completion_prefix.clear();
                        last_completions.clear();
                        tab_count = 0;
                        continue;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    })();

    terminal::disable_raw_mode()?;
    result
}

fn print_prompt(_shell: &Shell, input: &str) {
    let cwd = std::env::current_dir().unwrap_or_default();
    let home = dirs::home_dir();
    let display_path = if let Some(ref home) = home {
        cwd.strip_prefix(home)
            .map(|p| format!("~{}", p.display()))
            .unwrap_or_else(|_| cwd.display().to_string())
    } else {
        cwd.display().to_string()
    };

    let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "host".to_string());

    print!(
        "{}{}{}{}{} ",
        user.clone().with(Color::Green),
        "@".with(Color::DarkGrey),
        hostname.clone().with(Color::Green),
        ":".with(Color::DarkGrey),
        display_path.clone().with(Color::Blue),
    );
    print!("{}", input);
}

fn display_prompt(
    _shell: &Shell,
    highlighter: &SyntaxHighlighter,
    input: &str,
    cursor_pos: usize,
    fish_features: bool,
    hist: &History,
) -> anyhow::Result<()> {
    let mut stdout = io::stdout();

    let cwd = std::env::current_dir()?;
    let home = dirs::home_dir();
    let display_path = if let Some(ref home) = home {
        cwd.strip_prefix(home)
            .map(|p| format!("~{}", p.display()))
            .unwrap_or_else(|_| cwd.display().to_string())
    } else {
        cwd.display().to_string()
    };

    let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "host".to_string());

    // Get right prompt (git branch)
    let right_prompt = get_right_prompt();

    execute!(stdout, cursor::MoveToColumn(0), terminal::Clear(ClearType::CurrentLine))?;

    // Prompt
    let prompt_display_len = user.len() + 1 + hostname.len() + 1 + display_path.len() + 1;

    // Print left prompt with colors
    print!(
        "{}{}{}{}{} ",
        user.clone().with(Color::Green),
        "@".with(Color::DarkGrey),
        hostname.clone().with(Color::Green),
        ":".with(Color::DarkGrey),
        display_path.clone().with(Color::Blue),
    );

    // Highlight input
    let highlighted = highlighter.highlight(input);
    print!("{}", highlighted);

    // Fish-style autosuggestion (grey ghost text)
    if fish_features && cursor_pos == input.len() && !input.is_empty() {
        if let Some(suggestion) = highlighter.get_autosuggestion(input, hist.entries()) {
            let remaining = &suggestion[input.len()..];
            if !remaining.is_empty() {
                print!("{}", remaining.with(Color::DarkGrey));
            }
        }
    }

    // Print right prompt
    let term_width = terminal::size()?.0 as usize;
    let input_display_len = prompt_display_len + input.len();
    let right_len = right_prompt.len();
    if input_display_len + right_len + 2 < term_width {
        let spaces = term_width - input_display_len - right_len;
        print!("{:spaces$}{}", "", right_prompt.with(Color::DarkGrey), spaces = spaces);
    }

    // Position cursor
    let total_col = (prompt_display_len + cursor_pos) as u16;
    execute!(stdout, cursor::MoveToColumn(total_col))?;

    stdout.flush()?;
    Ok(())
}

fn display_reverse_prompt(_shell: &Shell, query: &str) -> anyhow::Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, cursor::MoveToColumn(0), terminal::Clear(ClearType::CurrentLine))?;
    print!(
        "{} {}{}{}",
        "(reverse-i-search)".with(Color::Yellow),
        "'".with(Color::DarkGrey),
        query.with(Color::Cyan),
        "'".with(Color::DarkGrey),
    );
    stdout.flush()?;
    Ok(())
}

fn get_right_prompt() -> String {
    // Try to get git branch
    if let Ok(output) = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .output()
    {
        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !branch.is_empty() {
                // Check for dirty status
                let dirty = std::process::Command::new("git")
                    .args(["status", "--porcelain"])
                    .output()
                    .map(|o| !o.stdout.is_empty())
                    .unwrap_or(false);

                let indicator = if dirty { "*" } else { "" };
                return format!("{}{} ", branch, indicator);
            }
        }
    }
    String::new()
}

fn common_prefix(strings: &[String]) -> String {
    if strings.is_empty() {
        return String::new();
    }

    let first = &strings[0];
    let mut end = first.len();

    for s in &strings[1..] {
        end = end.min(s.len());
        for i in 0..end {
            if first.as_bytes()[i] != s.as_bytes()[i] {
                end = i;
                break;
            }
        }
    }

    first[..end].to_string()
}
