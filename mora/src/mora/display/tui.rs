use crossterm::{
    event::{self, Event, KeyCode, KeyEvent as CrosstermKeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    buffer::Buffer as RatatuiBuffer,
    layout::{Rect, Size},
    style::{Color, Modifier, Style},
    Terminal,
};
use std::io;
use std::time::Duration;

use super::backend::{DisplayBackend, InputEvent, MouseKind, MoraRect, CellBuffer};
use super::event::{MoraKeyCode, MoraKeyEvent, MoraKeyModifiers};
use super::style::{MoraColor, MoraStyle};

pub struct TuiBackend {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TuiBackend {
    pub fn new() -> Self {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).expect("Failed to create terminal");
        Self { terminal }
    }

    pub fn draw<F>(&mut self, render_fn: F) -> Result<(), String>
    where
        F: FnOnce(&mut RatatuiBuffer, Rect),
    {
        self.terminal
            .draw(|frame| {
                let area = frame.area();
                render_fn(frame.buffer_mut(), area);
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

impl DisplayBackend for TuiBackend {
    fn init(&mut self) -> Result<(), String> {
        enable_raw_mode().map_err(|e| e.to_string())?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(|e| e.to_string())?;
        self.terminal.clear().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn cleanup(&mut self) -> Result<(), String> {
        disable_raw_mode().map_err(|e| e.to_string())?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen).map_err(|e| e.to_string())?;
        self.terminal.show_cursor().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn size(&self) -> (u16, u16) {
        let size = self.terminal.size().unwrap_or(Size::new(80, 24));
        (size.width, size.height)
    }

    fn clear(&mut self) {
        let _ = self.terminal.clear();
    }

    fn flush(&mut self) -> Result<(), String> {
        self.terminal.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn set_cell(&mut self, x: u16, y: u16, ch: char, style: MoraStyle) {
        let buf = self.terminal.current_buffer_mut();
        if x < buf.area.width && y < buf.area.height {
            let ratatui_style: Style = style.into();
            buf[(x, y)].set_char(ch).set_style(ratatui_style);
        }
    }

    fn set_line(&mut self, x: u16, y: u16, text: &str, style: MoraStyle) {
        let buf = self.terminal.current_buffer_mut();
        let ratatui_style: Style = style.into();
        for (i, ch) in text.chars().enumerate() {
            let cx = x + i as u16;
            if cx >= buf.area.width {
                break;
            }
            if y < buf.area.height {
                buf[(cx, y)].set_char(ch).set_style(ratatui_style);
            }
        }
    }

    fn poll_event(&mut self, timeout_ms: u64) -> Option<InputEvent> {
        if event::poll(Duration::from_millis(timeout_ms)).ok()? {
            match event::read().ok()? {
                Event::Key(key) => {
                    let mora_key: MoraKeyEvent = key.into();
                    Some(InputEvent::Key(mora_key))
                }
                Event::Resize(w, h) => Some(InputEvent::Resize(w, h)),
                Event::Mouse(mouse) => {
                    let kind = match mouse.kind {
                        event::MouseEventKind::Down(_) => MouseKind::Press,
                        event::MouseEventKind::Up(_) => MouseKind::Release,
                        event::MouseEventKind::Drag(_) => MouseKind::Drag,
                        event::MouseEventKind::ScrollUp => MouseKind::ScrollUp,
                        event::MouseEventKind::ScrollDown => MouseKind::ScrollDown,
                        _ => return None,
                    };
                    Some(InputEvent::Mouse {
                        x: mouse.column,
                        y: mouse.row,
                        kind,
                    })
                }
                Event::FocusGained => Some(InputEvent::FocusGained),
                Event::FocusLost => Some(InputEvent::FocusLost),
                _ => None,
            }
        } else {
            None
        }
    }

    fn hide_cursor(&mut self) {
        let _ = self.terminal.hide_cursor();
    }

    fn show_cursor(&mut self) {
        let _ = self.terminal.show_cursor();
    }

    fn set_cursor(&mut self, x: u16, y: u16) {
        let _ = self.terminal.set_cursor_position((x, y));
    }

    fn render_buffer(&mut self, buf: &CellBuffer) -> Result<(), String> {
        self.terminal
            .draw(|frame| {
                let area = frame.area();
                let ratatui_buf = frame.buffer_mut();
                for y in 0..buf.height.min(area.height) {
                    for x in 0..buf.width.min(area.width) {
                        let cell = buf.get(x, y);
                        let ratatui_style: Style = cell.style.into();
                        ratatui_buf[(x, y)].set_char(cell.ch).set_style(ratatui_style);
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
