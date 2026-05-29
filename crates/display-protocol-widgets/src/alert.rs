use display_protocol::{Border, Padding, Style, UiNode, BoxNode, FlexNode};
use crate::palette;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Alert {
    level: AlertLevel,
    title: Option<String>,
    message: String,
}

impl Alert {
    pub fn new(level: AlertLevel, message: impl Into<String>) -> Self {
        Self { level, title: None, message: message.into() }
    }

    pub fn title(mut self, t: impl Into<String>) -> Self { self.title = Some(t.into()); self }

    pub fn build(self) -> UiNode {
        let (icon_str, fg, bg, border_fg) = match self.level {
            AlertLevel::Info => ("ℹ", palette::INFO, palette::Color::new(20, 40, 60), palette::INFO),
            AlertLevel::Success => ("✓", palette::SUCCESS, palette::Color::new(20, 50, 30), palette::SUCCESS),
            AlertLevel::Warning => ("⚠", palette::WARNING, palette::Color::new(50, 45, 20), palette::WARNING),
            AlertLevel::Error => ("✖", palette::DANGER, palette::Color::new(50, 20, 20), palette::DANGER),
        };

        let mut children = Vec::new();

        let icon = UiNode::text(icon_str).color(fg).bold();

        if let Some(title) = &self.title {
            children.push(UiNode::Row(FlexNode {
                children: vec![
                    icon,
                    UiNode::text(title).color(fg).bold(),
                ],
                style: Style::default(),
                gap: 1,
                align: display_protocol::Align::Center,
                justify: display_protocol::Justify::Start,
                padding: Padding::ZERO,
                width: None,
                height: None,
                flex_grow: 0.0,
                flex_shrink: 1.0,
            }));
        } else {
            children.push(icon);
        }

        children.push(UiNode::text(&self.message).color(fg));

        UiNode::Box(BoxNode {
            children,
            style: Style::default().fg(fg).bg(bg),
            padding: Padding::new(0, 2, 0, 2),
            border: Border::all(Some(Style::default().fg(border_fg))),
            title: None,
            width: None,
            height: None,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
        })
    }
}

impl From<Alert> for UiNode {
    fn from(a: Alert) -> Self { a.build() }
}
