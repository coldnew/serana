use crate::palette;
use display_protocol::{StatusBarNode, Style, UiNode};

/// A status bar with left and right sections.
///
/// Works on both TUI (one-line with left/right alignment) and WGPU
/// (same, with richer styling).
///
/// ```ignore
/// StatusBar::new()
///     .left(vec![
///         UiNode::text("NORMAL").bold(),
///         UiNode::text(" │ "),
///         UiNode::text("main.rs"),
///     ])
///     .right(vec![
///         UiNode::text("utf-8"),
///         UiNode::text(" │ "),
///         UiNode::text("Rust"),
///     ])
///     .build()
/// ```
#[derive(Debug, Clone)]
pub struct StatusBar {
    left: Vec<UiNode>,
    right: Vec<UiNode>,
    bg: Option<display_protocol::Color>,
    fg: Option<display_protocol::Color>,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            left: Vec::new(),
            right: Vec::new(),
            bg: None,
            fg: None,
        }
    }

    pub fn left(mut self, items: Vec<UiNode>) -> Self {
        self.left = items;
        self
    }
    pub fn right(mut self, items: Vec<UiNode>) -> Self {
        self.right = items;
        self
    }
    pub fn left_text(mut self, text: impl Into<String>) -> Self {
        self.left.push(UiNode::text(text));
        self
    }
    pub fn right_text(mut self, text: impl Into<String>) -> Self {
        self.right.push(UiNode::text(text));
        self
    }
    pub fn bg(mut self, c: display_protocol::Color) -> Self {
        self.bg = Some(c);
        self
    }
    pub fn fg(mut self, c: display_protocol::Color) -> Self {
        self.fg = Some(c);
        self
    }

    pub fn build(self) -> UiNode {
        let mut style = Style::default();
        if let Some(fg) = self.fg {
            style = style.fg(fg);
        } else {
            style = style.fg(palette::LIGHT);
        }
        if let Some(bg) = self.bg {
            style = style.bg(bg);
        } else {
            style = style.bg(palette::Color::new(35, 35, 45));
        }

        UiNode::StatusBar(StatusBarNode {
            left: self.left,
            right: self.right,
            style,
        })
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl From<StatusBar> for UiNode {
    fn from(sb: StatusBar) -> Self {
        sb.build()
    }
}
