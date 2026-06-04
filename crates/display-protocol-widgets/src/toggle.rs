use crate::palette;
use display_protocol::{FlexNode, Padding, Style, UiNode};

/// A toggle switch (on/off).
///
/// Visually distinct from Checkbox — renders as a sliding switch
/// rather than a checkmark box.
///
/// ```ignore
/// Toggle::new("Dark mode").on(true).build()
/// Toggle::new("Notifications").on(false).disabled(true).build()
/// ```
#[derive(Debug, Clone)]
pub struct Toggle {
    label: String,
    on: bool,
    disabled: bool,
}

impl Toggle {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            on: false,
            disabled: false,
        }
    }

    pub fn on(mut self, v: bool) -> Self {
        self.on = v;
        self
    }
    pub fn disabled(mut self, v: bool) -> Self {
        self.disabled = v;
        self
    }

    pub fn build(self) -> UiNode {
        let (track_bg, knob_fg, label_fg) = if self.disabled {
            (palette::DARK, palette::MUTED, palette::MUTED)
        } else if self.on {
            (palette::PRIMARY, palette::WHITE, palette::LIGHT)
        } else {
            (
                palette::Color::new(55, 55, 55),
                palette::MUTED,
                palette::LIGHT,
            )
        };

        // Toggle track: [●  ] or [  ●]
        let track = if self.on {
            UiNode::text("  ●").color(knob_fg).bg(track_bg)
        } else {
            UiNode::text("●  ").color(knob_fg).bg(track_bg)
        };

        let label = UiNode::text(&self.label).color(label_fg);

        let mut style = Style::default();
        if self.disabled {
            style = style.dim();
        }

        UiNode::Row(FlexNode {
            children: vec![track, label],
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

impl From<Toggle> for UiNode {
    fn from(t: Toggle) -> Self {
        t.build()
    }
}
