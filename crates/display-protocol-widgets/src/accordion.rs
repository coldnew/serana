use display_protocol::{Align, FlexNode, Justify, Padding, Style, UiNode, BoxNode};
use crate::palette;

#[derive(Debug, Clone)]
pub struct Accordion {
    title: String,
    expanded: bool,
    children: Vec<UiNode>,
}

impl Accordion {
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), expanded: false, children: Vec::new() }
    }

    pub fn expanded(mut self, v: bool) -> Self { self.expanded = v; self }
    pub fn child(mut self, node: UiNode) -> Self { self.children.push(node); self }
    pub fn children(mut self, nodes: Vec<UiNode>) -> Self { self.children = nodes; self }

    pub fn build(self) -> UiNode {
        let indicator = if self.expanded { "▼" } else { "▶" };

        let header = UiNode::Box(BoxNode {
            children: vec![
                UiNode::Row(FlexNode {
                    children: vec![
                        UiNode::text(indicator).color(palette::PRIMARY),
                        UiNode::text(&self.title).color(palette::LIGHT),
                    ],
                    style: Style::default(),
                    gap: 1,
                    align: Align::Center,
                    justify: Justify::Start,
                    padding: Padding::new(0, 1, 0, 1),
                    width: None,
                    height: None,
                    flex_grow: 0.0,
                    flex_shrink: 1.0,
                }),
            ],
            style: Style::default(),
            padding: Padding::ZERO,
            border: display_protocol::Border::NONE,
            title: None,
            width: None,
            height: None,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
        });

        let body = UiNode::Box(BoxNode {
            children: self.children,
            style: Style::default(),
            padding: Padding::new(0, 2, 0, 2),
            border: display_protocol::Border::NONE,
            title: None,
            width: None,
            height: None,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
        });

        UiNode::Column(FlexNode {
            children: vec![
                header,
                UiNode::show(self.expanded, body),
            ],
            style: Style::default(),
            gap: 0,
            align: Align::Stretch,
            justify: Justify::Start,
            padding: Padding::ZERO,
            width: None,
            height: None,
            flex_grow: 0.0,
            flex_shrink: 1.0,
        })
    }
}

impl From<Accordion> for UiNode {
    fn from(a: Accordion) -> Self { a.build() }
}
