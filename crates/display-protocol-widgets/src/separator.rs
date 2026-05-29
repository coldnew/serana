use display_protocol::{DividerNode, Style, UiNode};
use crate::palette;


/// Visual style for a separator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeparatorStyle {
    /// ── solid line
    Solid,
    /// ─ ─ dashed line
    Dashed,
    /// ··· dotted line
    Dotted,
    /// Blank vertical gap
    Spacer,
}

/// A horizontal divider / separator.
///
/// Works on both TUI (character-based line) and WGPU (rendered line).
///
/// ```ignore
/// Separator::new().build()              // ──── (default solid)
/// Separator::new().dashed().build()     // ─ ─ ─
/// Separator::new().dotted().build()     // · · ·
/// Separator::new().spacer(2).build()    // blank 2-line gap
/// ```
#[derive(Debug, Clone)]
pub struct Separator {
    style: SeparatorStyle,
    color: Option<display_protocol::Color>,
}

impl Separator {
    pub fn new() -> Self {
        Self {
            style: SeparatorStyle::Solid,
            color: None,
        }
    }

    pub fn solid(mut self) -> Self { self.style = SeparatorStyle::Solid; self }
    pub fn dashed(mut self) -> Self { self.style = SeparatorStyle::Dashed; self }
    pub fn dotted(mut self) -> Self { self.style = SeparatorStyle::Dotted; self }

    /// Create a blank spacer.
    pub fn spacer() -> Self {
        Self { style: SeparatorStyle::Spacer, color: None }
    }

    pub fn color(mut self, c: display_protocol::Color) -> Self { self.color = Some(c); self }

    pub fn build(self) -> UiNode {
        if self.style == SeparatorStyle::Spacer {
            return UiNode::Divider(DividerNode {
                style: Style::default(),
                char: Some(' '),
            });
        }

        let ch = match self.style {
            SeparatorStyle::Solid => '─',
            SeparatorStyle::Dashed => '┄',
            SeparatorStyle::Dotted => '·',
            SeparatorStyle::Spacer => ' ',
        };

        let mut style = Style::default();
        if let Some(c) = self.color {
            style = style.fg(c);
        } else {
            style = style.fg(palette::MUTED);
        }

        UiNode::Divider(DividerNode {
            style,
            char: Some(ch),
        })
    }
}

impl Default for Separator {
    fn default() -> Self { Self::new() }
}

impl From<Separator> for UiNode {
    fn from(s: Separator) -> Self { s.build() }
}
