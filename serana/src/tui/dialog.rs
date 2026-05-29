//! Interactive popup dialogs for model, theme, and session selection.

use display_protocol::{Color, Style, StyledLine, StyledSpan, ScreenBuffer};
use super::render_helpers::*;
use super::theme::{self, Theme};

/// Convert a ratatui Style (from Theme) to a display-protocol Style.
fn to_dp_style(s: &ratatui::style::Style) -> Style {
    use ratatui::style::{Color as RtColor, Modifier};
    let mut out = Style::new();
    if let Some(c) = s.fg {
        out.fg = Some(match c {
            RtColor::Rgb(r, g, b) => Color::new(r, g, b),
            RtColor::Reset => Color::WHITE,
            _ => Color::WHITE,
        });
    }
    if let Some(c) = s.bg {
        out.bg = Some(match c {
            RtColor::Rgb(r, g, b) => Color::new(r, g, b),
            RtColor::Reset => Color::BLACK,
            _ => Color::BLACK,
        });
    }
    out.bold = s.add_modifier.contains(Modifier::BOLD);
    out.italic = s.add_modifier.contains(Modifier::ITALIC);
    out.underline = s.add_modifier.contains(Modifier::UNDERLINED);
    out.dim = s.add_modifier.contains(Modifier::DIM);
    out
}

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

/// Render a centered popup dialog over the existing frame.
pub fn render_dialog(buf: &mut ScreenBuffer, dialog: &Dialog) {
    let theme = Theme::default();
    let aquamarine = Color::new(0, 180, 255);
    let bg = Color::BLACK;
    let width = buf.width;
    let height = buf.height;

    // Calculate popup size (centered, max 60x20 or 2/3 of screen)
    let popup_w = width.min(60).min(width * 2 / 3);
    let popup_h = height.min(20).min(height * 2 / 3);
    let popup_x = (width.saturating_sub(popup_w)) / 2;
    let popup_y = (height.saturating_sub(popup_h)) / 2;

    // Clear background
    clear_region(buf, popup_x, popup_y, popup_w, popup_h, bg);

    // Border with title
    paint_block(buf, popup_x, popup_y, popup_w, popup_h, aquamarine, bg, Some(&dialog.title));

    // Inner area (inside border)
    let inner_x = popup_x + 1;
    let inner_y = popup_y + 1;
    let inner_w = popup_w.saturating_sub(2);
    let inner_h = popup_h.saturating_sub(2);

    // Vertical layout: filter (1) + list (Min(3)) + help (1)
    let vlayout = compute_vertical_layout(inner_h, &[
        VConstraint::Length(1),
        VConstraint::Min(3),
        VConstraint::Length(1),
    ]);

    // Filter line
    let filter_y = inner_y + vlayout[0].0;
    let dim_style = theme.dim;
    let filter_line = if dialog.filter.is_empty() {
        StyledLine::new(vec![StyledSpan::new("Type to filter...", dim_style)])
    } else {
        StyledLine::new(vec![
            StyledSpan::new(&dialog.filter, Style::new().fg(aquamarine)),
            StyledSpan::new("▏", Style::new().fg(aquamarine).bold()),
        ])
    };
    paint_styled_line(buf, inner_x, filter_y, &filter_line);

    // List area
    let list_y = inner_y + vlayout[1].0;
    let list_h = vlayout[1].1;
    let list_width = inner_w as usize;

    let filtered = dialog.filtered_items();
    let selected_original = dialog.selected;
    let selected_filtered = selected_original
        .and_then(|selected| filtered.iter().position(|(idx, _)| *idx == selected))
        .unwrap_or(0);
    let show_count = list_h as usize > 1 && filtered.len() > list_h as usize;
    let visible_height = if show_count {
        (list_h as usize).saturating_sub(1)
    } else {
        list_h as usize
    };
    let (start, end) = visible_range(selected_filtered, filtered.len(), visible_height);

    let mut list_lines = Vec::new();
    if filtered.is_empty() {
        list_lines.push(StyledLine::new(vec![StyledSpan::new(
            "  No matching items",
            dim_style,
        )]));
    } else {
        let primary_width = primary_column_width(&filtered, list_width);
        for (pos, (_, item)) in filtered.iter().enumerate().take(end).skip(start) {
            list_lines.push(render_dialog_item(
                item,
                pos == selected_filtered,
                list_width,
                primary_width,
            ));
        }
    }

    paint_paragraph(buf, inner_x, list_y, inner_w, visible_height as u16, &list_lines, 0);

    if show_count && !filtered.is_empty() {
        let count_line = StyledLine::new(vec![StyledSpan::new(
            format!("  ({}/{})", selected_filtered + 1, filtered.len()),
            dim_style,
        )]);
        paint_styled_line(buf, inner_x, list_y + visible_height as u16, &count_line);
    }

    // Help line
    let help_y = inner_y + vlayout[2].0;
    let help = dialog_help_line(inner_w as usize, &theme);
    paint_styled_line(buf, inner_x, help_y, &help);
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
) -> StyledLine {
    let prefix = if selected { "› " } else { "  " };
    let prefix_style = Style::new().fg(Color::new(0, 180, 255));
    let label_style = if selected {
        Style::new().fg(Color::new(0, 180, 255)).bold()
    } else {
        Style::default()
    };
    let description_style = Style::new().fg(Color::new(156, 163, 176));
    let label_width = primary_width.saturating_sub(2).min(width.saturating_sub(4));
    let label = truncate_text(&sanitize_single_line(&item.label), label_width);

    let mut spans = vec![
        StyledSpan::new(prefix.to_string(), prefix_style),
        StyledSpan::new(label.clone(), label_style),
    ];

    let description = sanitize_single_line(&item.description);
    if !description.is_empty() && width > 40 {
        let spacing = " ".repeat(primary_width.saturating_sub(label.chars().count()).max(1));
        let used = 2 + label.chars().count() + spacing.chars().count();
        let desc_width = width.saturating_sub(used + 2);
        if desc_width > 10 {
            spans.push(StyledSpan::plain(spacing));
            spans.push(StyledSpan::new(
                truncate_text(&description, desc_width),
                description_style,
            ));
        }
    }

    StyledLine::new(spans)
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

fn dialog_help_line(width: usize, theme: &Theme) -> StyledLine {
    StyledLine::new(vec![StyledSpan::new(
        truncate_text(
            "↑↓: navigate  Enter: select  Esc: cancel  Type: filter",
            width,
        ),
        theme.dim,
    )])
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
        let text: String = line.spans.iter().map(|span| span.text.as_str()).collect();
        assert!(text.starts_with("› very long…"));
        assert!(!text.contains('\n'));
        assert!(!text.contains('\t'));
    }

    #[test]
    fn test_dialog_help_line_truncates_to_width() {
        let theme = Theme::default();
        let line = dialog_help_line(18, &theme);
        let text: String = line.spans.iter().map(|span| span.text.as_str()).collect();
        assert!(text.chars().count() <= 18);
        assert!(text.ends_with('…'));
    }
}
