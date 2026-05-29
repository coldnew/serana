use anyhow::Result;
use clap::Parser as ClapParser;
use display_tui::{install_panic_hook, TuiTerminal};
use ratatui::style::{Modifier, Style as RatStyle};
use std::path::PathBuf;

use vivi::buffer::Buffer;
use vivi::editor::Editor;

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
        Some(path) => Buffer::from_file(path)?,
        None => Buffer::new(),
    };

    // Set up terminal
    install_panic_hook();
    let mut terminal = TuiTerminal::new()?;
    terminal.init()?;

    let mut editor = Editor::new(buffer);

    // Execute startup command if given
    if let Some(cmd) = &cli.command {
        for ch in cmd.chars() {
            editor.handle_key(display_protocol::KeyEvent::char(ch));
        }
        if cmd.starts_with('q') {
            editor.handle_key(display_protocol::KeyEvent::new(
                display_protocol::KeyCode::Enter,
                display_protocol::KeyModifiers::EMPTY,
            ));
        }
    }

    // If + flag, go to last line
    if cli.plus {
        editor.handle_key(display_protocol::KeyEvent::char('G'));
    }

    // Hide hardware cursor — we render a software cursor on the ScreenBuffer
    // to have full control over shape and blink rate per mode.
    terminal.hide_cursor();

    let mut tick: u64 = 0;

    // Main event loop
    loop {
        tick = tick.wrapping_add(1);
        // Vim default cursor blink: ~600ms on, ~600ms off (at 50ms poll)
        let cursor_blink_on = (tick / 12) % 2 == 0;

        // Update terminal size
        let (w, h) = terminal.size();
        editor.set_term_size(w, h);

        // Render via ScreenBuffer → paint_into → ratatui
        let mut screen = vivi::render::render(&editor, w, h);

        // Software cursor: per-mode shape, blinks at vim rate
        vivi::render::draw_mode_cursor(&mut screen, &editor, cursor_blink_on);

        // Paint ScreenBuffer → ratatui, copying all cell attributes
        terminal.terminal().draw(|frame| {
            let area = frame.area();
            let buf = frame.buffer_mut();
            for y in 0..screen.height.min(area.height) {
                for x in 0..screen.width.min(area.width) {
                    let sc = screen.get(x, y);
                    let rc = &mut buf[(x, y)];
                    rc.set_symbol(&sc.ch.to_string());
                    let fg = display_tui::conversions::color_to_ratatui(sc.fg);
                    let bg = display_tui::conversions::color_to_ratatui(sc.bg);
                    let mut style = RatStyle::default().fg(fg).bg(bg);
                    if sc.bold {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    if sc.italic {
                        style = style.add_modifier(Modifier::ITALIC);
                    }
                    if sc.underline {
                        style = style.add_modifier(Modifier::UNDERLINED);
                        if let Some(uc) = sc.underline_color {
                            style = style.underline_color(display_tui::conversions::color_to_ratatui(uc));
                        }
                    }
                    if sc.dim {
                        style = style.add_modifier(Modifier::DIM);
                    }
                    if sc.reverse {
                        style = style.add_modifier(Modifier::REVERSED);
                    }
                    if sc.blink {
                        style = style.add_modifier(Modifier::SLOW_BLINK);
                    }
                    if sc.strikethrough {
                        style = style.add_modifier(Modifier::CROSSED_OUT);
                    }
                    rc.set_style(style);
                }
            }
        })?;

        // Ensure hardware cursor stays hidden after ratatui draw
        terminal.hide_cursor();

        // Poll for input
        match terminal.poll_input(50) {
            Some(display_protocol::InputEvent::Key(key)) => {
                tick = 0; // Reset blink — cursor stays solid while typing
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
