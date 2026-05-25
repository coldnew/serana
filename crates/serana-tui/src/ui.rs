use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Padding, Paragraph, Wrap,
};
use ratatui::Frame;

use crate::app::{App, AppMode, ChatMessage, MessageRole, TodoItem, TodoStatus};
use crate::markdown::render_markdown;
use crate::symbols::Symbols;
use crate::theme::{self, Theme};

const PI_LOGO: &[&str] = &[
    "▀████████████▀",
    " ╘███    ███  ",
    "  ███    ███  ",
    "  ███    ███  ",
    " ▄███▄  ▄███▄ ",
];

fn gradient_logo_style(_area: Rect) -> Vec<(String, Style)> {
    let colors = [
        ratatui::style::Color::Rgb(255, 92, 200),
        ratatui::style::Color::Rgb(200, 110, 255),
        ratatui::style::Color::Rgb(120, 130, 255),
        ratatui::style::Color::Rgb(60, 200, 255),
        ratatui::style::Color::Rgb(120, 255, 220),
    ];
    PI_LOGO
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let color_idx = (i * colors.len() / PI_LOGO.len()).min(colors.len() - 1);
            (line.to_string(), Style::new().fg(colors[color_idx]))
        })
        .collect()
}

fn center_text(text: &str, width: usize) -> String {
    if text.len() >= width {
        return text.chars().take(width).collect();
    }
    let left_pad = (width - text.len()) / 2;
    let right_pad = width - text.len() - left_pad;
    format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
}

fn _fit_to_width(text: String, width: usize) -> String {
    if text.len() > width {
        text.chars().take(width).collect()
    } else {
        format!("{}{}", text, " ".repeat(width - text.len()))
    }
}

fn render_welcome(frame: &mut Frame, area: Rect, app: &App) {
    let theme = Theme::default();
    let s = app.symbols;

    if area.width < 4 || area.height < 4 {
        return;
    }

    let title = format!(" Serana v{} ", app.version);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border)
        .title(Span::styled(title, theme.muted));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let show_right = inner.width >= 58;
    let cols = if show_right {
        Layout::horizontal([
            Constraint::Percentage(35),
            Constraint::Length(1),
            Constraint::Percentage(65),
        ])
        .split(inner)
    } else {
        Layout::horizontal([Constraint::Percentage(100)]).split(inner)
    };
    let left_area = cols[0];
    let right_area = if show_right { Some(cols[2]) } else { None };

    let mut left_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            center_text("Welcome back!", left_area.width as usize),
            theme.accent,
        )),
        Line::from(""),
    ];

    let logo_styled = gradient_logo_style(left_area);
    for (text, style) in &logo_styled {
        left_lines.push(Line::from(Span::styled(
            center_text(text, left_area.width as usize),
            *style,
        )));
    }

    left_lines.push(Line::from(""));
    left_lines.push(Line::from(Span::styled(
        center_text(&app.model, left_area.width as usize),
        theme.muted,
    )));
    left_lines.push(Line::from(Span::styled(
        center_text(&app.provider, left_area.width as usize),
        theme.dim,
    )));

    let left_para = Paragraph::new(left_lines);
    frame.render_widget(left_para, left_area);

    let Some(right_area) = right_area else {
        return;
    };

    let divider = Paragraph::new(vec![
        Line::from(Span::styled(s.box_round.vertical, theme.border));
        right_area.height as usize
    ]);
    frame.render_widget(divider, cols[1]);

    let mut right_lines: Vec<Line> = Vec::new();
    let sep = format!(" {}", s.hr_char.repeat(right_area.width.saturating_sub(3) as usize));

    right_lines.push(Line::from(Span::styled(
        format!(" {}", "Tips"),
        theme.accent,
    )));
    right_lines.push(Line::from(vec![
        Span::styled(" ?", theme.dim),
        Span::styled(" for keyboard shortcuts", theme.muted),
    ]));
    right_lines.push(Line::from(vec![
        Span::styled(" #", theme.dim),
        Span::styled(" for prompt actions", theme.muted),
    ]));
    right_lines.push(Line::from(vec![
        Span::styled(" /", theme.dim),
        Span::styled(" for commands", theme.muted),
    ]));
    right_lines.push(Line::from(vec![
        Span::styled(" !", theme.dim),
        Span::styled(" to run bash", theme.muted),
    ]));
    right_lines.push(Line::from(vec![
        Span::styled(" $", theme.dim),
        Span::styled(" to run python", theme.muted),
    ]));
    right_lines.push(Line::from(Span::styled(&sep, theme.dim)));
    right_lines.push(Line::from(Span::styled(" LSP Servers", theme.accent)));
    right_lines.push(Line::from(Span::styled(
        format!("  {} No LSP servers", s.pending),
        theme.dim,
    )));
    right_lines.push(Line::from(Span::styled(&sep, theme.dim)));
    right_lines.push(Line::from(Span::styled(
        " Recent sessions",
        theme.accent,
    )));
    if app.recent_sessions.is_empty() {
        right_lines.push(Line::from(Span::styled(
            format!("  {} No recent sessions", s.bullet),
            theme.dim,
        )));
    } else {
        for session in app.recent_sessions.iter().take(3) {
            let name = session.title.as_deref().unwrap_or(&session.id);
            let date = session.updated_at.format("%m-%d %H:%M");
            right_lines.push(Line::from(vec![
                Span::styled(format!("  {} ", s.bullet), theme.dim),
                Span::styled(
                    truncate_text(name, right_area.width.saturating_sub(18) as usize),
                    theme.muted,
                ),
                Span::styled(format!(" ({})", date), theme.dim),
            ]));
        }
    }

    let right_para = Paragraph::new(right_lines);
    frame.render_widget(right_para, right_area);
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

fn clamp_line(line: Line<'static>, width: usize) -> Line<'static> {
    if width == 0 {
        return Line::from("");
    }
    let mut remaining = width;
    let mut spans = Vec::new();
    for span in line.spans {
        if remaining == 0 {
            break;
        }
        let text = span.content.replace('\t', "   ");
        let text_len = text.chars().count();
        if text_len <= remaining {
            spans.push(Span::styled(text, span.style));
            remaining -= text_len;
        } else {
            let truncated: String = text.chars().take(remaining).collect();
            spans.push(Span::styled(truncated, span.style));
            remaining = 0;
        }
    }
    Line::from(spans)
}

fn render_message_para(msg: &ChatMessage, width: usize, theme: &Theme, symbols: &Symbols) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    match msg.role {
        MessageRole::User => {
            lines.push(Line::from(""));
            let md_theme = crate::markdown::MarkdownTheme::default();
            let user_style = Style::new().fg(theme::MUTED_TEAL).bg(theme::USER_MSG_BG);
            let md_lines = render_markdown(&msg.content, &md_theme, width.saturating_sub(4));
            for md_line in md_lines {
                let mut spans = vec![Span::styled("  ".to_string(), user_style)];
                spans.extend(md_line.spans.into_iter().map(|span| {
                    Span::styled(span.content.into_owned(), span.style.patch(user_style))
                }));
                lines.push(Line::from(spans));
            }
        }
        MessageRole::Agent => {
            lines.push(Line::from(""));

            if let Some(ref thinking) = msg.thinking {
                for t_line in thinking.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", t_line),
                        theme.thinking,
                    )));
                }
                lines.push(Line::from(""));
            }

            let md_theme = crate::markdown::MarkdownTheme::default();
            let md_lines = render_markdown(&msg.content, &md_theme, width.saturating_sub(4));
            for md_line in md_lines {
                let mut prefixed = vec![Span::raw("    ")];
                prefixed.extend(md_line.spans.into_iter());
                lines.push(Line::from(prefixed));
            }

            for tool in &msg.tool_calls {
                let tool_lines =
                    crate::tool_execution::render_tool_call(tool, symbols, width.saturating_sub(2));
                lines.extend(tool_lines);
            }
        }
        MessageRole::System => {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {} {}", symbols.warning, msg.content),
                theme.warning,
            )));
        }
    }

    lines
}

fn render_messages(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = Theme::default();
    let s = app.symbols;
    let mut all_lines: Vec<Line<'static>> = Vec::new();

    for msg in &app.messages {
        let mut msg_lines = render_message_para(msg, area.width as usize, &theme, s);
        all_lines.append(&mut msg_lines);
    }

    if !app.pending_messages.is_empty() && app.mode == AppMode::Processing {
        all_lines.push(Line::from(""));
        all_lines.extend(render_processing_lines(
            &app.pending_messages,
            (app.tick_count % s.spinner.len() as u64) as usize,
            &theme,
            s,
            area.width as usize,
        ));
    }

    if !app.status_messages.is_empty() {
        all_lines.push(Line::from(""));
        all_lines.extend(render_status_notice_lines(
            &app.status_messages,
            &theme,
            area.width as usize,
        ));
    }

    if !app.todo_items.is_empty() {
        all_lines.push(Line::from(""));
        all_lines.extend(render_todo_lines(&app.todo_items, &theme));
    }

    if !app.btw_notes.is_empty() {
        all_lines.push(Line::from(""));
        all_lines.extend(render_btw_lines(
            &app.btw_notes,
            &theme,
            s,
            area.width as usize,
        ));
    }

    all_lines.push(Line::from(""));
    all_lines = all_lines
        .into_iter()
        .map(|line| clamp_line(line, area.width as usize))
        .collect();

    let scroll_offset = app.scroll.min(all_lines.len().saturating_sub(area.height as usize));

    let para = Paragraph::new(all_lines)
        .scroll((scroll_offset as u16, 0))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(theme.border),
        );

    frame.render_widget(para, area);
}

fn render_todo_lines(items: &[TodoItem], theme: &Theme) -> Vec<Line<'static>> {
    let incomplete = items
        .iter()
        .filter(|todo| todo.status == TodoStatus::Pending || todo.status == TodoStatus::InProgress)
        .count();
    let label = if incomplete == 1 { "todo" } else { "todos" };
    let mut lines = vec![Line::from(vec![
        Span::styled("  Tasks", theme.accent),
        Span::styled(
            format!("  {} incomplete {} / {} total", incomplete, label, items.len()),
            theme.dim,
        ),
    ])];

    for todo in items {
        let (marker, style) = todo_marker_style(todo.status, theme);
        lines.push(Line::from(vec![
            Span::raw("  - ["),
            Span::styled(marker.to_string(), style),
            Span::raw("] "),
            Span::styled(todo.content.clone(), style),
        ]));
    }

    lines
}

fn todo_marker_style(status: TodoStatus, theme: &Theme) -> (&'static str, Style) {
    match status {
        TodoStatus::Pending => (" ", theme.muted),
        TodoStatus::InProgress => (
            "/",
            Style::new()
                .fg(theme::AQUAMARINE)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        TodoStatus::Done => ("x", theme.dim),
        TodoStatus::Abandoned => (
            "-",
            Style::default().add_modifier(
                ratatui::style::Modifier::DIM | ratatui::style::Modifier::CROSSED_OUT,
            ),
        ),
    }
}

fn render_btw_lines(
    notes: &[String],
    theme: &Theme,
    symbols: &Symbols,
    width: usize,
) -> Vec<Line<'static>> {
    let panel_width = width.saturating_sub(2).max(4);
    let inner_width = panel_width.saturating_sub(4);
    let title = truncate_text(" By the way ", panel_width.saturating_sub(2));
    let fill = panel_width.saturating_sub(2 + title.chars().count());
    let left_fill = fill / 2;
    let right_fill = fill - left_fill;

    let mut lines = vec![Line::from(vec![
        Span::styled(
            format!(
                "  {}{}",
                symbols.box_round.top_left,
                symbols.box_round.horizontal.repeat(left_fill)
            ),
            theme.border,
        ),
        Span::styled(title, theme.warning),
        Span::styled(
            format!(
                "{}{}",
                symbols.box_round.horizontal.repeat(right_fill),
                symbols.box_round.top_right
            ),
            theme.border,
        ),
    ])];

    for note in notes {
        let bullet = format!("{} ", symbols.bullet);
        let text_width = inner_width.saturating_sub(bullet.chars().count());
        let note_text = truncate_text(note, text_width);
        let pad = " ".repeat(text_width.saturating_sub(note_text.chars().count()));
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} ", symbols.box_round.vertical),
                theme.border,
            ),
            Span::styled(bullet, theme.dim),
            Span::styled(note_text, theme.muted),
            Span::raw(pad),
            Span::styled(format!(" {}", symbols.box_round.vertical), theme.border),
        ]));
    }

    lines.push(Line::from(Span::styled(
        format!(
            "  {}{}{}",
            symbols.box_round.bottom_left,
            symbols.box_round.horizontal.repeat(panel_width.saturating_sub(2)),
            symbols.box_round.bottom_right
        ),
        theme.border,
    )));
    lines
}

fn render_processing_lines(
    pending: &[String],
    spinner_idx: usize,
    theme: &Theme,
    symbols: &Symbols,
    width: usize,
) -> Vec<Line<'static>> {
    let panel_width = width.saturating_sub(2).max(4);
    let inner_width = panel_width.saturating_sub(4);
    let spinner = symbols.spinner[spinner_idx % symbols.spinner.len()];
    let message_width = inner_width.saturating_sub(spinner.chars().count() + 1);
    let message = truncate_text("Responding... (esc to cancel)", message_width);
    let mut lines = vec![
        Line::from(Span::styled(
            format!(
                "  {}{}{}",
                symbols.box_sharp.top_left,
                symbols.box_sharp.horizontal.repeat(panel_width.saturating_sub(2)),
                symbols.box_sharp.top_right
            ),
            theme.border,
        )),
        padded_panel_line(
            vec![
                Span::styled(spinner.to_string(), theme.accent),
                Span::raw(" "),
                Span::styled(message, theme.muted),
            ],
            inner_width,
            theme,
            symbols,
        ),
    ];

    for item in pending {
        lines.push(padded_panel_line(
            vec![Span::styled(
                truncate_text(item, inner_width),
                theme.dim,
            )],
            inner_width,
            theme,
            symbols,
        ));
    }

    lines.push(Line::from(Span::styled(
        format!(
            "  {}{}{}",
            symbols.box_sharp.bottom_left,
            symbols.box_sharp.horizontal.repeat(panel_width.saturating_sub(2)),
            symbols.box_sharp.bottom_right
        ),
        theme.border,
    )));
    lines
}

fn padded_panel_line(
    mut content: Vec<Span<'static>>,
    inner_width: usize,
    theme: &Theme,
    symbols: &Symbols,
) -> Line<'static> {
    let content_width = spans_text_width(&content);
    let pad = " ".repeat(inner_width.saturating_sub(content_width));
    let mut spans = vec![Span::styled(
        format!("  {} ", symbols.box_sharp.vertical),
        theme.border,
    )];
    spans.append(&mut content);
    spans.push(Span::raw(pad));
    spans.push(Span::styled(
        format!(" {}", symbols.box_sharp.vertical),
        theme.border,
    ));
    Line::from(spans)
}

fn spans_text_width(spans: &[Span<'static>]) -> usize {
    spans
        .iter()
        .map(|span| span.content.chars().count())
        .sum()
}

fn render_status_notice_lines(
    messages: &[String],
    theme: &Theme,
    width: usize,
) -> Vec<Line<'static>> {
    let label = "[status]";
    let prefix_width = 2 + label.chars().count() + 1;
    let text_width = width.saturating_sub(prefix_width);
    messages
        .iter()
        .map(|msg| {
            Line::from(vec![
                Span::raw("  "),
                Span::styled(label.to_string(), theme.info),
                Span::raw(" "),
                Span::styled(truncate_text(msg, text_width), theme.muted),
            ])
        })
        .collect()
}

fn render_input(frame: &mut Frame, area: Rect, app: &App) {
    let theme = Theme::default();
    let editor = &app.editor;

    let border_style = match app.mode {
        AppMode::Normal => Style::new().fg(theme::MUTED_TEAL),
        AppMode::Input => {
            Style::new().fg(theme::AQUAMARINE)
        }
        AppMode::Processing => Style::new().fg(theme::CORAL),
    };

    let mut text_lines: Vec<Line<'static>> = Vec::new();

    if editor.is_empty() && editor.line_count() <= 1 {
        let placeholder = match app.mode {
            AppMode::Processing => "Waiting for AI response...",
            _ => "Type your message... (Shift+Enter for newline)",
        };
        text_lines.push(Line::from(Span::styled(placeholder, theme.dim)));
    } else {
        for row in 0..editor.line_count() {
            let line_text = editor.line(row);
            let mut spans = Vec::new();

            if row == editor.row() {
                // Current line — render with cursor
                let col = editor.col();
                let mut byte_pos = 0;
                for ch in line_text.chars() {
                    if byte_pos == col {
                        // Cursor position (before this char)
                        spans.push(Span::styled(
                            "▏",
                            Style::new().fg(theme::AQUAMARINE).add_modifier(ratatui::style::Modifier::BOLD),
                        ));
                    }
                    if byte_pos >= col && byte_pos < col + ch.len_utf8() {
                        // Character at cursor gets highlight
                        spans.push(Span::styled(
                            ch.to_string(),
                            Style::default().add_modifier(ratatui::style::Modifier::REVERSED),
                        ));
                    } else {
                        spans.push(Span::raw(ch.to_string()));
                    }
                    byte_pos += ch.len_utf8();
                }
                // Cursor at end of line
                if col >= line_text.len() {
                    spans.push(Span::styled(
                        "▏",
                        Style::new().fg(theme::AQUAMARINE).add_modifier(ratatui::style::Modifier::BOLD),
                    ));
                }
            } else {
                // Non-current line
                spans.push(Span::raw(line_text.to_string()));
            }

            text_lines.push(Line::from(spans));
        }
    }

    let line_count = text_lines.len().max(1);
    let height = (line_count as u16 + 2).min(area.height); // +2 for borders
    let para = Paragraph::new(text_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style)
                .padding(Padding::horizontal(2))
                .title(build_status_line(area.width.saturating_sub(2) as usize, app)),
        )
        .wrap(Wrap { trim: false })
        .scroll((
            (editor.row() as u16 + 1).saturating_sub(height.saturating_sub(2)),
            0,
        ));

    frame.render_widget(para, area);
}

/// Render autocomplete dropdown popup below the input area.
fn render_autocomplete(frame: &mut Frame, area: Rect, app: &App) {
    let ac = match app.autocomplete.as_ref() {
        Some(ac) if !ac.items.is_empty() => ac,
        _ => return,
    };

    let max_visible = 5.min(ac.items.len() as u16);
    let show_count = ac.items.len() > max_visible as usize;
    // Popup height = items + border + optional scroll count
    let popup_height = max_visible + 2 + u16::from(show_count);
    // Position: right below the input area, same width
    let popup = Rect {
        x: area.x,
        y: area.y + area.height,
        width: area.width,
        height: popup_height.min(frame.area().height.saturating_sub(area.y + area.height)),
    };

    if popup.height < 3 {
        return; // Not enough room
    }

    frame.render_widget(Clear, popup);

    let inner = popup.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(theme::AQUAMARINE))
        .title(Span::styled(
            " Commands ",
            Style::new().fg(theme::AQUAMARINE),
        ));
    frame.render_widget(block, popup);

    let total_items = ac.items.len();
    let items: Vec<ListItem> = ac
        .items
        .iter()
        .take(max_visible as usize)
        .enumerate()
        .map(|(i, item)| {
            let selected = i == ac.selected;
            let name_style = if selected {
                Style::new().fg(theme::AQUAMARINE).add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default()
            };
            let desc_style = Style::new().fg(theme::MUTED_TEAL);
            let cursor = if selected { "› " } else { "  " };
            let line = Line::from(vec![
                Span::styled(cursor, Style::new().fg(theme::AQUAMARINE)),
                Span::styled(truncate_text(&item.value, 18), name_style),
                Span::raw(" "),
                Span::styled(truncate_text(&item.description, popup.width.saturating_sub(24) as usize), desc_style),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(ac.selected));

    let list = List::new(items);

    frame.render_stateful_widget(list, inner, &mut list_state);

    if show_count {
        let count_line = Line::from(Span::styled(
            format!("  {}/{}", ac.selected + 1, total_items),
            Style::new().fg(theme::MUTED_TEAL),
        ));
        let count_area = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: 1,
        };
        frame.render_widget(Paragraph::new(count_line), count_area);
    }
}

fn build_status_line(width: usize, app: &App) -> Line<'static> {
    let theme = Theme::default();
    let s = app.symbols;
    let sep = Span::styled(format!(" {} ", s.sep_dot), theme.dim);
    let fill = s.hr_char;

    let preset = crate::status_line::resolve_preset(&app.status_preset);
    let mut left = render_status_group(&preset.left_segments, app, &theme, &sep);
    let mut right = render_status_group(&preset.right_segments, app, &theme, &sep);

    while status_width(&left) + status_width(&right) + gap_width(&left, &right) > width
        && !right.is_empty()
    {
        drop_last_status_segment(&mut right);
    }
    while status_width(&left) + status_width(&right) + gap_width(&left, &right) > width
        && !left.is_empty()
    {
        drop_last_status_segment(&mut left);
    }

    let left_width = status_width(&left);
    let right_width = status_width(&right);
    let mut spans = left;
    if !spans.is_empty() && !right.is_empty() {
        let fill_width = width.saturating_sub(left_width + right_width).max(1);
        spans.push(Span::styled(fill.repeat(fill_width), theme.border));
    }
    spans.extend(right);

    Line::from(spans)
}

fn render_status_group(
    segments: &[crate::status_line::StatusSegment],
    app: &App,
    theme: &Theme,
    sep: &Span<'static>,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for seg in segments {
        if let Some(sp) = render_status_segment(*seg, app, theme) {
            if !spans.is_empty() {
                spans.push(sep.clone());
            }
            spans.push(sp);
        }
    }
    spans
}

fn status_width(spans: &[Span<'static>]) -> usize {
    spans.iter().map(|sp| sp.content.chars().count()).sum()
}

fn gap_width(left: &[Span<'static>], right: &[Span<'static>]) -> usize {
    if left.is_empty() || right.is_empty() {
        0
    } else {
        1
    }
}

fn drop_last_status_segment(spans: &mut Vec<Span<'static>>) {
    spans.pop();
    if spans.last().is_some_and(|span| span.content.trim() == "·") {
        spans.pop();
    }
}

fn render_status_segment(
    seg: crate::status_line::StatusSegment,
    app: &App,
    theme: &Theme,
) -> Option<Span<'static>> {
    use crate::status_line::StatusSegment::*;
    match seg {
        Pi => Some(Span::styled("π", theme.accent)),
        Mode => {
            let (text, style) = match app.mode {
                AppMode::Normal => ("normal", theme.success),
                AppMode::Input => ("input", theme.accent),
                AppMode::Processing => ("busy", theme.warning),
            };
            Some(Span::styled(text, style))
        }
        Session => {
            let session = app
                .current_session_id
                .as_deref()
                .map(|id| id.chars().take(8).collect::<String>())
                .unwrap_or_else(|| "new".to_string());
            Some(Span::styled(session, theme.dim))
        }
        Model => Some(Span::styled(
            format!("{}/{}", app.provider, app.model),
            theme.info,
        )),
        Hostname => Some(Span::styled(app.hostname.clone(), theme.dim)),
        Git => app.git_branch.as_ref().map(|branch| {
            let mut text = branch.clone();
            if app.git_staged > 0 || app.git_unstaged > 0 || app.git_untracked > 0 {
                let mut parts = Vec::new();
                if app.git_staged > 0 {
                    parts.push(format!("+{}", app.git_staged));
                }
                if app.git_unstaged > 0 {
                    parts.push(format!("~{}", app.git_unstaged));
                }
                if app.git_untracked > 0 {
                    parts.push(format!("?{}", app.git_untracked));
                }
                text.push_str(&format!(" {}", parts.join(" ")));
            }
            let style = if app.git_staged > 0 || app.git_unstaged > 0 || app.git_untracked > 0 {
                theme.warning
            } else {
                theme.dim
            };
            Span::styled(text, style)
        }),
        Workspace => {
            let name = app
                .workspace
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("workspace");
            Some(Span::styled(format!("@{}", name), theme.dim))
        }
        Tokens => {
            let total_in = app.tokens_input + app.tokens_cache_read;
            if total_in > 0 || app.tokens_output > 0 {
                Some(Span::styled(
                    format!("{}k/{}k", total_in / 1000, app.tokens_output / 1000),
                    theme.dim,
                ))
            } else {
                None
            }
        }
        TokenRate => {
            let rate = app.token_rate();
            if rate > 0.0 {
                Some(Span::styled(format!("@{:.0}t/s", rate), theme.dim))
            } else {
                None
            }
        }
        ContextPct => {
            let total_in = app.tokens_input + app.tokens_cache_read;
            let total_tokens = total_in + app.tokens_output;
            if app.context_window > 0 {
                let pct = total_tokens as f64 / app.context_window as f64 * 100.0;
                let style = if pct >= 90.0 {
                    theme.error
                } else if pct >= 70.0 {
                    theme.warning
                } else {
                    theme.dim
                };
                return Some(Span::styled(
                    format!("{:.1}%/{}k", pct, app.context_window / 1000),
                    style,
                ));
            }
            None
        }
        Cost => {
            let cost = app.cost_estimate();
            if cost > 0.0 {
                Some(Span::styled(
                    format!("${:.2}", cost),
                    Style::new().fg(theme::DIFF_YELLOW),
                ))
            } else {
                None
            }
        }
        SessionTime => Some(Span::styled(app.session_elapsed(), theme.dim)),
        ThinkingLevel => {
            if app.thinking_level != crate::app::ThinkingLevel::Off {
                let label = match app.thinking_level {
                    crate::app::ThinkingLevel::Low => "think:low",
                    crate::app::ThinkingLevel::Medium => "think:med",
                    crate::app::ThinkingLevel::High => "think:high",
                    _ => return None,
                };
                Some(Span::styled(label, Style::new().fg(theme::AQUAMARINE)))
            } else {
                None
            }
        }
        Iterations => {
            if app.iterations_max > 0 {
                let style = if app.iterations_used >= app.iterations_max {
                    theme.error
                } else if app.iterations_used as f64 / app.iterations_max as f64 > 0.8 {
                    theme.warning
                } else {
                    theme.dim
                };
                Some(Span::styled(
                    format!("iter:{}/{}", app.iterations_used, app.iterations_max),
                    style,
                ))
            } else {
                None
            }
        }
    }
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let theme = Theme::default();
    let showing_welcome = app.show_welcome && app.messages.is_empty();

    let main_layout = Layout::vertical([
        Constraint::Length(if showing_welcome { 0 } else { 3 }),
        Constraint::Min(0),
        Constraint::Length(3),
    ]);
    let [header_area, content_area, input_area] = main_layout.areas(frame.area());

    if !showing_welcome {
        let title = format!(" Serana v{} ", app.version);
        let header = Paragraph::new(Line::from(Span::styled(&title, theme.muted)))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme.border)
                    .title(title.clone()),
            );
        frame.render_widget(header, header_area);
    }

    if showing_welcome {
        render_welcome(frame, content_area, app);
    } else {
        render_messages(frame, content_area, app);
    }

    render_input(frame, input_area, app);
    render_autocomplete(frame, input_area, app);

    // Render dialog on top if active
    if let Some(ref dialog) = app.active_dialog {
        crate::dialog::render_dialog(frame, dialog);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn todo(content: &str, status: TodoStatus) -> TodoItem {
        TodoItem {
            content: content.to_string(),
            status,
        }
    }

    #[test]
    fn test_todo_lines_show_incomplete_and_total_counts() {
        let theme = Theme::default();
        let lines = render_todo_lines(
            &[
                todo("one", TodoStatus::Pending),
                todo("two", TodoStatus::InProgress),
                todo("three", TodoStatus::Done),
            ],
            &theme,
        );
        assert!(lines[0].to_string().contains("2 incomplete todos / 3 total"));
    }

    #[test]
    fn test_todo_lines_use_reference_status_markers() {
        let theme = Theme::default();
        let lines = render_todo_lines(
            &[
                todo("pending", TodoStatus::Pending),
                todo("active", TodoStatus::InProgress),
                todo("done", TodoStatus::Done),
                todo("dropped", TodoStatus::Abandoned),
            ],
            &theme,
        );
        let rendered: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
        assert!(rendered.iter().any(|line| line.contains("- [ ] pending")));
        assert!(rendered.iter().any(|line| line.contains("- [/] active")));
        assert!(rendered.iter().any(|line| line.contains("- [x] done")));
        assert!(rendered.iter().any(|line| line.contains("- [-] dropped")));
    }

    #[test]
    fn test_btw_lines_render_rounded_panel() {
        let theme = Theme::default();
        let notes = vec!["remember this".to_string()];
        let lines = render_btw_lines(&notes, &theme, &crate::symbols::UNICODE, 40);
        let rendered: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
        assert!(rendered[0].contains("╭"));
        assert!(rendered[0].contains("By the way"));
        assert!(rendered[1].contains("• remember this"));
        assert!(rendered[2].contains("╰"));
    }

    #[test]
    fn test_btw_lines_fit_requested_width() {
        let theme = Theme::default();
        let notes = vec!["a very long note that should be clipped".to_string()];
        let lines = render_btw_lines(&notes, &theme, &crate::symbols::UNICODE, 24);
        assert!(lines
            .iter()
            .all(|line| line.to_string().chars().count() <= 24));
    }

    #[test]
    fn test_btw_lines_fit_tiny_width() {
        let theme = Theme::default();
        let notes = vec!["tiny".to_string()];
        let lines = render_btw_lines(&notes, &theme, &crate::symbols::UNICODE, 8);
        assert!(lines
            .iter()
            .all(|line| line.to_string().chars().count() <= 8));
    }

    #[test]
    fn test_processing_lines_render_bordered_loader() {
        let theme = Theme::default();
        let pending = vec!["tool call pending".to_string()];
        let lines = render_processing_lines(&pending, 0, &theme, &crate::symbols::UNICODE, 42);
        let rendered: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
        assert!(rendered[0].contains("┌"));
        assert!(rendered[1].contains("Responding"));
        assert!(rendered[2].contains("tool call pending"));
        assert!(rendered[3].contains("└"));
    }

    #[test]
    fn test_processing_lines_fit_requested_width() {
        let theme = Theme::default();
        let pending = vec!["a very long pending status message".to_string()];
        let lines = render_processing_lines(&pending, 0, &theme, &crate::symbols::UNICODE, 18);
        assert!(lines
            .iter()
            .all(|line| line.to_string().chars().count() <= 18));
    }

    #[test]
    fn test_status_notice_lines_use_label() {
        let theme = Theme::default();
        let messages = vec!["Switched to model: gpt-4.1".to_string()];
        let lines = render_status_notice_lines(&messages, &theme, 80);
        let text = lines[0].to_string();
        assert!(text.contains("[status]"));
        assert!(text.contains("Switched to model"));
    }

    #[test]
    fn test_status_notice_lines_fit_requested_width() {
        let theme = Theme::default();
        let messages = vec!["a very long status notice that should be clipped".to_string()];
        let lines = render_status_notice_lines(&messages, &theme, 18);
        assert!(lines
            .iter()
            .all(|line| line.to_string().chars().count() <= 18));
    }
}
