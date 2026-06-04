use display_protocol::{SpanNode, Style, UiNode};

/// An inline styled span of text.
///
/// Unlike `Text` (a block-level leaf), `Span` is designed for inline
/// composition within flowing text. Works on both TUI and WGPU.
///
/// ```ignore
/// Span::new("highlighted").fg(palette::WARNING).build()
/// Span::new("keyword").bold().fg(palette::PRIMARY).build()
/// Span::new("dim comment").dim().fg(palette::MUTED).build()
/// ```
#[derive(Debug, Clone)]
pub struct Span {
    content: String,
    fg: Option<display_protocol::Color>,
    bold: bool,
    italic: bool,
    underline: bool,
    dim: bool,
}

impl Span {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            fg: None,
            bold: false,
            italic: false,
            underline: false,
            dim: false,
        }
    }

    pub fn fg(mut self, c: display_protocol::Color) -> Self {
        self.fg = Some(c);
        self
    }
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }
    pub fn dim(mut self) -> Self {
        self.dim = true;
        self
    }

    pub fn build(self) -> UiNode {
        let mut style = Style::default();
        if let Some(fg) = self.fg {
            style = style.fg(fg);
        }
        if self.bold {
            style = style.bold();
        }
        if self.italic {
            style = style.italic();
        }
        if self.underline {
            style = style.underline();
        }
        if self.dim {
            style = style.dim();
        }

        UiNode::Span(SpanNode {
            content: self.content,
            style,
        })
    }
}

impl From<Span> for UiNode {
    fn from(s: Span) -> Self {
        s.build()
    }
}
