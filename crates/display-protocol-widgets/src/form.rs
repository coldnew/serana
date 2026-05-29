use display_protocol::{Align, FlexNode, Justify, Padding, Style, UiNode};
use crate::palette;

#[derive(Debug, Clone)]
pub struct FormField {
    label: String,
    input: UiNode,
    error: Option<String>,
    required: bool,
}

impl FormField {
    pub fn new(label: impl Into<String>, input: UiNode) -> Self {
        Self { label: label.into(), input, error: None, required: false }
    }

    pub fn error(mut self, msg: impl Into<String>) -> Self { self.error = Some(msg.into()); self }
    pub fn required(mut self, v: bool) -> Self { self.required = v; self }

    pub fn build(self) -> UiNode {
        let mut children = Vec::new();

        let mut label_parts = Vec::new();
        label_parts.push(UiNode::text(&self.label).color(palette::LIGHT));
        if self.required {
            label_parts.push(UiNode::text(" *").color(palette::DANGER));
        }

        children.push(UiNode::Row(FlexNode {
            children: label_parts,
            style: Style::default(),
            gap: 0,
            align: Align::Center,
            justify: Justify::Start,
            padding: Padding::ZERO,
            width: None,
            height: None,
            flex_grow: 0.0,
            flex_shrink: 1.0,
        }));

        children.push(self.input);

        if let Some(err) = &self.error {
            children.push(UiNode::text(err).color(palette::DANGER));
        }

        UiNode::Column(FlexNode {
            children,
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

#[derive(Debug, Clone)]
pub struct Form {
    fields: Vec<UiNode>,
    actions: Vec<UiNode>,
    gap: u16,
}

impl Form {
    pub fn new() -> Self {
        Self { fields: Vec::new(), actions: Vec::new(), gap: 1 }
    }

    pub fn field(mut self, node: UiNode) -> Self { self.fields.push(node); self }
    pub fn action(mut self, node: UiNode) -> Self { self.actions.push(node); self }
    pub fn gap(mut self, g: u16) -> Self { self.gap = g; self }

    pub fn build(self) -> UiNode {
        let mut children: Vec<UiNode> = self.fields;

        if !self.actions.is_empty() {
            children.push(UiNode::Row(FlexNode {
                children: self.actions,
                style: Style::default(),
                gap: 1,
                align: Align::Center,
                justify: Justify::End,
                padding: Padding::new(1, 0, 0, 0),
                width: None,
                height: None,
                flex_grow: 0.0,
                flex_shrink: 1.0,
            }));
        }

        UiNode::Column(FlexNode {
            children,
            style: Style::default(),
            gap: self.gap,
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

impl Default for Form {
    fn default() -> Self { Self::new() }
}
