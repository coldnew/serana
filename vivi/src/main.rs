use anyhow::Result;
use clap::Parser as ClapParser;
use display_tui::{install_panic_hook, TuiTerminal};
use std::path::PathBuf;

mod buffer;
mod cursor;
mod editor;
mod mode;
mod render;

use buffer::Buffer;
use editor::Editor;
use mode::Mode;

#[derive(ClapParser)]
#[command(name = "vivi")]
#[command(about = "A vim-like terminal text editor")]
#[command(version)]
struct Cli {
    /// File to open
    file: Option<PathBuf>,

    /// Start at the last line of the file
    #[arg(short = '+')]
    plus: bool,

    /// Execute command after opening
    #[arg(short = 'c')]
    command: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load or create buffer
    let buffer = match &cli.file {
        Some(path) => {
            if path.exists() {
                Buffer::from_file(path)?
            } else {
                let mut buf = Buffer::new();
                buf.save_as(path)?;
                buf
            }
        }
        None => Buffer::new(),
    };

    // Set up terminal
    install_panic_hook();
    let mut terminal = TuiTerminal::new()?;
    terminal.init()?;

    let mut editor = Editor::new(buffer);

    // Execute startup command if given
    if let Some(cmd) = &cli.command {
        editor.handle_key(display_protocol::KeyEvent::new(
            display_protocol::KeyCode::Char(':'),
            display_protocol::KeyModifiers::EMPTY,
        ));
        for ch in cmd.chars() {
            editor.handle_key(display_protocol::KeyEvent::char(ch));
        }
        editor.handle_key(display_protocol::KeyEvent::new(
            display_protocol::KeyCode::Enter,
            display_protocol::KeyModifiers::EMPTY,
        ));
    }

    // If + flag, go to last line
    if cli.plus {
        editor.handle_key(display_protocol::KeyEvent::new(
            display_protocol::KeyCode::Char('G'),
            display_protocol::KeyModifiers::EMPTY,
        ));
    }

    // Main event loop
    loop {
        // Update terminal size
        let (w, h) = terminal.size();
        editor.set_term_size(w, h);

        let screen = render::render(&editor, w, h);
        terminal.render_screen_buffer(&screen)?;

        let (cursor_x, cursor_y) = render::cursor_position(&editor, w, h);
        terminal.set_cursor_style(cursor_style(editor.mode()));
        terminal.set_cursor(cursor_x, cursor_y);
        terminal.show_cursor();

        // Poll for input
        match terminal.poll_input(50) {
            Some(display_protocol::InputEvent::Key(key)) => {
                editor.handle_key(key);
            }
            Some(display_protocol::InputEvent::Resize { width, height }) => {
                editor.set_term_size(width, height);
                terminal.clear();
            }
            _ => {}
        }

        if editor.should_quit() {
            break;
        }
    }

    terminal.cleanup()?;
    Ok(())
}

fn cursor_style(mode: Mode) -> display_protocol::CursorStyle {
    match mode {
        Mode::Insert | Mode::Command => display_protocol::CursorStyle::Bar,
        Mode::Replace => display_protocol::CursorStyle::Underline,
        Mode::Normal | Mode::Visual | Mode::VisualLine => display_protocol::CursorStyle::Block,
    }
}
