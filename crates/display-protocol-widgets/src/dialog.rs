use display_protocol::{Align, Border, Justify, Padding, Style, UiNode, BoxNode, FlexNode, OverlayNode};
use crate::palette;

#[derive(Debug, Clone)]
pub struct Dialog {
    title: String,
    body: Vec<UiNode>,
    actions: Vec<UiNode>,
    width: u16,
    height: u16,
}

impl Dialog {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: Vec::new(),
            actions: Vec::new(),
            width: 60,
            height: 20,
        }
    }

    pub fn body(mut self, children: Vec<UiNode>) -> Self { self.body = children; self }
    pub fn action(mut self, node: UiNode) -> Self { self.actions.push(node); self }
    pub fn actions(mut self, nodes: Vec<UiNode>) -> Self { self.actions = nodes; self }
    pub fn width(mut self, w: u16) -> Self { self.width = w; self }
    pub fn height(mut self, h: u16) -> Self { self.height = h; self }

    pub fn build(self) -> UiNode {
        let content = UiNode::Box(BoxNode {
            children: vec![
                UiNode::Column(FlexNode {
                    children: vec![
                        UiNode::Row(FlexNode {
                            children: vec![
                                UiNode::text(&self.title).bold().color(palette::LIGHT),
                            ],
                            style: Style::default(),
                            gap: 0,
                            align: Align::Center,
                            justify: Justify::Start,
                            padding: Padding::new(0, 1, 0, 1),
                            width: None,
                            height: None,
                            flex_grow: 0.0,
                            flex_shrink: 1.0,
                        }),
                        if self.body.is_empty() {
                            UiNode::None
                        } else {
                            UiNode::Column(FlexNode {
                                children: self.body,
                                style: Style::default(),
                                gap: 1,
                                align: Align::Start,
                                justify: Justify::Start,
                                padding: Padding::new(1, 2, 1, 2),
                                width: None,
                                height: None,
                                flex_grow: 1.0,
                                flex_shrink: 1.0,
                            })
                        },
                        if self.actions.is_empty() {
                            UiNode::None
                        } else {
                            UiNode::Row(FlexNode {
                                children: vec![
                                    UiNode::Row(FlexNode {
                                        children: Vec::new(),
                                        style: Style::default(),
                                        gap: 0,
                                        align: Align::Center,
                                        justify: Justify::Start,
                                        padding: Padding::ZERO,
                                        width: None,
                                        height: None,
                                        flex_grow: 1.0,
                                        flex_shrink: 1.0,
                                    }),
                                    UiNode::Row(FlexNode {
                                        children: self.actions,
                                        style: Style::default(),
                                        gap: 1,
                                        align: Align::Center,
                                        justify: Justify::End,
                                        padding: Padding::new(0, 1, 0, 1),
                                        width: None,
                                        height: None,
                                        flex_grow: 0.0,
                                        flex_shrink: 0.0,
                                    }),
                                ],
                                style: Style::default(),
                                gap: 0,
                                align: Align::Center,
                                justify: Justify::Start,
                                padding: Padding::new(1, 0, 1, 0),
                                width: None,
                                height: None,
                                flex_grow: 0.0,
                                flex_shrink: 1.0,
                            })
                        },
                    ],
                    style: Style::default(),
                    gap: 0,
                    align: Align::Stretch,
                    justify: Justify::Start,
                    padding: Padding::ZERO,
                    width: Some(self.width),
                    height: Some(self.height),
                    flex_grow: 0.0,
                    flex_shrink: 1.0,
                }),
            ],
            style: Style::default(),
            padding: Padding::new(0, 0, 0, 0),
            border: Border::all(Some(Style::default().fg(palette::PRIMARY))),
            title: Some(self.title),
            width: None,
            height: None,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
        });

        UiNode::Overlay(OverlayNode {
            child: Box::new(content),
            x: 0,
            y: 0,
            z_index: 100,
            style: Style::default().bg(palette::Color::new(0, 0, 0)),
        })
    }
}

impl From<Dialog> for UiNode {
    fn from(d: Dialog) -> Self { d.build() }
}
