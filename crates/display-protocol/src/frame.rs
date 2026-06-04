use super::types::Style;
use serde::{Deserialize, Serialize};

/// A single cell in the character grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: Style::default(),
        }
    }
}

/// Character grid representing the rendered content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid {
    pub width: u16,
    pub height: u16,
    pub cells: Vec<Cell>,
}

impl Grid {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::default(); width as usize * height as usize],
        }
    }

    pub fn get(&self, x: u16, y: u16) -> Cell {
        if x < self.width && y < self.height {
            self.cells[y as usize * self.width as usize + x as usize]
        } else {
            Cell::default()
        }
    }

    pub fn set(&mut self, x: u16, y: u16, cell: Cell) {
        if x < self.width && y < self.height {
            self.cells[y as usize * self.width as usize + x as usize] = cell;
        }
    }

    pub fn clear(&mut self) {
        self.cells.fill(Cell::default());
    }
}

/// Cursor state for the frame
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CursorState {
    pub x: u16,
    pub y: u16,
    pub visible: bool,
    /// Cursor style hint for the frontend
    pub style: CursorStyle,
}

impl Default for CursorState {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            visible: true,
            style: CursorStyle::Block,
        }
    }
}

/// Cursor rendering style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorStyle {
    Block,
    Underline,
    Bar,
}

/// Status line content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusLine {
    pub left: Vec<(String, Style)>,
    pub right: Vec<(String, Style)>,
}

impl Default for StatusLine {
    fn default() -> Self {
        Self {
            left: Vec::new(),
            right: Vec::new(),
        }
    }
}

/// A complete frame update from core to frontend
///
/// This is the primary data channel. The core renders its state into a
/// FrameUpdate and sends it to the frontend for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameUpdate {
    /// The character grid
    pub grid: Grid,
    /// Cursor position and visibility
    pub cursor: CursorState,
    /// Status line content
    pub status_line: StatusLine,
    /// Command line text (bottom row, e.g. ":" prompts, search)
    pub command_line: Option<String>,
    /// Help bar text (top row, mode indicator)
    pub help_bar: Option<String>,
    /// Whether a full redraw is needed (vs incremental diff)
    pub full_redraw: bool,
}

impl FrameUpdate {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            grid: Grid::new(width, height),
            cursor: CursorState::default(),
            status_line: StatusLine::default(),
            command_line: None,
            help_bar: None,
            full_redraw: true,
        }
    }
}
