//! Box component for bordered containers.

use crate::component::{Component, Container};
use crate::style::{Colors, Style};

/// Box with rounded border (oh-my-pi style).
pub struct BoxWidget {
    container: Container,
    title: Option<String>,
    border_style: Style,
    padding: Padding,
}

#[derive(Clone, Copy)]
pub struct Padding {
    pub left: usize,
    pub right: usize,
    pub top: usize,
    pub bottom: usize,
}

impl Default for Padding {
    fn default() -> Self {
        Self {
            left: 1,
            right: 1,
            top: 0,
            bottom: 0,
        }
    }
}

impl BoxWidget {
    pub fn new() -> Self {
        Self {
            container: Container::new(),
            title: None,
            border_style: Style::default().fg(Colors::GRAY),
            padding: Padding::default(),
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    pub fn with_padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    pub fn add_child(&mut self, component: Box<dyn Component>) {
        self.container.add_child(component);
    }

    fn top_border(&self, inner_width: usize) -> String {
        let mut line = String::from("╭");
        if let Some(title) = &self.title {
            line.push(' ');
            line.push_str(title);
            line.push(' ');
            let used = title.chars().count() + 2;
            line.push_str(&"─".repeat(inner_width.saturating_sub(used)));
        } else {
            line.push_str(&"─".repeat(inner_width));
        }
        line.push('╮');
        self.border_style.apply(&line)
    }

    fn bottom_border(&self, inner_width: usize) -> String {
        self.border_style
            .apply(&format!("╰{}╯", "─".repeat(inner_width)))
    }
}

impl Component for BoxWidget {
    fn render(&self, width: usize) -> Vec<String> {
        let inner_width = width.saturating_sub(2);
        let content_width = inner_width.saturating_sub(self.padding.left + self.padding.right);
        let mut lines = Vec::new();
        lines.push(self.top_border(inner_width));

        for _ in 0..self.padding.top {
            lines.push(format!("│{}│", " ".repeat(inner_width)));
        }

        for content in self.container.render(content_width.max(1)) {
            let visible = strip_ansi_escapes::strip_str(&content).chars().count();
            let pad_right = inner_width.saturating_sub(self.padding.left + visible);
            lines.push(format!(
                "{}{}{}{}{}",
                self.border_style.apply("│"),
                " ".repeat(self.padding.left),
                content,
                " ".repeat(pad_right),
                self.border_style.apply("│")
            ));
        }

        for _ in 0..self.padding.bottom {
            lines.push(format!("│{}│", " ".repeat(inner_width)));
        }

        lines.push(self.bottom_border(inner_width));
        lines
    }

    fn invalidate(&mut self) {
        self.container.invalidate();
    }
}

impl Default for BoxWidget {
    fn default() -> Self {
        Self::new()
    }
}
