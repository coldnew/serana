use display_protocol_widgets::{
    AssistantMessageSpec, BashExecutionSpec, BorderedLoaderSpec, BoxSpec, CancellableLoaderSpec,
    CountdownTimerSpec, DiffLineKindSpec, DiffSpec, DynamicBorderSpec, EditorSpec,
    ExecutionStatusSpec, FooterSpec, HistorySearchSpec, ImageSpec, InputSpec, KeybindingHintsSpec,
    LoaderSpec, LoginDialogSpec, MarkdownSpec, MessageFrameSpec, SelectListSpec, SelectorSpec,
    SettingsListSpec, StatusLineSpec, TabBarSpec, TextSpec, TodoReminderSpec, ToolExecutionSpec,
    TreeNodeSpec, TreeSelectorSpec, TruncatedTextSpec, UserMessageSpec, VisualTruncateSpec,
    WelcomeSpec, WidgetSpec,
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
        WidgetSpec::Editor(spec) => render_editor(spec, width),
        WidgetSpec::Image(spec) => render_image(spec, width),
        WidgetSpec::TabBar(spec) => render_tab_bar(spec, width),
        WidgetSpec::MessageFrame(spec) => render_message_frame(spec, width),
        WidgetSpec::AssistantMessage(spec) => render_assistant_message(spec, width),
        WidgetSpec::UserMessage(spec) => render_user_message(spec, width),
        WidgetSpec::ToolExecution(spec) => render_tool_execution(spec, width),
        WidgetSpec::BashExecution(spec) => render_bash_execution(spec, width),
        WidgetSpec::Diff(spec) => render_diff(spec, width),
        WidgetSpec::StatusLine(spec) => render_status_line(spec, width),
        WidgetSpec::Footer(spec) => render_footer(spec, width),
        WidgetSpec::KeybindingHints(spec) => render_keybinding_hints(spec, width),
        WidgetSpec::ModelSelector(spec)
        | WidgetSpec::SessionSelector(spec)
        | WidgetSpec::SettingsSelector(spec)
        | WidgetSpec::ThemeSelector(spec) => render_selector(spec, width),
        WidgetSpec::TreeSelector(spec) => render_tree_selector(spec, width),
        WidgetSpec::LoginDialog(spec) => render_login_dialog(spec, width),
        WidgetSpec::HistorySearch(spec) => render_history_search(spec, width),
        WidgetSpec::CountdownTimer(spec) => render_countdown_timer(spec, width),
        WidgetSpec::TodoReminder(spec) => render_todo_reminder(spec, width),
        WidgetSpec::Welcome(spec) => render_welcome(spec, width),
        WidgetSpec::BorderedLoader(spec) => render_bordered_loader(spec, width),
        WidgetSpec::DynamicBorder(spec) => render_dynamic_border(spec, width),
        WidgetSpec::VisualTruncate(spec) => render_visual_truncate(spec, width),
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

fn render_editor(spec: &EditorSpec, width: u16) -> Vec<String> {
    let height = spec.height.max(1) as usize;
    let start = spec.scroll_top.min(spec.lines.len());
    let end = (start + height).min(spec.lines.len());
    let gutter_width = if spec.show_gutter {
        spec.lines.len().max(1).to_string().len() + 2
    } else {
        0
    };
    let content_width = (width as usize).saturating_sub(gutter_width + 2).max(1);
    let mut lines = Vec::new();

    for row in start..end {
        let source = spec.lines.get(row).map(String::as_str).unwrap_or_default();
        let marker = if row == spec.cursor_line { ">" } else { " " };
        let mut content = source.to_string();
        if spec.focused && row == spec.cursor_line {
            content = insert_cursor(&content, spec.cursor_col);
        }
        let gutter = if spec.show_gutter {
            format!("{marker}{:>width$} ", row + 1, width = gutter_width - 2)
        } else {
            marker.to_string()
        };
        lines.push(truncate_to_width(
            &format!("{gutter}{}", truncate_to_width(&content, content_width)),
            width as usize,
        ));
    }

    while lines.len() < height {
        lines.push(String::new());
    }
    lines
}

fn render_image(spec: &ImageSpec, width: u16) -> Vec<String> {
    let image_width = spec.width.min(width).max(4);
    let image_height = spec.height.max(1);
    let mut lines = Vec::new();
    let title = format!(" image:{} ", spec.frame_id);
    lines.push(border_top(&title, image_width as usize));
    for index in 0..image_height.saturating_sub(2) {
        let body = if index == 0 {
            spec.alt.as_deref().unwrap_or("backend image frame")
        } else {
            ""
        };
        lines.push(border_line(body, image_width as usize));
    }
    if image_height > 1 {
        lines.push(border_bottom(image_width as usize));
    }
    lines
}

fn render_tab_bar(spec: &TabBarSpec, width: u16) -> Vec<String> {
    let active = spec.active.min(spec.items.len().saturating_sub(1));
    let tabs = spec
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let marker = if item.modified { " ●" } else { "" };
            if index == active {
                format!("▌ {}{} ", item.label, marker)
            } else {
                format!("  {}{} ", item.label, marker)
            }
        })
        .collect::<String>();
    vec![truncate_to_width(&tabs, width as usize)]
}

fn render_message_frame(spec: &MessageFrameSpec, width: u16) -> Vec<String> {
    let mut body = Vec::new();
    for child in &spec.body {
        body.extend(render_widget_spec_to_lines(
            child,
            width.saturating_sub(4).max(1),
        ));
    }
    trim_rendered_lines(&mut body);
    render_panel(&spec.title, &body, width)
}

fn render_assistant_message(spec: &AssistantMessageSpec, width: u16) -> Vec<String> {
    let title = spec
        .model
        .as_ref()
        .map(|model| format!("Assistant · {model}"))
        .unwrap_or_else(|| "Assistant".to_string());
    let body = render_markdown(
        &MarkdownSpec::new(&spec.text).padding(0, 0),
        width.saturating_sub(4),
    );
    render_panel(&title, &body, width)
}

fn render_user_message(spec: &UserMessageSpec, width: u16) -> Vec<String> {
    let body = wrap_text(&spec.text, width.saturating_sub(4).max(1) as usize);
    render_panel("User", &body, width)
}

fn render_tool_execution(spec: &ToolExecutionSpec, width: u16) -> Vec<String> {
    let mut body = vec![format!(
        "{} {}",
        status_icon(spec.status),
        status_label(spec.status)
    )];
    if let Some(command) = &spec.command {
        body.push(format!("$ {command}"));
    }
    body.extend(spec.output.iter().cloned());
    render_panel(&spec.title, &body, width)
}

fn render_bash_execution(spec: &BashExecutionSpec, width: u16) -> Vec<String> {
    let exit = spec
        .exit_code
        .map(|code| format!(" · exit {code}"))
        .unwrap_or_default();
    let mut body = vec![
        format!(
            "{} {}{}",
            status_icon(spec.status),
            status_label(spec.status),
            exit
        ),
        format!("$ {}", spec.command),
    ];
    body.extend(spec.output.iter().cloned());
    render_panel("Bash", &body, width)
}

fn render_diff(spec: &DiffSpec, width: u16) -> Vec<String> {
    let body = spec
        .lines
        .iter()
        .map(|line| {
            let prefix = match line.kind {
                DiffLineKindSpec::Context => " ",
                DiffLineKindSpec::Added => "+",
                DiffLineKindSpec::Removed => "-",
                DiffLineKindSpec::Hunk => "@",
            };
            truncate_to_width(
                &format!("{prefix} {}", line.text),
                width.saturating_sub(4) as usize,
            )
        })
        .collect::<Vec<_>>();
    render_panel(&spec.title, &body, width)
}

fn render_status_line(spec: &StatusLineSpec, width: u16) -> Vec<String> {
    let left = render_segments(&spec.left);
    let right = render_segments(&spec.right);
    let width = width as usize;
    let used = left.chars().count() + right.chars().count();
    let gap = if used < width {
        " ".repeat(width - used)
    } else {
        " ".to_string()
    };
    vec![truncate_to_width(&format!("{left}{gap}{right}"), width)]
}

fn render_footer(spec: &FooterSpec, width: u16) -> Vec<String> {
    vec![truncate_to_width(&spec.items.join(" · "), width as usize)]
}

fn render_keybinding_hints(spec: &KeybindingHintsSpec, width: u16) -> Vec<String> {
    let line = spec
        .hints
        .iter()
        .map(|hint| format!("{} {}", hint.key, hint.label))
        .collect::<Vec<_>>()
        .join("   ");
    vec![truncate_to_width(&line, width as usize)]
}

fn render_selector(spec: &SelectorSpec, width: u16) -> Vec<String> {
    let selected = spec.selected.min(spec.options.len().saturating_sub(1));
    let start = selected
        .saturating_sub(spec.max_visible / 2)
        .min(spec.options.len().saturating_sub(spec.max_visible));
    let end = (start + spec.max_visible).min(spec.options.len());
    let mut body = Vec::new();
    for index in start..end {
        let item = &spec.options[index];
        let prefix = if index == selected { "→ " } else { "  " };
        let suffix = item
            .description
            .as_ref()
            .map(|description| format!("  {description}"))
            .unwrap_or_default();
        body.push(truncate_to_width(
            &format!("{prefix}{}{}", item.label, suffix),
            width.saturating_sub(4) as usize,
        ));
    }
    render_panel(&spec.title, &body, width)
}

fn render_tree_selector(spec: &TreeSelectorSpec, width: u16) -> Vec<String> {
    let mut body = Vec::new();
    for (index, root) in spec.roots.iter().enumerate() {
        render_tree_node(
            root,
            &[index],
            0,
            &spec.selected_path,
            width.saturating_sub(4),
            &mut body,
        );
    }
    render_panel(&spec.title, &body, width)
}

fn render_login_dialog(spec: &LoginDialogSpec, width: u16) -> Vec<String> {
    render_panel(
        &format!("Login · {}", spec.provider),
        &[
            format!("Open: {}", spec.verification_uri),
            format!("Code: {}", spec.user_code),
            spec.status.clone(),
        ],
        width,
    )
}

fn render_history_search(spec: &HistorySearchSpec, width: u16) -> Vec<String> {
    let selected = spec.selected.min(spec.results.len().saturating_sub(1));
    let mut body = vec![format!("/{}", spec.query)];
    for (index, result) in spec.results.iter().enumerate() {
        let prefix = if index == selected { "→ " } else { "  " };
        body.push(format!("{prefix}{result}"));
    }
    render_panel("History Search", &body, width)
}

fn render_countdown_timer(spec: &CountdownTimerSpec, width: u16) -> Vec<String> {
    vec![truncate_to_width(
        &format!("{}: {}s", spec.label, spec.remaining_seconds),
        width as usize,
    )]
}

fn render_todo_reminder(spec: &TodoReminderSpec, width: u16) -> Vec<String> {
    let body = spec
        .items
        .iter()
        .map(|item| truncate_to_width(&format!("☐ {item}"), width.saturating_sub(4) as usize))
        .collect::<Vec<_>>();
    render_panel("Todo", &body, width)
}

fn render_welcome(spec: &WelcomeSpec, width: u16) -> Vec<String> {
    let mut body = vec![spec.subtitle.clone()];
    body.extend(spec.actions.iter().map(|action| format!("• {action}")));
    render_panel(&spec.title, &body, width)
}

fn render_bordered_loader(spec: &BorderedLoaderSpec, width: u16) -> Vec<String> {
    let body = render_loader(&spec.loader, width.saturating_sub(4));
    render_panel(&spec.title, &body, width)
}

fn render_dynamic_border(spec: &DynamicBorderSpec, width: u16) -> Vec<String> {
    let title = if spec.active {
        format!("● {}", spec.title)
    } else {
        spec.title.clone()
    };
    let body = render_widget_spec_to_lines(&spec.child, width.saturating_sub(4));
    render_panel(&title, &body, width)
}

fn render_visual_truncate(spec: &VisualTruncateSpec, width: u16) -> Vec<String> {
    vec![truncate_with_suffix(
        &spec.text,
        &spec.suffix,
        width as usize,
    )]
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

fn render_panel(title: &str, body: &[String], width: u16) -> Vec<String> {
    let width = width.max(4) as usize;
    let mut lines = vec![border_top(title, width)];
    if body.is_empty() {
        lines.push(border_line("", width));
    } else {
        for line in body {
            lines.push(border_line(line, width));
        }
    }
    lines.push(border_bottom(width));
    lines
}

fn border_top(title: &str, width: usize) -> String {
    let title = truncate_to_width(title, width.saturating_sub(4));
    let title_width = title.chars().count();
    let fill = width.saturating_sub(title_width + 3);
    format!("┌─{title}{}┐", "─".repeat(fill))
}

fn border_bottom(width: usize) -> String {
    format!("└{}┘", "─".repeat(width.saturating_sub(2)))
}

fn border_line(text: &str, width: usize) -> String {
    let content_width = width.saturating_sub(4);
    format!(
        "│ {} │",
        pad_to_width(truncate_to_width(text, content_width), content_width)
    )
}

fn render_segments(segments: &[display_protocol_widgets::StatusSegmentSpec]) -> String {
    segments
        .iter()
        .map(|segment| {
            segment
                .value
                .as_ref()
                .map(|value| format!("{}: {value}", segment.label))
                .unwrap_or_else(|| segment.label.clone())
        })
        .collect::<Vec<_>>()
        .join("  ")
}

fn status_icon(status: ExecutionStatusSpec) -> &'static str {
    match status {
        ExecutionStatusSpec::Pending => "○",
        ExecutionStatusSpec::Running => "⠸",
        ExecutionStatusSpec::Success => "✓",
        ExecutionStatusSpec::Failed => "✕",
        ExecutionStatusSpec::Cancelled => "⊘",
    }
}

fn status_label(status: ExecutionStatusSpec) -> &'static str {
    match status {
        ExecutionStatusSpec::Pending => "pending",
        ExecutionStatusSpec::Running => "running",
        ExecutionStatusSpec::Success => "success",
        ExecutionStatusSpec::Failed => "failed",
        ExecutionStatusSpec::Cancelled => "cancelled",
    }
}

fn render_tree_node(
    node: &TreeNodeSpec,
    path: &[usize],
    depth: usize,
    selected_path: &[usize],
    width: u16,
    lines: &mut Vec<String>,
) {
    let selected = path == selected_path;
    let marker = if selected { "→" } else { " " };
    let branch = if node.children.is_empty() {
        " "
    } else if node.expanded {
        "▼"
    } else {
        "▶"
    };
    let indent = "  ".repeat(depth);
    lines.push(truncate_to_width(
        &format!("{marker}{indent}{branch} {}", node.label),
        width as usize,
    ));

    if node.expanded {
        for (index, child) in node.children.iter().enumerate() {
            let mut child_path = path.to_vec();
            child_path.push(index);
            render_tree_node(child, &child_path, depth + 1, selected_path, width, lines);
        }
    }
}

fn insert_cursor(text: &str, cursor_col: usize) -> String {
    let mut out = String::new();
    let mut inserted = false;
    for (index, ch) in text.chars().enumerate() {
        if index == cursor_col {
            out.push('▉');
            inserted = true;
        }
        out.push(ch);
    }
    if !inserted {
        out.push('▉');
    }
    out
}

fn truncate_with_suffix(text: &str, suffix: &str, width: usize) -> String {
    let text_len = text.chars().count();
    if text_len <= width {
        return text.to_string();
    }
    let suffix_len = suffix.chars().count();
    if suffix_len >= width {
        return suffix.chars().take(width).collect();
    }
    let keep = width - suffix_len;
    format!("{}{}", text.chars().take(keep).collect::<String>(), suffix)
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

    #[test]
    fn editor_renders_cursor_and_gutter() {
        let lines = render_editor(
            &EditorSpec::new(vec!["fn main() {".to_string(), "}".to_string()])
                .cursor(0, 3)
                .height(2)
                .focused(true),
            40,
        );

        assert!(lines[0].contains(">1 fn ▉main() {"));
    }

    #[test]
    fn selector_renders_as_bordered_panel() {
        let lines = render_selector(
            &SelectorSpec::new(
                "Model",
                vec![
                    display_protocol_widgets::SelectorOptionSpec::new("gpt-5", "gpt-5"),
                    display_protocol_widgets::SelectorOptionSpec::new("mini", "gpt-5-mini"),
                ],
            )
            .selected(1),
            40,
        );

        assert_eq!(lines[0], "┌─Model────────────────────────────────┐");
        assert!(lines.iter().any(|line| line.contains("→ gpt-5-mini")));
    }
}
