use crate::palette;
use display_protocol::{UiNode, Wrap};

const LABEL_COLUMN_WIDTH: usize = 30;

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
        let mut rows = Vec::new();
        for (index, item) in self.items.into_iter().enumerate() {
            let selected = self.selected == Some(index);
            let prefix = if selected { "→ " } else { "  " };
            let mut row = UiNode::text(format!(
                "{prefix}{:<LABEL_COLUMN_WIDTH$}  {}",
                item.label, item.value
            ))
            .wrap(Wrap::NoWrap);
            if selected {
                row = row.bg(palette::Color::new(35, 45, 65));
            }
            rows.push(row);

            if selected {
                if let Some(description) = item.description {
                    rows.push(
                        UiNode::text(format!("  {description}"))
                            .color(palette::MUTED)
                            .wrap(Wrap::NoWrap),
                    );
                }
            }
        }

        UiNode::column(rows)
    }
}

impl From<SettingsList> for UiNode {
    fn from(list: SettingsList) -> Self {
        list.build()
    }
}
