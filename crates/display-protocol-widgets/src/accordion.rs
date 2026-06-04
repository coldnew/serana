use crate::palette;
use display_protocol::{FlexNode, Padding, Style, UiNode, Wrap};

#[derive(Debug, Clone)]
pub struct Accordion {
    title: String,
    expanded: bool,
    children: Vec<UiNode>,
}

impl Accordion {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            expanded: false,
            children: Vec::new(),
        }
    }

    pub fn expanded(mut self, v: bool) -> Self {
        self.expanded = v;
        self
    }
    pub fn child(mut self, node: UiNode) -> Self {
        self.children.push(node);
        self
    }
    pub fn children(mut self, nodes: Vec<UiNode>) -> Self {
        self.children = nodes;
        self
    }

    pub fn build(self) -> UiNode {
        let indicator = if self.expanded { "▼" } else { "▶" };
        let mut children = vec![UiNode::text(format!("{indicator} {}", self.title))
            .color(palette::LIGHT)
            .wrap(Wrap::NoWrap)];
        if self.expanded {
            children.extend(self.children);
        }

        UiNode::Column(FlexNode {
            children,
            style: Style::default(),
            gap: 1,
            align: display_protocol::Align::Start,
            justify: display_protocol::Justify::Start,
            padding: Padding::ZERO,
            width: None,
            height: None,
            flex_grow: 0.0,
            flex_shrink: 1.0,
        })
    }
}

impl From<Accordion> for UiNode {
    fn from(a: Accordion) -> Self {
        a.build()
    }
}
