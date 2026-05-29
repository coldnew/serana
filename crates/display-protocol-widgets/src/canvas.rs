use display_protocol::{CanvasNode, Color, UiNode};

/// A canvas for custom pixel-level drawing.
///
/// TUI backend: reserves the given width×height rect but leaves cells blank.
/// WGPU backend: renders `frame_id` content (images, graphs, rich graphics).
///
/// ```ignore
/// Canvas::new(80, 24).build()
/// Canvas::new(400, 300).frame("minimap-1").build()
/// Canvas::new(200, 100).bg(Color::new(10, 10, 20)).build()
/// ```
#[derive(Debug, Clone)]
pub struct Canvas {
    width: u16,
    height: u16,
    frame_id: String,
    bg: Color,
}

impl Canvas {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            frame_id: String::new(),
            bg: Color::BLACK,
        }
    }

    /// Set the frame ID for WGPU rendering.
    pub fn frame(mut self, id: impl Into<String>) -> Self { self.frame_id = id.into(); self }

    /// Set background color.
    pub fn bg(mut self, c: Color) -> Self { self.bg = c; self }

    pub fn build(self) -> UiNode {
        UiNode::Canvas(CanvasNode {
            width: self.width,
            height: self.height,
            frame_id: self.frame_id,
            bg: self.bg,
        })
    }
}

impl From<Canvas> for UiNode {
    fn from(c: Canvas) -> Self { c.build() }
}
