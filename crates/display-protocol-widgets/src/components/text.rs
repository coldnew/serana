use display_protocol::{Color, Style, TextNode, UiNode, Wrap};

#[derive(Debug, Clone)]
pub struct Text {
    content: String,
    style: Style,
    wrap: Wrap,
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: Style::default(),
            wrap: Wrap::Wrap,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.style = self.style.fg(color);
        self
    }

    pub fn bold(mut self) -> Self {
        self.style = self.style.bold();
        self
    }

    pub fn dim(mut self) -> Self {
        self.style = self.style.dim();
        self
    }

    pub fn no_wrap(mut self) -> Self {
        self.wrap = Wrap::NoWrap;
        self
    }

    pub fn truncate(mut self) -> Self {
        self.wrap = Wrap::Truncate;
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::Text(TextNode {
            content: self.content,
            style: self.style,
            wrap: self.wrap,
        })
    }
}

impl From<Text> for UiNode {
    fn from(text: Text) -> Self {
        text.build()
    }
}
