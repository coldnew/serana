use display_protocol::{Orientation, SplitPaneNode, Style, UiNode};
use crate::palette;

/// A split pane with two children separated by a divider.
///
/// Works on both TUI (`│` or `─` divider) and WGPU (draggable handle).
///
/// ```ignore
/// SplitPane::horizontal(left, right)
///     .ratio(0.3)
///     .build()
///
/// SplitPane::vertical(top, bottom)
///     .ratio(0.7)
///     .build()
/// ```
#[derive(Debug, Clone)]
pub struct SplitPane {
    orientation: Orientation,
    ratio: f32,
    first: UiNode,
    second: UiNode,
    divider_color: Option<display_protocol::Color>,
}

impl SplitPane {
    /// Create a horizontal split (left | right).
    pub fn horizontal(first: impl Into<UiNode>, second: impl Into<UiNode>) -> Self {
        Self {
            orientation: Orientation::Horizontal,
            ratio: 0.5,
            first: first.into(),
            second: second.into(),
            divider_color: None,
        }
    }

    /// Create a vertical split (top / bottom).
    pub fn vertical(first: impl Into<UiNode>, second: impl Into<UiNode>) -> Self {
        Self {
            orientation: Orientation::Vertical,
            ratio: 0.5,
            first: first.into(),
            second: second.into(),
            divider_color: None,
        }
    }

    pub fn ratio(mut self, r: f32) -> Self { self.ratio = r.clamp(0.0, 1.0); self }
    pub fn divider_color(mut self, c: display_protocol::Color) -> Self { self.divider_color = Some(c); self }

    pub fn build(self) -> UiNode {
        let mut divider_style = Style::default();
        if let Some(c) = self.divider_color {
            divider_style = divider_style.fg(c);
        } else {
            divider_style = divider_style.fg(palette::MUTED);
        }

        UiNode::SplitPane(SplitPaneNode {
            orientation: self.orientation,
            ratio: self.ratio,
            first: Box::new(self.first),
            second: Box::new(self.second),
            divider_style,
        })
    }
}

impl From<SplitPane> for UiNode {
    fn from(sp: SplitPane) -> Self { sp.build() }
}
