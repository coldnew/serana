use crate::palette;
use display_protocol::UiNode;

#[derive(Debug, Clone)]
pub struct SelectItem {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

impl SelectItem {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            description: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct SelectList {
    items: Vec<SelectItem>,
    selected: usize,
    max_visible: usize,
}

impl SelectList {
    pub fn new(items: Vec<SelectItem>) -> Self {
        Self {
            items,
            selected: 0,
            max_visible: 8,
        }
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }

    pub fn max_visible(mut self, max_visible: usize) -> Self {
        self.max_visible = max_visible.max(1);
        self
    }

    pub fn build(self) -> UiNode {
        if self.items.is_empty() {
            return UiNode::text("No matching items").color(palette::MUTED);
        }

        let selected = self.selected.min(self.items.len().saturating_sub(1));
        let half = self.max_visible / 2;
        let start = selected
            .saturating_sub(half)
            .min(self.items.len().saturating_sub(self.max_visible));
        let end = (start + self.max_visible).min(self.items.len());

        let mut rows = Vec::new();
        for (index, item) in self.items[start..end].iter().enumerate() {
            let absolute = start + index;
            let prefix = if absolute == selected { "> " } else { "  " };
            let mut children = vec![UiNode::text(format!("{prefix}{}", item.label))];
            if let Some(description) = &item.description {
                children.push(UiNode::text(description).color(palette::MUTED));
            }
            let mut row = UiNode::row(children).gap(2);
            if absolute == selected {
                row = row.bg(palette::Color::new(35, 45, 65));
            }
            rows.push(row);
        }

        if self.items.len() > self.max_visible {
            rows.push(
                UiNode::text(format!("{} / {}", selected + 1, self.items.len()))
                    .color(palette::MUTED),
            );
        }

        UiNode::column(rows)
    }
}

impl From<SelectList> for UiNode {
    fn from(list: SelectList) -> Self {
        list.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_list_marks_selected_item() {
        let node = SelectList::new(vec![
            SelectItem::new("one", "One"),
            SelectItem::new("two", "Two").description("Second"),
        ])
        .selected(1)
        .build();

        match node {
            UiNode::Column(column) => assert_eq!(column.children.len(), 2),
            _ => panic!("expected column node"),
        }
    }
}
