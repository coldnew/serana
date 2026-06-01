use crossterm::{
    cursor::SetCursorStyle,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use display_protocol::buffer::ScreenBuffer;
use display_protocol::{CursorStyle, FrameUpdate, Grid, InputEvent};
use ratatui::{backend::CrosstermBackend, buffer::Buffer as RatatuiBuffer, layout::Rect, Terminal};
use std::io;

/// Shared TUI terminal for mora and serana.
///
/// Wraps ratatui Terminal with crossterm backend. Provides:
/// - Terminal lifecycle (init/cleanup with raw mode, alternate screen)
/// - Drawing via ratatui buffer or display-protocol FrameUpdate
/// - Input polling that produces display-protocol InputEvent
pub struct TuiTerminal {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TuiTerminal {
    /// Create a new TUI terminal. Call `init()` before use.
    pub fn new() -> Result<Self, io::Error> {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    /// Initialize terminal: enable raw mode, enter alternate screen, clear.
    pub fn init(&mut self) -> Result<(), io::Error> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        self.terminal.clear()?;
        Ok(())
    }

    /// Initialize terminal with mouse capture support.
    pub fn init_with_mouse(&mut self) -> Result<(), io::Error> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;
        self.terminal.clear()?;
        Ok(())
    }

    /// Restore terminal: disable raw mode, leave alternate screen, show cursor.
    pub fn cleanup(&mut self) -> Result<(), io::Error> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            SetCursorStyle::DefaultUserShape,
            crossterm::event::DisableMouseCapture,
            crossterm::cursor::Show
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    /// Get terminal size (width, height).
    pub fn size(&self) -> (u16, u16) {
        let size = self
            .terminal
            .size()
            .unwrap_or(ratatui::layout::Size::new(80, 24));
        (size.width, size.height)
    }

    /// Draw using a closure that receives the ratatui buffer.
    pub fn draw<F>(&mut self, render_fn: F) -> Result<(), io::Error>
    where
        F: FnOnce(&mut RatatuiBuffer, Rect),
    {
        self.terminal.draw(|frame| {
            let area = frame.area();
            render_fn(frame.buffer_mut(), area);
        })?;
        Ok(())
    }

    pub fn render_frame(&mut self, frame: &FrameUpdate) -> Result<(), io::Error> {
        let grid = &frame.grid;
        self.terminal.draw(|f| {
            let area = f.area();
            let buf = f.buffer_mut();
            for y in 0..grid.height.min(area.height) {
                for x in 0..grid.width.min(area.width) {
                    let cell = grid.get(x, y);
                    let ratatui_cell = buf.get_mut(x, y);
                    ratatui_cell.set_symbol(&cell.ch.to_string());
                    ratatui_cell.set_style(crate::conversions::style_to_ratatui(cell.style));
                }
            }
        })?;
        Ok(())
    }

    pub fn render_screen_buffer(&mut self, screen: &ScreenBuffer) -> Result<(), io::Error> {
        self.terminal.draw(|frame| {
            let area = frame.area();
            let buf = frame.buffer_mut();
            for y in 0..screen.height.min(area.height) {
                for x in 0..screen.width.min(area.width) {
                    let cell = screen.get(x, y);
                    let ratatui_cell = &mut buf[(x, y)];
                    ratatui_cell.set_symbol(&cell.ch.to_string());
                    ratatui_cell.set_fg(crate::conversions::color_to_ratatui(cell.fg));
                    ratatui_cell.set_bg(crate::conversions::color_to_ratatui(cell.bg));
                }
            }
        })?;
        Ok(())
    }

    /// Poll for input with timeout. Returns display-protocol InputEvent.
    pub fn poll_input(&mut self, timeout_ms: u64) -> Option<InputEvent> {
        super::input::poll_input(timeout_ms)
    }

    /// Access the underlying ratatui terminal for custom rendering.
    pub fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<io::Stdout>> {
        &mut self.terminal
    }

    /// Hide cursor.
    pub fn hide_cursor(&mut self) {
        let _ = self.terminal.hide_cursor();
    }

    /// Show cursor.
    pub fn show_cursor(&mut self) {
        let _ = self.terminal.show_cursor();
    }

    /// Set cursor position.
    pub fn set_cursor(&mut self, x: u16, y: u16) {
        let _ = self.terminal.set_cursor_position((x, y));
    }

    /// Set hardware cursor style when supported by the terminal emulator.
    pub fn set_cursor_style(&mut self, style: CursorStyle) {
        let style = match style {
            CursorStyle::Block => SetCursorStyle::SteadyBlock,
            CursorStyle::Underline => SetCursorStyle::SteadyUnderScore,
            CursorStyle::Bar => SetCursorStyle::SteadyBar,
        };
        let _ = execute!(self.terminal.backend_mut(), style);
    }

    /// Clear the terminal.
    pub fn clear(&mut self) {
        let _ = self.terminal.clear();
    }

    /// Flush the terminal.
    pub fn flush(&mut self) -> Result<(), io::Error> {
        self.terminal.flush()
    }
}

/// Install a panic hook that restores the terminal before the default panic handler.
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(io::stdout(), LeaveAlternateScreen);
        let _ = crossterm::execute!(io::stdout(), SetCursorStyle::DefaultUserShape);
        let _ = crossterm::execute!(io::stdout(), crossterm::cursor::Show);
        default_hook(info);
    }));
}

/// Convert a display-protocol Grid to a ratatui Buffer (for advanced use cases).
pub fn grid_to_ratatui_buf(grid: &Grid) -> RatatuiBuffer {
    let area = Rect::new(0, 0, grid.width, grid.height);
    let mut buf = RatatuiBuffer::empty(area);
    for y in 0..grid.height {
        for x in 0..grid.width {
            let cell = grid.get(x, y);
            let ratatui_cell = buf.get_mut(x, y);
            ratatui_cell.set_symbol(&cell.ch.to_string());
            ratatui_cell.set_style(crate::conversions::style_to_ratatui(cell.style));
        }
    }
    buf
}
