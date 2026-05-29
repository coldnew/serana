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
        let last_line = editor.buffer().line_count().saturating_sub(1);
        editor.handle_key(display_protocol::KeyEvent::new(
            display_protocol::KeyCode::Char('G'),
            display_protocol::KeyModifiers::EMPTY,
        ));
        let _ = last_line; // handled by G
    }
    let mut tick: u64 = 0;
    // Main event loop with cursor blink (~600ms on/off, vim default)
    loop {
        tick = tick.wrapping_add(1);
        let cursor_blink_on = (tick / 12) % 2 == 0;
        // Update terminal size
        let (w, h) = terminal.size();
        editor.set_term_size(w, h);

        // Render
        let screen = render::render(&editor, w, h);
        terminal.terminal().draw(|frame| {
            let area = frame.area();
            let buf = frame.buffer_mut();
            for y in 0..screen.height.min(area.height) {
                for x in 0..screen.width.min(area.width) {
                    let cell = screen.get(x, y);
                    let ratatui_cell = &mut buf[(x, y)];
                    ratatui_cell.set_symbol(&cell.ch.to_string());
                    let proto_fg = display_protocol::Color::new(cell.fg.r, cell.fg.g, cell.fg.b);
                    let proto_bg = display_protocol::Color::new(cell.bg.r, cell.bg.g, cell.bg.b);
                    ratatui_cell.set_fg(display_tui::conversions::color_to_ratatui(proto_fg));
                    ratatui_cell.set_bg(display_tui::conversions::color_to_ratatui(proto_bg));
                }
            }
            // Position cursor
            let cursor = editor.cursor();
            let scroll = editor.scroll();
            let screen_col = cursor.col.saturating_sub(scroll.col);
            let screen_row = cursor.row.saturating_sub(scroll.row);
            let gutter = render::line_number_width(editor.buffer().line_count(), w as usize);
            let cursor_x = (gutter + screen_col) as u16;
            let cursor_y = screen_row as u16;
            if cursor_blink_on && cursor_x < area.width && cursor_y < area.height {
                buf[(cursor_x, cursor_y)].set_bg(
                    ratatui::style::Color::Rgb(204, 204, 204)
                );
                buf[(cursor_x, cursor_y)].set_fg(
                    ratatui::style::Color::Rgb(0, 0, 0)
                );
            }
        })?;

        // Software cursor only — hide hardware cursor to avoid double blink
        terminal.hide_cursor();

        // Poll for input
        match terminal.poll_input(50) {
            Some(display_protocol::InputEvent::Key(key)) => {
                tick = 0; // reset blink — cursor visible while typing
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
