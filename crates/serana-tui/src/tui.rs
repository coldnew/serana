//! Main TUI engine with differential rendering.

use std::io;
use std::time::{Duration, Instant};

use crate::component::{Component, Container};
use crate::terminal::Terminal;

pub struct Tui {
    terminal: Terminal,
    root: Container,
    prev_lines: Vec<String>,
    needs_render: bool,
    last_render: Instant,
    min_interval: Duration,
}

impl Tui {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            terminal: Terminal::new()?,
            root: Container::new(),
            prev_lines: Vec::new(),
            needs_render: true,
            last_render: Instant::now() - Duration::from_secs(1),
            min_interval: Duration::from_millis(16),
        })
    }

    pub fn root_mut(&mut self) -> &mut Container {
        &mut self.root
    }

    pub fn request_render(&mut self) {
        self.needs_render = true;
    }

    pub fn render(&mut self) -> io::Result<()> {
        if !self.needs_render && self.last_render.elapsed() < self.min_interval {
            return Ok(());
        }

        self.terminal.refresh_size()?;
        let width = self.terminal.size().0 as usize;
        let new_lines = self.root.render(width);

        let max = new_lines.len().max(self.prev_lines.len());
        let mut buf = String::with_capacity(max * 80);

        for i in 0..max {
            let new = new_lines.get(i).map(String::as_str).unwrap_or("");
            let old = self.prev_lines.get(i).map(String::as_str).unwrap_or("");
            if new != old {
                // Move cursor to row i+1, col 1, write, clear rest of line.
                use std::fmt::Write;
                let _ = write!(buf, "\x1b[{};1H{}\x1b[K", i + 1, new);
            }
        }

        // Clear leftover rows if content shrank.
        for i in new_lines.len()..self.prev_lines.len() {
            use std::fmt::Write;
            let _ = write!(buf, "\x1b[{};1H\x1b[K", i + 1);
        }

        if !buf.is_empty() {
            self.terminal.write_str(&buf)?;
        }

        self.prev_lines = new_lines;
        self.needs_render = false;
        self.last_render = Instant::now();
        Ok(())
    }

    pub fn clear_screen(&mut self) -> io::Result<()> {
        self.terminal.clear_screen()?;
        self.prev_lines.clear();
        Ok(())
    }

    pub fn hide_cursor(&self) -> io::Result<()> {
        self.terminal.hide_cursor()
    }

    pub fn show_cursor(&self) -> io::Result<()> {
        self.terminal.show_cursor()
    }

    pub fn restore(&self) -> io::Result<()> {
        self.terminal.restore()
    }

    pub fn terminal_size(&self) -> (u16, u16) {
        self.terminal.size()
    }
}
