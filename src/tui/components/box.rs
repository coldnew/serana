//! Box component for bordered containers

use crate::tui::component::{Component, Container};
use crate::tui::style::{Style, Colors};

/// Box with border around content
pub struct Box {
    container: Container,
    title: Option<String>,
    border_style: Style,
    padding: Padding,
    width: usize,
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

impl Box {
    pub fn new() -> Self {
        Self {
            container: Container::new(),
            title: None,
            border_style: Style::default().fg(Colors::Gray),
            padding: Padding::default(),
            width: 0,
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

    fn render_top_border(&self, width: usize, has_title: bool) -> String {
        let title_str = if let Some(ref title) = self.title {
            format!(" {} ", title)
        } else {
            String::new()
        };

        let border_char = "─";
        let left_len = if has_title { title_str.len() } else { 0 };
        let remaining = width.saturating_sub(left_len + 2);

        format!(
            "┌{}{}{}┐",
            if has_title { &title_str } else { String::new() },
            border_char.repeat(remaining)
        )
    }

    fn render_bottom_border(&self, width: usize) -> String {
        format!("└{}┘", "─".repeat(width))
    }

    fn render_content(&self, width: usize) -> Vec<String> {
        let content_width = width.saturating_sub(self.padding.left + self.padding.right);
        let content_lines = self.container.render(content_width);

        let mut padded = Vec::new();
        for _ in 0..self.padding.top {
            padded.push(String::new());
        }
        for line in content_lines {
            let trimmed = if line.len() > content_width {
                line[..content_width].to_string()
            } else {
                line
            };
            let padded_line = format!(
                "{}{}{}",
                " ".repeat(self.padding.left),
                trimmed,
                " ".repeat(width.saturating_sub(trimmed.len() + self.padding.left + self.padding.right))
            );
            padded.push(padded_line);
        }
        for _ in 0..self.padding.bottom {
            padded.push(String::new());
        }
        padded
    }
}

impl Component for Box {
    fn render(&self, width: usize) -> Vec<String> {
        let has_title = self.title.is_some();
        let top = self.border_style.apply(&self.render_top_border(width, has_title));

        let content = self.render_content(width);
        let mut lines = vec![top];
        for line in content {
            let bordered = format!("│{}│", line);
            lines.push(bordered);
        }
        lines.push(self.border_style.apply(&self.render_bottom_border(width)));
        lines
    }

    fn invalidate(&mut self) {
        self.container.invalidate();
    }
}

impl Default for Box {
    fn default() -> Self {
        Self::new()
    }
}
