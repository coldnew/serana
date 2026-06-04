use crate::palette;
use display_protocol::{Align, Justify, UiNode};

#[derive(Debug, Clone)]
pub struct SettingItem {
    pub label: String,
    pub value: String,
    pub description: Option<String>,
}

impl SettingItem {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            description: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct SettingsList {
    items: Vec<SettingItem>,
    selected: Option<usize>,
}

impl SettingsList {
    pub fn new(items: Vec<SettingItem>) -> Self {
        Self {
            items,
            selected: None,
        }
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = Some(selected);
        self
    }

    pub fn build(self) -> UiNode {
        let rows = self
            .items
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let selected = self.selected == Some(index);
                let mut left = vec![UiNode::text(item.label).bold()];
                if let Some(description) = item.description {
                    left.push(UiNode::text(description).color(palette::MUTED));
                }
                let mut row = UiNode::row(vec![
                    UiNode::column(left),
                    UiNode::text(item.value).color(palette::PRIMARY),
                ])
                .justify(Justify::SpaceBetween)
                .align(Align::Center);
                if selected {
                    row = row.bg(palette::Color::new(35, 45, 65));
                }
                row
            })
            .collect();

        UiNode::column(rows).gap(1)
    }
}

impl From<SettingsList> for UiNode {
    fn from(list: SettingsList) -> Self {
        list.build()
    }
}
