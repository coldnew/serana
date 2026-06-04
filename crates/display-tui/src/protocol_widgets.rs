use display_protocol_widgets::{
    BoxSpec, CancellableLoaderSpec, InputSpec, LoaderSpec, MarkdownSpec, SelectListSpec,
    SettingsListSpec, TextSpec, TruncatedTextSpec, WidgetSpec,
};

pub fn render_widget_spec_to_lines(spec: &WidgetSpec, width: u16) -> Vec<String> {
    match spec {
        WidgetSpec::Text(spec) => render_text(spec, width),
        WidgetSpec::Box(spec) => render_box(spec, width),
        WidgetSpec::Spacer(spec) => vec![String::new(); spec.lines as usize],
        WidgetSpec::TruncatedText(spec) => render_truncated_text(spec, width),
        WidgetSpec::Input(spec) => render_input(spec, width),
        WidgetSpec::Loader(spec) => render_loader(spec, width),
        WidgetSpec::CancellableLoader(spec) => render_cancellable_loader(spec, width),
        WidgetSpec::SelectList(spec) => render_select_list(spec, width),
        WidgetSpec::SettingsList(spec) => render_settings_list(spec, width),
        WidgetSpec::Markdown(spec) => render_markdown(spec, width),
    }
}

pub fn trim_rendered_lines(lines: &mut Vec<String>) {
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
}

fn render_text(spec: &TextSpec, width: u16) -> Vec<String> {
    if spec.text.trim().is_empty() {
        return Vec::new();
    }
    let empty = " ".repeat(width as usize);
    let content_width = width
        .saturating_sub(spec.padding_x.saturating_mul(2))
        .max(1) as usize;
    let mut lines = Vec::new();
    for _ in 0..spec.padding_y {
        lines.push(empty.clone());
    }
    for line in wrap_text(&spec.text.replace('\t', "   "), content_width) {
        lines.push(padded_line(&line, width as usize, spec.padding_x as usize));
    }
    for _ in 0..spec.padding_y {
        lines.push(empty.clone());
    }
    lines
}

fn render_box(spec: &BoxSpec, width: u16) -> Vec<String> {
    if spec.children.is_empty() {
        return Vec::new();
    }
    let content_width = width
        .saturating_sub(spec.padding_x.saturating_mul(2))
        .max(1);
    let mut child_lines = Vec::new();
    for child in &spec.children {
        for line in render_widget_spec_to_lines(child, content_width) {
            child_lines.push(format!("{}{}", " ".repeat(spec.padding_x as usize), line));
        }
    }
    if child_lines.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    for _ in 0..spec.padding_y {
        lines.push(" ".repeat(width as usize));
    }
    for line in child_lines {
        lines.push(pad_to_width(line, width as usize));
    }
    for _ in 0..spec.padding_y {
        lines.push(" ".repeat(width as usize));
    }
    lines
}

fn render_truncated_text(spec: &TruncatedTextSpec, width: u16) -> Vec<String> {
    let available = width
        .saturating_sub(spec.padding_x.saturating_mul(2))
        .max(1) as usize;
    let text = spec.text.lines().next().unwrap_or_default();
    let line = padded_line(
        &truncate_to_width(text, available),
        width as usize,
        spec.padding_x as usize,
    );
    let mut lines = Vec::new();
    for _ in 0..spec.padding_y {
        lines.push(" ".repeat(width as usize));
    }
    lines.push(line);
    for _ in 0..spec.padding_y {
        lines.push(" ".repeat(width as usize));
    }
    lines
}

fn render_input(spec: &InputSpec, width: u16) -> Vec<String> {
    let prompt_width = spec.prompt.chars().count();
    let available = (width as usize).saturating_sub(prompt_width).max(1);
    let cursor = spec.cursor.min(spec.value.len());
    let before = &spec.value[..cursor];
    let at_cursor = spec.value[cursor..].chars().next().unwrap_or(' ');
    let after_start = cursor
        + at_cursor
            .len_utf8()
            .min(spec.value.len().saturating_sub(cursor));
    let after = spec.value.get(after_start..).unwrap_or_default();
    let visible = if spec.focused {
        format!("{before}▉{after}")
    } else {
        spec.value.clone()
    };
    vec![format!(
        "{}{}",
        spec.prompt,
        pad_to_width(truncate_to_width(&visible, available), available)
    )]
}

fn render_loader(spec: &LoaderSpec, width: u16) -> Vec<String> {
    let frame = spec
        .frames
        .get(spec.frame % spec.frames.len().max(1))
        .map(String::as_str)
        .unwrap_or("");
    vec![truncate_to_width(
        &format!("{frame} {}", spec.message),
        width as usize,
    )]
}

fn render_cancellable_loader(spec: &CancellableLoaderSpec, width: u16) -> Vec<String> {
    let mut lines = render_loader(&spec.loader, width);
    if let Some(line) = lines.first_mut() {
        *line = truncate_to_width(&format!("{line}  {}", spec.cancel_hint), width as usize);
    }
    lines
}

fn render_select_list(spec: &SelectListSpec, width: u16) -> Vec<String> {
    if spec.items.is_empty() {
        return vec!["  No matching commands".to_string()];
    }
    let selected = spec.selected.min(spec.items.len().saturating_sub(1));
    let start = selected
        .saturating_sub(spec.max_visible / 2)
        .min(spec.items.len().saturating_sub(spec.max_visible));
    let end = (start + spec.max_visible).min(spec.items.len());
    let column_width = primary_column_width(spec);
    let mut lines = Vec::new();
    for index in start..end {
        let item = &spec.items[index];
        let prefix = if index == selected { "→ " } else { "  " };
        let label = truncate_to_width(&item.label, column_width.saturating_sub(2));
        let line = if let Some(description) = &item.description {
            let spacing = " ".repeat(column_width.saturating_sub(label.chars().count()).max(1));
            format!("{prefix}{label}{spacing}{description}")
        } else {
            format!("{prefix}{label}")
        };
        lines.push(truncate_to_width(&line, width as usize));
    }
    if start > 0 || end < spec.items.len() {
        lines.push(format!("  ({}/{})", selected + 1, spec.items.len()));
    }
    lines
}

fn render_settings_list(spec: &SettingsListSpec, width: u16) -> Vec<String> {
    if spec.items.is_empty() {
        return vec!["  No settings available".to_string()];
    }
    let selected = spec.selected.min(spec.items.len().saturating_sub(1));
    let start = selected
        .saturating_sub(spec.max_visible / 2)
        .min(spec.items.len().saturating_sub(spec.max_visible));
    let end = (start + spec.max_visible).min(spec.items.len());
    let label_width = spec
        .items
        .iter()
        .map(|item| item.label.chars().count())
        .max()
        .unwrap_or(1)
        .min(30);
    let mut lines = Vec::new();
    for index in start..end {
        let item = &spec.items[index];
        let prefix = if index == selected { "→ " } else { "  " };
        let label = truncate_to_width(&item.label, label_width);
        let spacing = " ".repeat(label_width.saturating_sub(label.chars().count()));
        let value_width = (width as usize).saturating_sub(prefix.len() + label_width + 4);
        let value = truncate_to_width(&item.current_value, value_width);
        lines.push(format!("{prefix}{label}{spacing}  {value}"));
    }
    if let Some(description) = &spec.items[selected].description {
        lines.push(String::new());
        for line in wrap_text(description, width.saturating_sub(4).max(1) as usize) {
            lines.push(format!("  {line}"));
        }
    }
    if spec.show_hint {
        lines.push(String::new());
        lines.push(truncate_to_width(
            &format!("  {}", spec.hint),
            width as usize,
        ));
    }
    lines
}

fn render_markdown(spec: &MarkdownSpec, width: u16) -> Vec<String> {
    let content_width = width
        .saturating_sub(spec.padding_x.saturating_mul(2))
        .max(1) as usize;
    let mut lines = Vec::new();
    for _ in 0..spec.padding_y {
        lines.push(" ".repeat(width as usize));
    }
    for raw in spec.text.lines() {
        let rendered = if let Some(text) = raw.strip_prefix("# ") {
            text.to_string()
        } else if let Some(text) = raw.strip_prefix("## ") {
            text.to_string()
        } else if let Some(text) = raw.strip_prefix("> ") {
            format!("▌ {text}")
        } else if let Some(text) = raw.strip_prefix("- ") {
            format!("• {text}")
        } else {
            raw.to_string()
        };
        for line in wrap_text(&rendered, content_width) {
            lines.push(padded_line(&line, width as usize, spec.padding_x as usize));
        }
    }
    for _ in 0..spec.padding_y {
        lines.push(" ".repeat(width as usize));
    }
    lines
}

fn primary_column_width(spec: &SelectListSpec) -> usize {
    let widest = spec
        .items
        .iter()
        .map(|item| item.label.chars().count() + 2)
        .max()
        .unwrap_or(1);
    widest.clamp(spec.min_primary_column_width, spec.max_primary_column_width)
}

fn padded_line(text: &str, width: usize, padding_x: usize) -> String {
    pad_to_width(
        format!("{}{}{}", " ".repeat(padding_x), text, " ".repeat(padding_x)),
        width,
    )
}

fn pad_to_width(mut text: String, width: usize) -> String {
    let len = text.chars().count();
    if len < width {
        text.push_str(&" ".repeat(width - len));
    }
    text
}

fn truncate_to_width(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for raw_line in text.lines() {
        let mut current = String::new();
        for word in raw_line.split_whitespace() {
            let current_len = current.chars().count();
            let word_len = word.chars().count();
            if current_len > 0 && current_len + 1 + word_len > width {
                out.push(current);
                current = String::new();
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
        if current.is_empty() {
            out.push(String::new());
        } else {
            out.push(current);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use display_protocol_widgets::{SelectItemSpec, SettingItemSpec};

    #[test]
    fn select_list_renders_cursor_and_description_columns() {
        let lines = render_select_list(
            &SelectListSpec::new(vec![
                SelectItemSpec::new("one", "One").description("First"),
                SelectItemSpec::new("two", "Two").description("Second"),
            ])
            .selected(1),
            60,
        );

        assert_eq!(lines[1], "→ Two                             Second");
    }

    #[test]
    fn settings_list_renders_selected_description_and_hint() {
        let lines = render_settings_list(
            &SettingsListSpec::new(vec![
                SettingItemSpec::new("model", "Model", "gpt-5"),
                SettingItemSpec::new("approval", "Approval", "on-request")
                    .description("Esc cancels changes"),
            ])
            .selected(1),
            60,
        );

        assert!(lines.iter().any(|line| line == "  Esc cancels changes"));
    }
}
