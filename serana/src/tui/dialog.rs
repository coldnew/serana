//! Interactive popup dialogs for model, theme, and session selection.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, ListState, Paragraph};

use super::theme::{self, Theme};

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
    pub state: ListState,
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
        let mut state = ListState::default();
        state.select(Some(0));
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
            state,
            filter: String::new(),
        }
    }

    pub fn new_theme_selector(themes: Vec<(String, String)>) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
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
            state,
            filter: String::new(),
        }
    }

    pub fn new_session_selector(sessions: Vec<(String, String)>) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
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
            state,
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
            self.state.select(None);
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let filtered_indices: Vec<usize> = filtered.iter().map(|(i, _)| *i).collect();
        let pos = filtered_indices.iter().position(|&i| i == current);
        let new_pos = match pos {
            Some(0) => filtered_indices.len() - 1,
            Some(p) => p - 1,
            None => 0,
        };
        self.state.select(Some(filtered_indices[new_pos]));
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        let filtered = self.filtered_items();
        if filtered.is_empty() {
            self.state.select(None);
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let filtered_indices: Vec<usize> = filtered.iter().map(|(i, _)| *i).collect();
        let pos = filtered_indices.iter().position(|&i| i == current);
        let new_pos = match pos {
            Some(p) if p + 1 >= filtered_indices.len() => 0,
            Some(p) => p + 1,
            None => 0,
        };
        self.state.select(Some(filtered_indices[new_pos]));
    }

    /// Type into the filter.
    pub fn filter_input(&mut self, ch: char) {
        self.filter.push(ch);
        // Re-validate selection is still in filtered set
        let filtered_indices: Vec<usize> = self.filtered_items().iter().map(|(i, _)| *i).collect();
        if let Some(current) = self.state.selected() {
            if !filtered_indices.contains(&current) {
                self.state.select(filtered_indices.first().copied());
            }
        }
    }

    /// Backspace on filter.
    pub fn filter_backspace(&mut self) {
        self.filter.pop();
        // Re-validate selection
        let filtered_indices: Vec<usize> = self.filtered_items().iter().map(|(i, _)| *i).collect();
        if let Some(current) = self.state.selected() {
            if !filtered_indices.contains(&current) {
                self.state.select(filtered_indices.first().copied());
            }
        }
    }

    /// Get the selected item's value.
    pub fn selected_value(&self) -> Option<&str> {
        self.state
            .selected()
            .and_then(|i| self.items.get(i))
            .map(|item| item.value.as_str())
    }
}

/// Render a centered popup dialog over the existing frame.
pub fn render_dialog(frame: &mut ratatui::Frame, dialog: &Dialog) {
    let theme = Theme::default();
    let area = frame.area();

    // Calculate popup size (centered, max 60x20 or 2/3 of screen)
    let popup_width = area.width.min(60).min(area.width * 2 / 3);
    let popup_height = area.height.min(20).min(area.height * 2 / 3);
    let popup_area = centered_rect(popup_width, popup_height, area);

    // Clear background
    frame.render_widget(Clear, popup_area);

    // Split: title + filter + list + help
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // filter
            Constraint::Min(3),    // list
            Constraint::Length(1), // help
        ])
        .split(popup_area.inner(ratatui::layout::Margin {
            horizontal: 1,
            vertical: 1,
        }));

    // Border
    let block = Block::default()
        .title(Span::styled(
            &dialog.title,
            Style::new()
                .fg(theme::AQUAMARINE)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(theme::AQUAMARINE));
    frame.render_widget(block, popup_area);

    // Filter line
    let filter_text = if dialog.filter.is_empty() {
        Line::from(Span::styled("Type to filter...", theme.dim))
    } else {
        Line::from(vec![
            Span::styled(&dialog.filter, Style::new().fg(theme::AQUAMARINE)),
            Span::styled(
                "▏",
                Style::new()
                    .fg(theme::AQUAMARINE)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    };
    frame.render_widget(Paragraph::new(filter_text), inner[0]);

    let filtered = dialog.filtered_items();
    let selected_original = dialog.state.selected();
    let selected_filtered = selected_original
        .and_then(|selected| filtered.iter().position(|(idx, _)| *idx == selected))
        .unwrap_or(0);
    let list_height = inner[1].height as usize;
    let show_count = list_height > 1 && filtered.len() > list_height;
    let visible_height = if show_count {
        list_height.saturating_sub(1)
    } else {
        list_height
    };
    let (start, end) = visible_range(selected_filtered, filtered.len(), visible_height);

    let mut list_lines = Vec::new();
    if filtered.is_empty() {
        list_lines.push(Line::from(Span::styled("  No matching items", theme.dim)));
    } else {
        let primary_width = primary_column_width(&filtered, inner[1].width as usize);
        for (pos, (_, item)) in filtered.iter().enumerate().take(end).skip(start) {
            list_lines.push(render_dialog_item(
                item,
                pos == selected_filtered,
                inner[1].width as usize,
                primary_width,
            ));
        }
    }

    let list_area = Rect {
        x: inner[1].x,
        y: inner[1].y,
        width: inner[1].width,
        height: visible_height.min(list_height) as u16,
    };
    frame.render_widget(Paragraph::new(list_lines), list_area);

    if show_count && !filtered.is_empty() {
        let count_area = Rect {
            x: inner[1].x,
            y: inner[1].y + visible_height as u16,
            width: inner[1].width,
            height: 1,
        };
        let count = Line::from(Span::styled(
            format!("  ({}/{})", selected_filtered + 1, filtered.len()),
            theme.dim,
        ));
        frame.render_widget(Paragraph::new(count), count_area);
    }

    // Help line
    let help = dialog_help_line(inner[2].width as usize, &theme);
    frame.render_widget(Paragraph::new(help), inner[2]);
}

/// Create a centered rectangle.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

fn visible_range(selected: usize, total: usize, max_visible: usize) -> (usize, usize) {
    if total == 0 || max_visible == 0 {
        return (0, 0);
    }
    let max_visible = max_visible.min(total);
    let start = selected
        .saturating_sub(max_visible / 2)
        .min(total.saturating_sub(max_visible));
    (start, start + max_visible)
}

fn primary_column_width(items: &[(usize, &DialogItem)], width: usize) -> usize {
    let widest = items
        .iter()
        .map(|(_, item)| sanitize_single_line(&item.label).chars().count() + 2)
        .max()
        .unwrap_or(1);
    widest.clamp(1, width.saturating_sub(4).min(32).max(1))
}

fn render_dialog_item(
    item: &DialogItem,
    selected: bool,
    width: usize,
    primary_width: usize,
) -> Line<'static> {
    let prefix = if selected { "› " } else { "  " };
    let prefix_style = Style::new().fg(theme::AQUAMARINE);
    let label_style = if selected {
        Style::new()
            .fg(theme::AQUAMARINE)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let description_style = Style::new().fg(theme::MUTED_TEAL);
    let label_width = primary_width.saturating_sub(2).min(width.saturating_sub(4));
    let label = truncate_text(&sanitize_single_line(&item.label), label_width);

    let mut spans = vec![
        Span::styled(prefix.to_string(), prefix_style),
        Span::styled(label.clone(), label_style),
    ];

    let description = sanitize_single_line(&item.description);
    if !description.is_empty() && width > 40 {
        let spacing = " ".repeat(primary_width.saturating_sub(label.chars().count()).max(1));
        let used = 2 + label.chars().count() + spacing.chars().count();
        let desc_width = width.saturating_sub(used + 2);
        if desc_width > 10 {
            spans.push(Span::raw(spacing));
            spans.push(Span::styled(
                truncate_text(&description, desc_width),
                description_style,
            ));
        }
    }

    Line::from(spans)
}

fn sanitize_single_line(text: &str) -> String {
    text.replace('\t', "   ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn truncate_text(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    if width <= 1 {
        return "…".chars().take(width).collect();
    }
    let mut out: String = text.chars().take(width - 1).collect();
    out.push('…');
    out
}

fn dialog_help_line(width: usize, theme: &Theme) -> Line<'static> {
    Line::from(Span::styled(
        truncate_text(
            "↑↓: navigate  Enter: select  Esc: cancel  Type: filter",
            width,
        ),
        theme.dim,
    ))
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
        assert_eq!(dialog.state.selected(), Some(0));
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
        assert_eq!(dialog.state.selected(), Some(1));
        dialog.select_next();
        assert_eq!(dialog.state.selected(), Some(2));
        dialog.select_next(); // wraps
        assert_eq!(dialog.state.selected(), Some(0));
        dialog.select_previous();
        assert_eq!(dialog.state.selected(), Some(2));
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

    #[test]
    fn test_centered_rect() {
        let area = Rect::new(0, 0, 100, 50);
        let centered = centered_rect(60, 20, area);
        assert_eq!(centered.width, 60);
        assert_eq!(centered.height, 20);
        assert_eq!(centered.x, 20);
        assert_eq!(centered.y, 15);
    }

    #[test]
    fn test_visible_range_centers_selected_item() {
        assert_eq!(visible_range(6, 10, 5), (4, 9));
        assert_eq!(visible_range(0, 10, 5), (0, 5));
        assert_eq!(visible_range(9, 10, 5), (5, 10));
    }

    #[test]
    fn test_dialog_item_truncates_single_line_content() {
        let item = DialogItem {
            label: "very long model name".into(),
            description: "description\twith\nextra whitespace".into(),
            value: "model".into(),
        };
        let line = render_dialog_item(&item, true, 20, 12);
        let text: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert!(text.starts_with("› very long…"));
        assert!(!text.contains('\n'));
        assert!(!text.contains('\t'));
    }

    #[test]
    fn test_dialog_help_line_truncates_to_width() {
        let theme = Theme::default();
        let line = dialog_help_line(18, &theme);
        assert!(line.to_string().chars().count() <= 18);
        assert!(line.to_string().ends_with('…'));
    }
}
