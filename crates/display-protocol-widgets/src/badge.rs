use crate::palette;
use display_protocol::{Border, BoxNode, Padding, Style, UiNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeStyle {
    Default,
    Primary,
    Success,
    Warning,
    Danger,
    Info,
}

#[derive(Debug, Clone)]
pub struct Badge {
    label: String,
    style: BadgeStyle,
    width: Option<u16>,
}

impl Badge {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            style: BadgeStyle::Default,
            width: None,
        }
    }

    pub fn style(mut self, s: BadgeStyle) -> Self {
        self.style = s;
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    pub fn build(self) -> UiNode {
        let (fg, bg) = match self.style {
            BadgeStyle::Default => (palette::MUTED, palette::Color::new(55, 55, 55)),
            BadgeStyle::Primary => (palette::WHITE, palette::PRIMARY),
            BadgeStyle::Success => (palette::WHITE, palette::SUCCESS),
            BadgeStyle::Warning => (palette::BLACK, palette::WARNING),
            BadgeStyle::Danger => (palette::WHITE, palette::DANGER),
            BadgeStyle::Info => (palette::WHITE, palette::INFO),
        };

        UiNode::Box(BoxNode {
            children: vec![UiNode::text(&self.label).color(fg).bg(bg)],
            style: Style::default().fg(fg).bg(bg),
            padding: Padding::new(0, 1, 0, 1),
            border: Border::NONE,
            title: None,
            width: self.width,
            height: None,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
        })
    }
}

impl From<Badge> for UiNode {
    fn from(b: Badge) -> Self {
        b.build()
    }
}
