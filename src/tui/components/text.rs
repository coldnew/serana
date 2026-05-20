//! Text component.

use crate::tui::component::Component;
use crate::tui::style::Style;

pub struct Text {
    content: String,
    style: Option<Style>,
    wrap: bool,
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: None,
            wrap: true,
        }
    }

    pub fn styled(content: impl Into<String>, style: Style) -> Self {
        Self {
            content: content.into(),
            style: Some(style),
            wrap: true,
        }
    }

    /// Create centered text (no wrap, centered in terminal width)
    pub fn styled_centered(content: impl Into<String>, style: Style) -> Self {
        Self {
            content: content.into(),
            style: Some(style),
            wrap: false,
        }
    }

    pub fn no_wrap(mut self) -> Self {
        self.wrap = false;
        self
    }

    pub fn set_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
    }
}

impl Component for Text {
    fn render(&self, width: usize) -> Vec<String> {
        if self.content.is_empty() {
            return Vec::new();
        }
        let lines: Vec<String> = if self.wrap && width > 0 {
            textwrap::wrap(&self.content, width)
                .into_iter()
                .map(|cow| cow.into_owned())
                .collect()
        } else {
            vec![self.content.clone()]
        };
        match &self.style {
            Some(s) => lines.into_iter().map(|l| s.apply(&l)).collect(),
            None => lines,
        }
    }
}
