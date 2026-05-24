use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, AppMode, ChatMessage, MessageRole, TodoStatus};
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

fn render_message_para(msg: &ChatMessage, width: usize, theme: &Theme, symbols: &Symbols) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let h = symbols.box_sharp.horizontal;

    match msg.role {
        MessageRole::User => {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {} You", symbols.success),
                theme.success,
            )));
            for text_line in msg.content.lines() {
                lines.push(Line::from(Span::raw(format!("    {}", text_line))));
            }
        }
        MessageRole::Agent => {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {} Serana", symbols.cursor),
                theme.accent,
            )));

            if let Some(ref thinking) = msg.thinking {
                lines.push(Line::from(Span::styled(
                    format!("  {}{} thinking {}", symbols.box_sharp.top_left, h, h),
                    theme.thinking,
                )));
                for t_line in thinking.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {} {}", symbols.box_sharp.vertical, t_line),
                        theme.thinking,
                    )));
                }
                lines.push(Line::from(Span::styled(
                    format!("  {}{}", symbols.box_sharp.bottom_left, h),
                    theme.thinking,
                )));
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
        all_lines.push(Line::from(Span::styled(
            format!(
                "  {} Responding...",
                s.spinner[(app.tick_count % s.spinner.len() as u64) as usize]
            ),
            theme.accent,
        )));
        for pending in &app.pending_messages {
            all_lines.push(Line::from(Span::styled(
                format!("    {}", pending),
                theme.dim,
            )));
        }
    }

    if !app.status_messages.is_empty() {
        all_lines.push(Line::from(""));
        for msg in &app.status_messages {
            all_lines.push(Line::from(Span::styled(
                format!("  {} {}", s.arrow, msg),
                theme.info,
            )));
        }
    }

    if !app.todo_items.is_empty() {
        all_lines.push(Line::from(""));
        all_lines.push(Line::from(Span::styled("  Tasks", theme.accent)));
        for (i, todo) in app.todo_items.iter().enumerate() {
            let (check, style) = match todo.status {
                TodoStatus::Done => (s.checkbox_checked, theme.dim),
                TodoStatus::Abandoned => (s.checkbox_abandoned, Style::default().add_modifier(ratatui::style::Modifier::DIM | ratatui::style::Modifier::CROSSED_OUT)),
                TodoStatus::InProgress => (s.checkbox_in_progress, Style::default().fg(theme::AQUAMARINE).add_modifier(ratatui::style::Modifier::BOLD)),
                TodoStatus::Pending => (s.checkbox_unchecked, Style::default()),
            };
            all_lines.push(Line::from(Span::styled(
                format!("  {:>2}. {} {}", i + 1, check, todo.content),
                style,
            )));
        }
    }

    if !app.btw_notes.is_empty() {
        all_lines.push(Line::from(""));
        let h = s.box_sharp.horizontal;
        all_lines.push(Line::from(Span::styled(
            format!("  {}{} By the way {}{}", s.box_sharp.top_left, h, h, s.box_sharp.top_right),
            theme.warning,
        )));
        for note in &app.btw_notes {
            all_lines.push(Line::from(vec![
                Span::styled(format!("  {} ", s.box_sharp.vertical), theme.warning),
                Span::styled(format!("{} {}", s.bullet, note), theme.muted),
            ]));
        }
        all_lines.push(Line::from(Span::styled(
            format!("  {}{}", s.box_sharp.bottom_left, h.repeat(2)),
            theme.warning,
        )));
    }

    all_lines.push(Line::from(""));

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
                .border_style(border_style)
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
    // Popup height = items + 2 for border
    let popup_height = max_visible + 2;
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
            let line = Line::from(vec![
                Span::styled(format!(" {:<12}", item.value), name_style),
                Span::styled(&item.description, desc_style),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(ac.selected));

    let list = List::new(items)
        .highlight_style(
            Style::new().fg(theme::AQUAMARINE).add_modifier(ratatui::style::Modifier::BOLD),
        );

    frame.render_stateful_widget(list, inner, &mut list_state);
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
