//! Raw terminal handling for inline TUI (no alternate screen).

use std::io::{self, Write};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};

/// Terminal state manager for inline rendering.
pub struct Terminal {
    width: u16,
    height: u16,
}

impl Terminal {
    pub fn new() -> io::Result<Self> {
        let (width, height) = crossterm::terminal::size()?;
        enable_raw_mode()?;
        execute!(io::stdout(), EnableMouseCapture)?;
        Ok(Self { width, height })
    }

    pub fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    pub fn refresh_size(&mut self) -> io::Result<()> {
        let (w, h) = crossterm::terminal::size()?;
        self.width = w;
        self.height = h;
        Ok(())
    }

    pub fn clear_screen(&mut self) -> io::Result<()> {
        execute!(io::stdout(), Clear(ClearType::All), MoveTo(0, 0))
    }

    pub fn hide_cursor(&self) -> io::Result<()> {
        execute!(io::stdout(), Hide)
    }

    pub fn show_cursor(&self) -> io::Result<()> {
        execute!(io::stdout(), Show)
    }

    pub fn restore(&self) -> io::Result<()> {
        disable_raw_mode()?;
        execute!(io::stdout(), DisableMouseCapture, Show)
    }

    pub fn write_str(&self, s: &str) -> io::Result<()> {
        let mut stdout = io::stdout();
        stdout.write_all(s.as_bytes())?;
        stdout.flush()
    }
}
