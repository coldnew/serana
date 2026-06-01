use crate::mora::editor::MoraEditor;
use display_protocol::{Color, Style, UiNode};

use super::pad_to_width;

/// Menu item definitions: (label, start_col, end_col) — end_col inclusive.
const MENU_ITEMS: &[(&str, u16, u16)] = &[
    ("File", 1, 4),
    ("Edit", 7, 10),
    ("Options", 13, 19),
    ("Buffers", 22, 28),
    ("Tools", 31, 35),
    ("Help", 38, 41),
];

/// Emacs-style menu bar with selection highlight.
pub(super) fn build_menu_bar(editor: &MoraEditor, width: u16) -> UiNode {
    let t = &editor.theme;
    let fg = t.modeline_fg;
    let bg = t.modeline_bg;
    let sel = editor.menu_bar_selection;

    let full_text = pad_to_width(
        " File  Edit  Options  Buffers  Tools  Help ",
        width as usize,
    );
    let total_chars = full_text.chars().count();

    let normal_style = Style {
        fg: Some(fg),
        bg: Some(bg),
        ..Style::default()
    };
    let highlight_style = Style {
        fg: Some(bg),
        bg: Some(fg),
        ..Style::default()
    };

    // Build spans: split text around the selected item.
    let mut spans = Vec::new();

    match sel.and_then(|idx| MENU_ITEMS.get(idx)) {
        Some(&(_label, start, end)) => {
            let start = start as usize;
            let end = (end as usize + 1).min(total_chars);

            // Before selected.
            if start > 0 {
                let before: String = full_text.chars().take(start).collect();
                spans.push(UiNode::span(before, normal_style));
            }
            // Selected item.
            let selected: String = full_text.chars().skip(start).take(end - start).collect();
            spans.push(UiNode::span(selected, highlight_style));
            // After selected.
            if end < total_chars {
                let after: String = full_text.chars().skip(end).collect();
                spans.push(UiNode::span(after, normal_style));
            }
        }
        None => {
            spans.push(UiNode::span(full_text.clone(), normal_style));
        }
    }

    UiNode::row(spans)
        .width(width)
        .height(1)
}
