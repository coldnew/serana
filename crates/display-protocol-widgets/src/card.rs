use display_protocol::{Border, Padding, Style, UiNode, BoxNode};

#[derive(Debug, Clone)]
pub struct Card {
    title: Option<String>,
    children: Vec<UiNode>,
    padding: Padding,
    bordered: bool,
    width: Option<u16>,
    height: Option<u16>,
}

impl Card {
    pub fn new(children: Vec<UiNode>) -> Self {
        Self {
            title: None,
            children,
            padding: Padding::new(0, 2, 0, 2),
            bordered: true,
            width: None,
            height: None,
        }
    }

    pub fn title(mut self, t: impl Into<String>) -> Self { self.title = Some(t.into()); self }
    pub fn padding(mut self, p: Padding) -> Self { self.padding = p; self }
    pub fn bordered(mut self, v: bool) -> Self { self.bordered = v; self }
    pub fn width(mut self, w: u16) -> Self { self.width = Some(w); self }
    pub fn height(mut self, h: u16) -> Self { self.height = Some(h); self }

    pub fn build(self) -> UiNode {
        UiNode::Box(BoxNode {
            children: self.children,
            style: Style::default(),
            padding: self.padding,
            border: if self.bordered { Border::all(None) } else { Border::NONE },
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

impl From<Card> for UiNode {
    fn from(c: Card) -> Self { c.build() }
}
