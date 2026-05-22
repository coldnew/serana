use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, AppMode, ChatMessage, MessageRole, ToolCallStatus};
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
        ratatui::style::Color::Rgb(255, 139, 109),
        ratatui::style::Color::Rgb(255, 170, 130),
        ratatui::style::Color::Rgb(200, 200, 160),
        ratatui::style::Color::Rgb(130, 210, 200),
        ratatui::style::Color::Rgb(91, 192, 190),
        ratatui::style::Color::Rgb(111, 255, 233),
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

    let cols = Layout::horizontal([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)]);
    let areas = cols.split(area);
    let left_area = areas[0];
    let right_area = areas[1];

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

    if right_area.width < 20 {
        return;
    }

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
    right_lines.push(Line::from(Span::styled(
        format!("  {} No recent sessions", s.bullet),
        theme.dim,
    )));

    let right_para = Paragraph::new(right_lines);
    frame.render_widget(right_para, right_area);
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
                lines.push(Line::from(""));
                let icon = match tool.status {
                    ToolCallStatus::Pending => symbols.pending,
                    ToolCallStatus::Running => symbols.running,
                    ToolCallStatus::Success => symbols.success,
                    ToolCallStatus::Error => symbols.error,
                };
                let tool_style = match tool.status {
                    ToolCallStatus::Pending => theme.dim,
                    ToolCallStatus::Running => theme.accent,
                    ToolCallStatus::Success => theme.success,
                    ToolCallStatus::Error => theme.error,
                };
                lines.push(Line::from(Span::styled(
                    format!("  {} {}", icon, tool.name),
                    tool_style,
                )));
                if let Some(ref result) = tool.result {
                    lines.push(Line::from(Span::styled(
                        format!("    {}", result.lines().next().unwrap_or("")),
                        theme.muted,
                    )));
                }
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
        for todo in &app.todo_items {
            let check = if todo.done { s.checkbox_checked } else { s.checkbox_unchecked };
            let style = if todo.done { theme.dim } else { Style::default() };
            all_lines.push(Line::from(Span::styled(
                format!("    {} {}", check, todo.content),
                style,
            )));
        }
    }

    if !app.btw_notes.is_empty() {
        all_lines.push(Line::from(""));
        all_lines.push(Line::from(Span::styled("  By the way", theme.warning)));
        for note in &app.btw_notes {
            all_lines.push(Line::from(Span::styled(
                format!("    {} {}", s.bullet, note),
                theme.dim,
            )));
        }
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

    let (border_style, title) = match app.mode {
        AppMode::Normal => (Style::new().fg(theme::MUTED_TEAL), " Input "),
        AppMode::Input => (Style::new().fg(theme::AQUAMARINE), " Input "),
        AppMode::Processing => (Style::new().fg(theme::CORAL), " Working "),
    };

    let content = if app.input.is_empty() {
        match app.mode {
            AppMode::Processing => {
                Line::from(Span::styled("Waiting for AI response...", theme.dim))
            }
            _ => Line::from(Span::styled("Type your message...", theme.dim)),
        }
    } else {
        let mut spans = Vec::new();
        for (i, ch) in app.input.char_indices() {
            if i == app.cursor.min(app.input.len()) {
                spans.push(Span::styled(
                    ch.to_string(),
                    Style::default().add_modifier(ratatui::style::Modifier::REVERSED),
                ));
            } else {
                spans.push(Span::raw(ch.to_string()));
            }
        }
        if app.cursor >= app.input.len() {
            spans.push(Span::styled(
                " ",
                Style::default().add_modifier(ratatui::style::Modifier::REVERSED),
            ));
        }
        Line::from(spans)
    };

    let para = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(para, area);
}

fn render_status_line(frame: &mut Frame, area: Rect, app: &App) {
    let theme = Theme::default();
    let s = app.symbols;

    let mut left_segments = Vec::new();

    let (mode_text, mode_style) = match app.mode {
        AppMode::Normal => ("NORMAL", Style::new().fg(theme::SEAFOAM_GREEN)),
        AppMode::Input => ("INPUT", Style::new().fg(theme::AQUAMARINE)),
        AppMode::Processing => {
            ("BUSY", Style::new().fg(theme::CORAL).add_modifier(ratatui::style::Modifier::BOLD))
        }
    };
    left_segments.push(Span::styled(mode_text.to_string(), mode_style));
    let sep = format!(" {} ", s.sep_thin);
    left_segments.push(Span::styled(sep.clone(), theme.dim));

    left_segments.push(Span::styled(app.model.clone(), theme.info));
    left_segments.push(Span::styled(sep.clone(), theme.dim));

    if let Some(ref branch) = app.git_branch {
        left_segments.push(Span::styled(
            format!("git:{}", branch),
            theme.dim,
        ));
        if app.git_staged > 0 || app.git_unstaged > 0 || app.git_untracked > 0 {
            let mut parts = Vec::new();
            if app.git_staged > 0 {
                parts.push(format!("+{}", app.git_staged));
            }
            if app.git_unstaged > 0 {
                parts.push(format!("!{}", app.git_unstaged));
            }
            if app.git_untracked > 0 {
                parts.push(format!("?{}", app.git_untracked));
            }
            left_segments.push(Span::styled(
                format!(" ({})", parts.join(" ")),
                theme.warning,
            ));
        }
        left_segments.push(Span::styled(sep.clone(), theme.dim));
    }

    let workspace_name = app
        .workspace
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("workspace");
    left_segments.push(Span::styled(workspace_name.to_string(), theme.dim));

    if app.tokens_input > 0 || app.tokens_output > 0 {
        let total_input = app.tokens_input + app.tokens_cache_read;
        if total_input > 0 || app.tokens_output > 0 {
            left_segments.push(Span::styled(sep.clone(), theme.dim));
            let usage = format!("in:{} out:{}", total_input, app.tokens_output);
            left_segments.push(Span::styled(usage, theme.dim));
        }
        if app.context_window > 0 {
            let total_tokens = total_input + app.tokens_output;
            let pct = if app.context_window > 0 {
                (total_tokens as f64 / app.context_window as f64 * 100.0) as u32
            } else {
                0
            };
            if pct > 0 {
                left_segments.push(Span::styled(sep.clone(), theme.dim));
                let ctx_str = format!("ctx:{}%", pct);
                let ctx_style = if pct >= 90 {
                    theme.error
                } else if pct >= 70 {
                    theme.warning
                } else if pct >= 50 {
                    Style::new().fg(theme::TEAL)
                } else {
                    theme.dim
                };
                left_segments.push(Span::styled(ctx_str, ctx_style));
            }
        }
    }

    if app.iterations_max > 0 {
        left_segments.push(Span::styled(sep.clone(), theme.dim));
        let iter_text = format!("iter:{}/{}", app.iterations_used, app.iterations_max);
        let iter_style = if app.iterations_used >= app.iterations_max {
            theme.error
        } else if app.iterations_used as f64 / app.iterations_max as f64 > 0.8 {
            theme.warning
        } else {
            theme.dim
        };
        left_segments.push(Span::styled(iter_text, iter_style));
    }

    let right_text = Span::styled(" Ctrl+Q: quit", theme.dim);

    left_segments.push(Span::raw(" "));
    left_segments.push(right_text);

    let line = Line::from(left_segments);
    let para = Paragraph::new(line);
    frame.render_widget(para, area);
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let theme = Theme::default();

    let main_layout = Layout::vertical([
        Constraint::Length(if app.show_welcome && app.messages.is_empty() {
            frame.area().height.min(25)
        } else {
            3
        }),
        Constraint::Min(0),
        Constraint::Length(3),
        Constraint::Length(1),
    ]);
    let [header_area, content_area, input_area, status_area] = main_layout.areas(frame.area());

    let title = format!(" Serana v{} ", app.version);
    let header = Paragraph::new(Line::from(Span::styled(&title, theme.muted)))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border)
                .title(title.clone()),
        );
    frame.render_widget(header, header_area);

    if app.show_welcome && app.messages.is_empty() {
        let cols = Layout::horizontal([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)]);
        let _welcome_areas = cols.split(content_area);
        render_welcome(frame, content_area, app);
    } else {
        render_messages(frame, content_area, app);
    }

    render_input(frame, input_area, app);
    render_status_line(frame, status_area, app);
}
