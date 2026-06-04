use crate::palette;
use display_protocol::{ListMarker, ListNode, Style, UiNode};

/// A list of items with optional markers (bullet, number, dash).
///
/// Works on both TUI and WGPU backends.
///
/// ```ignore
/// List::bullets(vec![
///     UiNode::text("First item"),
///     UiNode::text("Second item"),
///     UiNode::text("Third item"),
/// ]).build()
///
/// List::numbered(vec![
///     UiNode::text("Step one"),
///     UiNode::text("Step two"),
/// ]).build()
///
/// List::new(items).marker(ListMarker::Dash).build()
/// ```
#[derive(Debug, Clone)]
pub struct List {
    items: Vec<UiNode>,
    marker: ListMarker,
}

impl List {
    pub fn new(items: Vec<UiNode>) -> Self {
        Self {
            items,
            marker: ListMarker::None,
        }
    }

    /// Create a bulleted list.
    pub fn bullets(items: Vec<UiNode>) -> Self {
        Self {
            items,
            marker: ListMarker::Bullet,
        }
    }

    /// Create a numbered list.
    pub fn numbered(items: Vec<UiNode>) -> Self {
        Self {
            items,
            marker: ListMarker::Number,
        }
    }

    /// Create a dashed list.
    pub fn dashed(items: Vec<UiNode>) -> Self {
        Self {
            items,
            marker: ListMarker::Dash,
        }
    }

    pub fn marker(mut self, m: ListMarker) -> Self {
        self.marker = m;
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::List(ListNode {
            items: self.items,
            marker: self.marker,
            style: Style::default().fg(palette::LIGHT),
        })
    }
}

impl From<List> for UiNode {
    fn from(l: List) -> Self {
        l.build()
    }
}
