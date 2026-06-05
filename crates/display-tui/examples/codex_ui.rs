use display_protocol::{InputEvent, KeyCode};
use display_protocol_widgets::{
    AnimationSpec, ApprovalKindSpec, ApprovalOverlaySpec, ChatComposerSpec, FooterSurfaceSpec,
    HistoryCellKindSpec, HistoryCellSpec, PatchCellSpec, PendingInputPreviewSpec, PlanCellKindSpec,
    PlanCellSpec, StatusIndicatorSpec, StatusSurfaceKindSpec, StatusSurfaceSpec, TokenUsageSpec,
    TranscriptSpec, UnifiedExecSpec, WidgetSpec,
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

const STEP_COUNT: usize = 6;

fn main() -> io::Result<()> {
    let mut terminal = TuiTerminal::new()?;
    display_tui::install_panic_hook();
    terminal.init()?;

    let result = run(&mut terminal);
    terminal.cleanup()?;
    result
}

fn run(terminal: &mut TuiTerminal) -> io::Result<()> {
    let mut state = DemoState::default();

    loop {
        terminal.draw(|buf, area| render_codex_ui(buf, area, &state))?;
        state.frame = state.frame.wrapping_add(1);

        if let Some(event) = terminal.poll_input(Duration::from_millis(120).as_millis() as u64) {
            match event {
                InputEvent::Key(key) => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => break,
                    KeyCode::Enter | KeyCode::Right | KeyCode::Char('n') => state.next(),
                    KeyCode::Left | KeyCode::Char('p') => state.previous(),
                    _ => {}
                },
                InputEvent::Resize { .. } => {}
                _ => {}
            }
        }
    }

    Ok(())
}

#[derive(Default)]
struct DemoState {
    frame: usize,
    step: usize,
}

impl DemoState {
    fn next(&mut self) {
        self.step = (self.step + 1).min(STEP_COUNT - 1);
    }

    fn previous(&mut self) {
        self.step = self.step.saturating_sub(1);
    }
}

fn render_codex_ui(buf: &mut Buffer, area: Rect, state: &DemoState) {
    let root = Block::default()
        .title(" Fake Codex offline demo ")
        .title_bottom(" Enter/n next · p previous · q/Esc quit · scripted local demo, no provider ")
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
    render_status_strip(buf, rows[0], state);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(64), Constraint::Percentage(36)])
        .split(rows[1]);

    render_spec_panel(buf, columns[0], "Transcript", transcript(state).into());
    render_sidebar(buf, columns[1], state);
    render_spec_panel(buf, rows[2], "Composer", composer(state).into());
}

fn render_status_strip(buf: &mut Buffer, area: Rect, state: &DemoState) {
    let mut parts = Vec::new();
    parts.extend(render_widget_spec_to_lines(
        &WidgetSpec::from(status_indicator(state)),
        28,
    ));
    parts.extend(render_widget_spec_to_lines(
        &WidgetSpec::from(animation(state.frame)),
        18,
    ));
    parts.extend(render_widget_spec_to_lines(
        &WidgetSpec::from(token_usage(state)),
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
                .title(format!(
                    "Step {}/{} · {}",
                    state.step + 1,
                    STEP_COUNT,
                    step_title(state.step)
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .render(area, buf);
}

fn render_sidebar(buf: &mut Buffer, area: Rect, state: &DemoState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(8),
            Constraint::Length(5),
        ])
        .split(area);

    render_spec_panel(buf, chunks[0], "Runtime", runtime_surface().into());
    render_spec_panel(buf, chunks[1], "Action", action_surface(state));
    render_spec_panel(buf, chunks[2], "Plan", plan(state).into());
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

fn transcript(state: &DemoState) -> TranscriptSpec {
    let mut cells = vec![HistoryCellSpec::new(
        HistoryCellKindSpec::User,
        vec!["Add a /login command and persist selected model in config.".to_string()],
    )];

    if state.step >= 1 {
        cells.push(HistoryCellSpec::new(
            HistoryCellKindSpec::Agent,
            vec![
                "I will inspect the TUI command flow, add a focused change, and verify it locally."
                    .to_string(),
            ],
        ));
    }
    if state.step == 1 {
        cells.push(streaming_cell(state.frame, "Planning the local demo steps"));
    }
    if state.step >= 2 {
        cells.push(HistoryCellSpec::new(
            HistoryCellKindSpec::Approval,
            vec!["Requested approval to run: cargo test -p serana".to_string()],
        ));
    }
    if state.step >= 3 {
        cells.push(HistoryCellSpec {
            title: Some("Bash".to_string()),
            children: vec![UnifiedExecSpec {
                stdout: vec![
                    "Checking serana".to_string(),
                    "test tui::slash_commands::login ... ok".to_string(),
                ],
                exit_code: if state.step >= 4 { Some(0) } else { None },
                ..UnifiedExecSpec::new(
                    "cargo test -p serana login",
                    if state.step >= 4 {
                        "success"
                    } else {
                        "running"
                    },
                )
            }
            .into()],
            ..HistoryCellSpec::new(HistoryCellKindSpec::Exec, Vec::new())
        });
    }
    if state.step >= 4 {
        cells.push(HistoryCellSpec {
            title: Some("Patch".to_string()),
            children: vec![PatchCellSpec {
                added: 42,
                removed: 7,
                status: "applied".to_string(),
                ..PatchCellSpec::new(vec![
                    "serana/src/tui/slash_commands.rs".to_string(),
                    "serana/src/core/config.rs".to_string(),
                ])
            }
            .into()],
            ..HistoryCellSpec::new(HistoryCellKindSpec::Patch, Vec::new())
        });
    }
    if state.step >= 5 {
        cells.push(HistoryCellSpec::new(
            HistoryCellKindSpec::Agent,
            vec![
                "Done. /login is wired, model selection persists, and the targeted tests pass."
                    .to_string(),
            ],
        ));
    }

    TranscriptSpec {
        title: "Offline transcript".to_string(),
        height: 16,
        ..TranscriptSpec::new(cells)
    }
}

fn streaming_cell(frame: usize, label: &str) -> HistoryCellSpec {
    HistoryCellSpec {
        title: Some("Assistant".to_string()),
        children: vec![animation_with_label(frame, label).into()],
        ..HistoryCellSpec::new(HistoryCellKindSpec::StreamingTail, Vec::new())
    }
}

fn status_indicator(state: &DemoState) -> StatusIndicatorSpec {
    let state_label = match state.step {
        0 => "idle",
        1 => "planning",
        2 => "approval",
        3 => "running tool",
        4 => "applying patch",
        _ => "complete",
    };
    StatusIndicatorSpec {
        active: state.step < STEP_COUNT - 1,
        details: Some("offline scripted run".to_string()),
        ..StatusIndicatorSpec::new("Fake Codex", state_label)
    }
}

fn runtime_surface() -> StatusSurfaceSpec {
    StatusSurfaceSpec {
        rows: vec![
            ("model".to_string(), "fake-gpt-5/offline".to_string()),
            ("provider".to_string(), "none".to_string()),
            ("network".to_string(), "disabled".to_string()),
            ("approval".to_string(), "simulated".to_string()),
        ],
        ..StatusSurfaceSpec::new(StatusSurfaceKindSpec::Runtime, "Runtime")
    }
}

fn action_surface(state: &DemoState) -> WidgetSpec {
    match state.step {
        0 => StatusSurfaceSpec {
            rows: vec![
                ("mode".to_string(), "waiting for prompt".to_string()),
                ("hint".to_string(), "press Enter".to_string()),
            ],
            ..StatusSurfaceSpec::new(StatusSurfaceKindSpec::Runtime, "Ready")
        }
        .into(),
        1 => animation_with_label(state.frame, "Thinking through local changes").into(),
        2 => ApprovalOverlaySpec {
            command: Some("cargo test -p serana login".to_string()),
            choices: vec!["Approve".to_string(), "Deny".to_string()],
            selected: 0,
            ..ApprovalOverlaySpec::new(
                ApprovalKindSpec::Exec,
                "Approve fake tool run?",
                "This demo does not execute the command from the UI; it only shows the approval surface.",
            )
        }
        .into(),
        3 => UnifiedExecSpec {
            stdout: vec!["running targeted tests".to_string()],
            ..UnifiedExecSpec::new("cargo test -p serana login", "running")
        }
        .into(),
        4 => PatchCellSpec {
            added: 42,
            removed: 7,
            status: "preview".to_string(),
            ..PatchCellSpec::new(vec![
                "serana/src/tui/slash_commands.rs".to_string(),
                "serana/src/core/config.rs".to_string(),
            ])
        }
        .into(),
        _ => StatusSurfaceSpec {
            rows: vec![
                ("tests".to_string(), "passed".to_string()),
                ("commit".to_string(), "ready".to_string()),
            ],
            ..StatusSurfaceSpec::new(StatusSurfaceKindSpec::Goal, "Complete")
        }
        .into(),
    }
}

fn token_usage(state: &DemoState) -> TokenUsageSpec {
    TokenUsageSpec {
        limit: Some(16_000),
        percent: Some((8 + state.step as u8 * 12).min(100)),
        reset: Some("offline".to_string()),
        ..TokenUsageSpec::new(1_200 + state.step as u64 * 1_400 + state.frame as u64)
    }
}

fn plan(state: &DemoState) -> PlanCellSpec {
    PlanCellSpec {
        active: Some(state.step.min(4)),
        ..PlanCellSpec::new(
            PlanCellKindSpec::Update,
            vec![
                "Read command/config flow".to_string(),
                "Ask approval for test run".to_string(),
                "Show fake tool output".to_string(),
                "Preview patch summary".to_string(),
                "Report result".to_string(),
            ],
        )
    }
}

fn composer(state: &DemoState) -> ChatComposerSpec {
    let mut pending = PendingInputPreviewSpec::new(vec![
        "run tests after approval".to_string(),
        "summarize generated patch".to_string(),
    ]);
    pending.interrupt_hint = Some("Esc/q exits demo".to_string());

    ChatComposerSpec {
        lines: vec![match state.step {
            0 => "Add /login and make model selection persist".to_string(),
            1 => "Planning offline response...".to_string(),
            2 => "Approve simulated test command".to_string(),
            3 => "Reading fake command output".to_string(),
            4 => "Reviewing patch summary".to_string(),
            _ => "Done - this was a local scripted demo".to_string(),
        }],
        cursor_col: 8 + state.frame % 4,
        attachments: vec!["serana/src/tui/slash_commands.rs".to_string()],
        pending: if (1..5).contains(&state.step) {
            Some(pending)
        } else {
            None
        },
        footer: FooterSurfaceSpec {
            left: vec![
                "fake-gpt-5".to_string(),
                "offline".to_string(),
                step_title(state.step).to_string(),
            ],
            right: vec!["Enter/n next".to_string(), "p previous".to_string()],
            mode: Some("chat".to_string()),
            goal: Some("demo".to_string()),
        },
        ..ChatComposerSpec::new(Vec::new())
    }
}

fn animation(frame: usize) -> AnimationSpec {
    animation_with_label(frame, "Thinking")
}

fn animation_with_label(frame: usize, label: &str) -> AnimationSpec {
    AnimationSpec {
        frame,
        ..AnimationSpec::new(
            label,
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
}

fn step_title(step: usize) -> &'static str {
    match step {
        0 => "Prompt",
        1 => "Plan",
        2 => "Approve",
        3 => "Tool",
        4 => "Patch",
        _ => "Done",
    }
}
