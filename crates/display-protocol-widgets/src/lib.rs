//! display-protocol-widgets: reusable UI widgets for display-protocol
//!
//! All widgets work on both TUI and WGPU backends. They are stateless
//! visual descriptions — build them with current state, get a `UiNode` tree,
//! then paint.

use display_protocol::UiNode;

pub mod palette;

mod button;
pub use button::{Button, ButtonStyle};

mod checkbox;
pub use checkbox::Checkbox;

mod badge;
pub use badge::{Badge, BadgeStyle};

mod alert;
pub use alert::{Alert, AlertLevel};

mod card;
pub use card::Card;

mod dialog;
pub use dialog::Dialog;

mod form;
pub use form::{Form, FormField};

mod accordion;
pub use accordion::Accordion;

mod separator;
pub use separator::{Separator, SeparatorStyle};

mod progress;
pub use progress::ProgressBar;

mod scroll_pane;
pub use scroll_pane::ScrollPane;

mod tabs;
pub use tabs::Tabs;

mod table;
pub use table::Table;

mod toggle;
pub use toggle::Toggle;

mod text_input;
pub use text_input::TextInput;

mod editor;
pub use editor::Editor;

mod tree_view;
pub use tree_view::{TreeView, TreeBranch};

mod split_pane;
pub use split_pane::SplitPane;

mod status_bar;
pub use status_bar::StatusBar;

mod popup;
pub use popup::Popup;
/// Renders text as a keyboard shortcut key (e.g. `Ctrl+K`).
pub fn kbd(text: impl Into<String>) -> UiNode {
    use display_protocol::{Border, Padding, Style, BoxNode};
    let label = text.into();
    UiNode::Box(BoxNode {
        children: vec![UiNode::text(&label).color(palette::MUTED)],
        style: Style::default().bg(palette::Color::new(40, 40, 40)).fg(palette::MUTED),
        padding: Padding::new(0, 1, 0, 1),
        border: Border::all(Some(Style::default().fg(palette::Color::new(60, 60, 60)))),
        title: None,
        width: None,
        height: None,
        min_width: None,
        min_height: None,
        max_width: None,
        max_height: None,
    })
}

/// Renders clickable link text.
pub fn link(text: impl Into<String>) -> UiNode {
    UiNode::text(text).underline().color(palette::PRIMARY)
}

/// Renders a simple text-based loading spinner (frame-based animation).
pub fn spinner(frame: usize) -> UiNode {
    let chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let c = chars[frame % chars.len()];
    UiNode::text(c.to_string()).color(palette::PRIMARY)
}

/// Renders a breadcrumb navigation trail.
pub fn breadcrumb(items: Vec<UiNode>) -> UiNode {
    use display_protocol::{FlexNode, Align, Justify};
    let mut children = Vec::new();
    for (i, item) in items.into_iter().enumerate() {
        if i > 0 {
            children.push(UiNode::text(" / ").color(palette::MUTED));
        }
        children.push(item);
    }
    UiNode::Row(FlexNode {
        children,
        style: display_protocol::Style::default(),
        gap: 0,
        align: Align::Center,
        justify: Justify::Start,
        padding: display_protocol::Padding::ZERO,
        width: None,
        height: None,
        flex_grow: 0.0,
        flex_shrink: 1.0,
    })
}
