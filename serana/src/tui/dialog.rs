//! Interactive popup dialogs for model, theme, and session selection.

/// Which dialog is currently open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogKind {
    ModelSelector,
    ThemeSelector,
    SessionSelector,
}

/// Active dialog state.
#[derive(Debug)]
pub struct Dialog {
    pub kind: DialogKind,
    pub title: String,
    pub items: Vec<DialogItem>,
    pub selected: Option<usize>,
    pub filter: String,
}

/// A selectable item in a dialog.
#[derive(Debug, Clone)]
pub struct DialogItem {
    pub label: String,
    pub description: String,
    pub value: String,
}

impl Dialog {
    pub fn new_model_selector(models: Vec<(String, String)>) -> Self {
        Self {
            kind: DialogKind::ModelSelector,
            title: " Select Model ".to_string(),
            items: models
                .into_iter()
                .map(|(name, desc)| DialogItem {
                    label: name.clone(),
                    description: desc,
                    value: name,
                })
                .collect(),
            selected: Some(0),
            filter: String::new(),
        }
    }

    pub fn new_theme_selector(themes: Vec<(String, String)>) -> Self {
        Self {
            kind: DialogKind::ThemeSelector,
            title: " Select Theme ".to_string(),
            items: themes
                .into_iter()
                .map(|(name, desc)| DialogItem {
                    label: name.clone(),
                    description: desc,
                    value: name,
                })
                .collect(),
            selected: Some(0),
            filter: String::new(),
        }
    }

    pub fn new_session_selector(sessions: Vec<(String, String)>) -> Self {
        Self {
            kind: DialogKind::SessionSelector,
            title: " Select Session ".to_string(),
            items: sessions
                .into_iter()
                .map(|(id, desc)| DialogItem {
                    label: desc.clone(),
                    description: String::new(),
                    value: id,
                })
                .collect(),
            selected: Some(0),
            filter: String::new(),
        }
    }

    /// Filtered items based on current filter text.
    pub fn filtered_items(&self) -> Vec<(usize, &DialogItem)> {
        if self.filter.is_empty() {
            self.items.iter().enumerate().collect()
        } else {
            let filter_lower = self.filter.to_lowercase();
            self.items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    item.label.to_lowercase().contains(&filter_lower)
                        || item.description.to_lowercase().contains(&filter_lower)
                })
                .collect()
        }
    }

    /// Move selection up.
    pub fn select_previous(&mut self) {
        let filtered = self.filtered_items();
        if filtered.is_empty() {
            self.selected = None;
            return;
        }
        let current = self.selected.unwrap_or(0);
        let filtered_indices: Vec<usize> = filtered.iter().map(|(i, _)| *i).collect();
        let pos = filtered_indices.iter().position(|&i| i == current);
        let new_pos = match pos {
            Some(0) => filtered_indices.len() - 1,
            Some(p) => p - 1,
            None => 0,
        };
        self.selected = Some(filtered_indices[new_pos]);
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        let filtered = self.filtered_items();
        if filtered.is_empty() {
            self.selected = None;
            return;
        }
        let current = self.selected.unwrap_or(0);
        let filtered_indices: Vec<usize> = filtered.iter().map(|(i, _)| *i).collect();
        let pos = filtered_indices.iter().position(|&i| i == current);
        let new_pos = match pos {
            Some(p) if p + 1 >= filtered_indices.len() => 0,
            Some(p) => p + 1,
            None => 0,
        };
        self.selected = Some(filtered_indices[new_pos]);
    }

    /// Type into the filter.
    pub fn filter_input(&mut self, ch: char) {
        self.filter.push(ch);
        // Re-validate selection is still in filtered set
        let filtered_indices: Vec<usize> = self.filtered_items().iter().map(|(i, _)| *i).collect();
        if let Some(current) = self.selected {
            if !filtered_indices.contains(&current) {
                self.selected = filtered_indices.first().copied();
            }
        }
    }

    /// Backspace on filter.
    pub fn filter_backspace(&mut self) {
        self.filter.pop();
        // Re-validate selection
        let filtered_indices: Vec<usize> = self.filtered_items().iter().map(|(i, _)| *i).collect();
        if let Some(current) = self.selected {
            if !filtered_indices.contains(&current) {
                self.selected = filtered_indices.first().copied();
            }
        }
    }

    /// Get the selected item's value.
    pub fn selected_value(&self) -> Option<&str> {
        self.selected
            .and_then(|i| self.items.get(i))
            .map(|item| item.value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_model_selector() {
        let dialog = Dialog::new_model_selector(vec![
            ("gpt-4o".into(), "OpenAI flagship".into()),
            ("claude-3.5".into(), "Anthropic".into()),
        ]);
        assert_eq!(dialog.items.len(), 2);
        assert_eq!(dialog.selected, Some(0));
    }

    #[test]
    fn test_dialog_filter() {
        let mut dialog = Dialog::new_model_selector(vec![
            ("gpt-4o".into(), "OpenAI".into()),
            ("claude-3.5".into(), "Anthropic".into()),
        ]);
        dialog.filter_input('c');
        assert_eq!(dialog.filtered_items().len(), 1);
        assert_eq!(dialog.filtered_items()[0].1.value, "claude-3.5");
    }

    #[test]
    fn test_dialog_navigation() {
        let mut dialog = Dialog::new_model_selector(vec![
            ("gpt-4o".into(), "OpenAI".into()),
            ("claude-3.5".into(), "Anthropic".into()),
            ("llama-3".into(), "Meta".into()),
        ]);
        dialog.select_next();
        assert_eq!(dialog.selected, Some(1));
        dialog.select_next();
        assert_eq!(dialog.selected, Some(2));
        dialog.select_next(); // wraps
        assert_eq!(dialog.selected, Some(0));
        dialog.select_previous();
        assert_eq!(dialog.selected, Some(2));
    }

    #[test]
    fn test_dialog_selected_value() {
        let dialog = Dialog::new_model_selector(vec![
            ("gpt-4o".into(), "OpenAI".into()),
            ("claude-3.5".into(), "Anthropic".into()),
        ]);
        assert_eq!(dialog.selected_value(), Some("gpt-4o"));
    }

    #[test]
    fn test_dialog_filter_backspace() {
        let mut dialog = Dialog::new_model_selector(vec![
            ("gpt-4o".into(), "OpenAI".into()),
            ("claude-3.5".into(), "Anthropic".into()),
        ]);
        dialog.filter_input('g');
        assert_eq!(dialog.filtered_items().len(), 1);
        dialog.filter_backspace();
        assert_eq!(dialog.filtered_items().len(), 2);
    }
}
