use display_protocol::{Border, BoxNode, Padding, Style, UiNode};

#[derive(Debug, Clone)]
pub struct BoxComponent {
    children: Vec<UiNode>,
    style: Style,
    padding: Padding,
    border: Border,
    title: Option<String>,
    width: Option<u16>,
    height: Option<u16>,
}

impl BoxComponent {
    pub fn new(children: Vec<UiNode>) -> Self {
        Self {
            children,
            style: Style::default(),
            padding: Padding::ZERO,
            border: Border::NONE,
            title: None,
            width: None,
            height: None,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    pub fn border(mut self, border: Border) -> Self {
        self.border = border;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: u16) -> Self {
        self.height = Some(height);
        self
    }

    pub(crate) fn width_opt(mut self, width: Option<u16>) -> Self {
        self.width = width;
        self
    }

    pub(crate) fn height_opt(mut self, height: Option<u16>) -> Self {
        self.height = height;
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::Box(BoxNode {
            children: self.children,
            style: self.style,
            padding: self.padding,
            border: self.border,
            title: self.title,
            width: self.width,
            height: self.height,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
        })
    }
}

impl From<BoxComponent> for UiNode {
    fn from(component: BoxComponent) -> Self {
        component.build()
    }
}
