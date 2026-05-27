use super::event::MoraKeyEvent;
use super::style::MoraStyle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoraRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl MoraRect {
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    Key(MoraKeyEvent),
    Resize(u16, u16),
    Mouse { x: u16, y: u16, kind: MouseKind },
    FocusGained,
    FocusLost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseKind {
    Press,
    Release,
    Drag,
    ScrollUp,
    ScrollDown,
}

#[derive(Debug, Clone)]
pub struct Cell {
    pub ch: char,
    pub style: MoraStyle,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: MoraStyle::default(),
        }
    }
}

pub struct CellBuffer {
    pub width: u16,
    pub height: u16,
    pub cells: Vec<Cell>,
}

impl CellBuffer {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::default(); (width as usize) * (height as usize)],
        }
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = Cell::default();
        }
    }

    pub fn set_cell(&mut self, x: u16, y: u16, ch: char, style: MoraStyle) {
        if x < self.width && y < self.height {
            let idx = (y as usize) * (self.width as usize) + (x as usize);
            self.cells[idx] = Cell { ch, style };
        }
    }

    pub fn set_line(&mut self, x: u16, y: u16, text: &str, style: MoraStyle) {
        for (i, ch) in text.chars().enumerate() {
            self.set_cell(x + i as u16, y, ch, style);
        }
    }

    pub fn get(&self, x: u16, y: u16) -> &Cell {
        let idx = (y as usize) * (self.width as usize) + (x as usize);
        &self.cells[idx]
    }
}

pub trait DisplayBackend {
    fn init(&mut self) -> Result<(), String>;
    fn cleanup(&mut self) -> Result<(), String>;
    fn size(&self) -> (u16, u16);
    fn clear(&mut self);
    fn flush(&mut self) -> Result<(), String>;
    fn set_cell(&mut self, x: u16, y: u16, ch: char, style: MoraStyle);
    fn set_line(&mut self, x: u16, y: u16, text: &str, style: MoraStyle);
    fn poll_event(&mut self, timeout_ms: u64) -> Option<InputEvent>;
    fn hide_cursor(&mut self);
    fn show_cursor(&mut self);
    fn set_cursor(&mut self, x: u16, y: u16);
    fn render_buffer(&mut self, buf: &CellBuffer) -> Result<(), String>;
}
