use display_protocol::{InputEvent, KeyCode};
use display_protocol_widgets::{
    AnimationSpec, ChatComposerSpec, FooterSurfaceSpec, HistoryCellKindSpec, HistoryCellSpec,
    PendingInputPreviewSpec, PlanCellKindSpec, PlanCellSpec, StatusIndicatorSpec,
    StatusSurfaceKindSpec, StatusSurfaceSpec, TokenUsageSpec, TranscriptSpec, UnifiedExecSpec,
    WidgetSpec,
};
use display_tui::{render_widget_spec_to_lines, trim_rendered_lines, TuiTerminal};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use std::{io, time::Duration};

fn main() -> io::Result<()> {
    let mut terminal = TuiTerminal::new()?;
    display_tui::install_panic_hook();
    terminal.init()?;

    let result = run(&mut terminal);
    terminal.cleanup()?;
    result
}

fn run(terminal: &mut TuiTerminal) -> io::Result<()> {
    let mut frame = 0usize;

    loop {
        terminal.draw(|buf, area| render_codex_ui(buf, area, frame))?;
        frame = frame.wrapping_add(1);

        if let Some(event) = terminal.poll_input(Duration::from_millis(120).as_millis() as u64) {
            match event {
                InputEvent::Key(key) => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => break,
                    _ => {}
                },
                InputEvent::Resize { .. } => {}
                _ => {}
            }
        }
    }

    Ok(())
}

fn render_codex_ui(buf: &mut Buffer, area: Rect, frame: usize) {
    let root = Block::default()
        .title(" Codex UI protocol example ")
        .title_bottom(" q/Esc quit ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));
    let inner = root.inner(area);
    root.render(area, buf);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(8),
        ])
        .split(inner);
    render_status_strip(buf, rows[0], frame);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(66), Constraint::Percentage(34)])
        .split(rows[1]);

    render_spec_panel(buf, columns[0], "Transcript", transcript(frame).into());
    render_sidebar(buf, columns[1], frame);
    render_spec_panel(buf, rows[2], "Composer", composer(frame).into());
}

fn render_status_strip(buf: &mut Buffer, area: Rect, frame: usize) {
    let mut parts = Vec::new();
    parts.extend(render_widget_spec_to_lines(
        &WidgetSpec::from(status_indicator(frame)),
        30,
    ));
    parts.extend(render_widget_spec_to_lines(
        &WidgetSpec::from(AnimationSpec {
            frame,
            ..AnimationSpec::new(
                "Thinking",
                vec![
                    "⠋".to_string(),
                    "⠙".to_string(),
                    "⠹".to_string(),
                    "⠸".to_string(),
                    "⠼".to_string(),
                    "⠴".to_string(),
                ],
            )
        }),
        20,
    ));
    parts.extend(render_widget_spec_to_lines(
        &WidgetSpec::from(token_usage(frame)),
        area.width.saturating_sub(4),
    ));
    let line = parts
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join("   ");

    Paragraph::new(Line::from(line))
        .block(
            Block::default()
                .title("Status")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .render(area, buf);
}

fn render_sidebar(buf: &mut Buffer, area: Rect, _frame: usize) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(5)])
        .split(area);

    render_spec_panel(buf, chunks[0], "Runtime", runtime_surface().into());
    render_spec_panel(buf, chunks[1], "Plan", plan().into());
}

fn render_spec_panel(buf: &mut Buffer, area: Rect, title: &'static str, spec: WidgetSpec) {
    let width = area.width.saturating_sub(2).max(1);
    let mut lines = render_widget_spec_to_lines(&spec, width);
    trim_rendered_lines(&mut lines);
    let lines = lines.into_iter().map(Line::from).collect::<Vec<_>>();

    Paragraph::new(lines)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .wrap(Wrap { trim: false })
        .render(area, buf);
}

fn transcript(frame: usize) -> TranscriptSpec {
    TranscriptSpec {
        title: "Conversation".to_string(),
        height: 16,
        ..TranscriptSpec::new(vec![
            HistoryCellSpec::new(
                HistoryCellKindSpec::User,
                vec!["build a protocol-backed Codex-style TUI".to_string()],
            ),
            HistoryCellSpec::new(
                HistoryCellKindSpec::Agent,
                vec![
                    "Using shared surface specs for transcript, status, plan, exec, and composer."
                        .to_string(),
                ],
            ),
            HistoryCellSpec {
                title: Some("Bash".to_string()),
                children: vec![UnifiedExecSpec {
                    stdout: vec![
                        "Checking display-protocol-widgets".to_string(),
                        "Checking display-tui".to_string(),
                    ],
                    ..UnifiedExecSpec::new("cargo check -p display-tui", "running")
                }
                .into()],
                ..HistoryCellSpec::new(HistoryCellKindSpec::Exec, Vec::new())
            },
            HistoryCellSpec {
                title: Some("Assistant".to_string()),
                children: vec![AnimationSpec {
                    frame,
                    ..AnimationSpec::new(
                        "Thinking",
                        vec![
                            "⠋".to_string(),
                            "⠙".to_string(),
                            "⠹".to_string(),
                            "⠸".to_string(),
                            "⠼".to_string(),
                            "⠴".to_string(),
                        ],
                    )
                }
                .into()],
                ..HistoryCellSpec::new(HistoryCellKindSpec::StreamingTail, Vec::new())
            },
        ])
    }
}

fn status_indicator(frame: usize) -> StatusIndicatorSpec {
    let frames = ["thinking", "thinking.", "thinking..", "thinking..."];
    StatusIndicatorSpec {
        active: true,
        details: Some("1 tool running".to_string()),
        ..StatusIndicatorSpec::new("Agent", frames[frame % frames.len()])
    }
}

fn runtime_surface() -> StatusSurfaceSpec {
    StatusSurfaceSpec {
        rows: vec![
            ("model".to_string(), "gpt-5".to_string()),
            ("approval".to_string(), "on-request".to_string()),
            ("cwd".to_string(), "/data/Workspace/serana-new".to_string()),
        ],
        ..StatusSurfaceSpec::new(StatusSurfaceKindSpec::Runtime, "Runtime")
    }
}

fn token_usage(frame: usize) -> TokenUsageSpec {
    TokenUsageSpec {
        limit: Some(200_000),
        percent: Some((42 + (frame % 5)) as u8),
        reset: Some("18:00".to_string()),
        ..TokenUsageSpec::new(84_000 + frame as u64 * 12)
    }
}

fn plan() -> PlanCellSpec {
    PlanCellSpec {
        active: Some(1),
        ..PlanCellSpec::new(
            PlanCellKindSpec::Update,
            vec![
                "Define protocol surfaces".to_string(),
                "Render through display-tui".to_string(),
                "Wire app-specific behavior later".to_string(),
            ],
        )
    }
}

fn composer(frame: usize) -> ChatComposerSpec {
    let mut pending = PendingInputPreviewSpec::new(vec![
        "run cargo test".to_string(),
        "commit feature".to_string(),
    ]);
    pending.interrupt_hint = Some("Esc interrupts current turn".to_string());

    ChatComposerSpec {
        lines: vec!["add animation and a Codex UI example".to_string()],
        cursor_col: 13 + frame % 3,
        attachments: vec!["crates/display-tui/examples/codex_ui.rs".to_string()],
        pending: if frame % 8 < 4 { Some(pending) } else { None },
        footer: FooterSurfaceSpec {
            left: vec!["gpt-5".to_string(), "workspace-write".to_string()],
            right: vec!["Enter send".to_string(), "Shift+Enter newline".to_string()],
            mode: Some("chat".to_string()),
            goal: Some("active".to_string()),
        },
        ..ChatComposerSpec::new(Vec::new())
    }
}
