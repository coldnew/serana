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
}
