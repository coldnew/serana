use display_protocol::{FlexNode, Padding, Style, UiNode};
use crate::palette;

#[derive(Debug, Clone)]
pub struct Checkbox {
    label: String,
    checked: bool,
    disabled: bool,
}

impl Checkbox {
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), checked: false, disabled: false }
    }

    pub fn checked(mut self, v: bool) -> Self { self.checked = v; self }
    pub fn disabled(mut self, v: bool) -> Self { self.disabled = v; self }

    pub fn build(self) -> UiNode {
        let (fg, check_fg, check_bg) = if self.disabled {
            (palette::MUTED, palette::MUTED, palette::DARK)
        } else if self.checked {
            (palette::LIGHT, palette::WHITE, palette::PRIMARY)
        } else {
            (palette::LIGHT, palette::MUTED, palette::DARK)
        };

        let check_mark = if self.checked { "✓" } else { " " };

        let checkbox = UiNode::text(check_mark)
            .color(check_fg)
            .bg(check_bg);

        let label = UiNode::text(&self.label)
            .color(fg);

        let mut style = Style::default();
        if self.disabled { style = style.dim(); }

        UiNode::Row(FlexNode {
            children: vec![checkbox, label],
            style,
            gap: 1,
            align: display_protocol::Align::Center,
            justify: display_protocol::Justify::Start,
            padding: Padding::ZERO,
            width: None,
            height: None,
            flex_grow: 0.0,
            flex_shrink: 1.0,
        })
    }
}

impl From<Checkbox> for UiNode {
    fn from(cb: Checkbox) -> Self { cb.build() }
}
