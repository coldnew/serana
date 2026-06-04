use display_protocol::{OverlayNode, Style, UiNode};

/// An absolutely-positioned overlay (popup, tooltip, dropdown).
///
/// Positioned relative to the viewport. Stacked by `z_index`.
/// Works on both TUI (paints on top of cells) and WGPU (alpha compositing).
///
/// ```ignore
/// Popup::new(menu_content)
///     .at(40, 10)
///     .build()
///
/// Popup::new(tooltip_text)
///     .at(x, y)
///     .z_index(100)
///     .build()
/// ```
#[derive(Debug, Clone)]
pub struct Popup {
    child: UiNode,
    x: u16,
    y: u16,
    z_index: i32,
}

impl Popup {
    pub fn new(child: impl Into<UiNode>) -> Self {
        Self {
            child: child.into(),
            x: 0,
            y: 0,
            z_index: 0,
        }
    }

    pub fn at(mut self, x: u16, y: u16) -> Self {
        self.x = x;
        self.y = y;
        self
    }
    pub fn x(mut self, x: u16) -> Self {
        self.x = x;
        self
    }
    pub fn y(mut self, y: u16) -> Self {
        self.y = y;
        self
    }
    pub fn z_index(mut self, z: i32) -> Self {
        self.z_index = z;
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::Overlay(OverlayNode {
            child: Box::new(self.child),
            x: self.x,
            y: self.y,
            z_index: self.z_index,
            style: Style::default(),
        })
    }
}

impl From<Popup> for UiNode {
    fn from(p: Popup) -> Self {
        p.build()
    }
}
