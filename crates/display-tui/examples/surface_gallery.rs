use display_protocol::{InputEvent, KeyCode};
use display_protocol_widgets::{
    AnimationSpec, AppLinkSuggestionTypeSpec, AppLinkViewSpec, ApprovalKindSpec,
    ApprovalOverlaySpec, ChatComposerSpec, FeedbackViewSpec, FooterSurfaceSpec, FormFieldSpec,
    HistoryCellKindSpec, HistoryCellSpec, HookCellSpec, IntegrationViewKindSpec,
    IntegrationViewSpec, MarkdownStreamSpec, McpElicitationOverlaySpec, McpToolCallSpec,
    MenuSurfaceSpec, MultiSelectPickerSpec, NavigationOverlaySpec, OverlayKindSpec, PatchCellSpec,
    PendingInputPreviewSpec, PendingThreadApprovalsSpec, PlanCellKindSpec, PlanCellSpec,
    RequestUserInputOverlaySpec, SelectionPopupKindSpec, SelectionPopupSpec, SelectionRowSpec,
    SessionInfoCellSpec, SessionPickerSpec, SetupScreenKindSpec, SetupScreenSpec,
    StatusIndicatorSpec, StatusSurfaceKindSpec, StatusSurfaceSpec, TerminalHyperlinkSpec,
    TextAreaSpec, TokenUsageSpec, TranscriptSpec, UnifiedExecSpec, VoiceMeterSpec,
    WebSearchCellSpec, WidgetSpec,
};
use display_tui::{render_widget_spec_to_lines, trim_rendered_lines, TuiTerminal};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use std::{io, time::Duration};

fn main() -> io::Result<()> {
    let mut terminal = TuiTerminal::new()?;
    display_tui::install_panic_hook();
    terminal.init()?;

    let result = run_gallery(&mut terminal);
    terminal.cleanup()?;
    result
}

fn run_gallery(terminal: &mut TuiTerminal) -> io::Result<()> {
    let mut selected = 0usize;
    let sections = gallery_sections();

    loop {
        terminal.draw(|buf, area| render_gallery(buf, area, &sections, selected))?;
        if let Some(event) = terminal.poll_input(Duration::from_millis(100).as_millis() as u64) {
            match event {
                InputEvent::Key(key) => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => break,
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => {
                        selected = (selected + 1).min(sections.len().saturating_sub(1));
                    }
                    _ => {}
                },
                InputEvent::Resize { .. } => {}
                _ => {}
            }
        }
    }

    Ok(())
}

fn render_gallery(buf: &mut Buffer, area: Rect, sections: &[GallerySection], selected: usize) {
    let root = Block::default()
        .title(" display-tui surface gallery ")
        .title_bottom(" q/Esc quit · ↑/↓ select ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));
    let inner = root.inner(area);
    root.render(area, buf);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(30)])
        .split(inner);

    render_nav(buf, chunks[0], sections, selected);
    render_section(buf, chunks[1], &sections[selected]);
}

fn render_nav(buf: &mut Buffer, area: Rect, sections: &[GallerySection], selected: usize) {
    let lines = sections
        .iter()
        .enumerate()
        .map(|(index, section)| {
            let marker = if index == selected { "→" } else { " " };
            let style = if index == selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(vec![Span::styled(
                format!("{marker} {}", section.title),
                style,
            )])
        })
        .collect::<Vec<_>>();

    Paragraph::new(lines)
        .block(Block::default().title(" Surfaces ").borders(Borders::RIGHT))
        .render(area, buf);
}

fn render_section(buf: &mut Buffer, area: Rect, section: &GallerySection) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(5)])
        .split(area);

    Paragraph::new(section.summary)
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: false })
        .render(chunks[0], buf);

    let render_width = chunks[1].width.saturating_sub(4).max(1);
    let mut lines = render_widget_spec_to_lines(&section.spec, render_width);
    trim_rendered_lines(&mut lines);
    let lines = lines.into_iter().map(Line::from).collect::<Vec<_>>();

    Paragraph::new(lines)
        .block(
            Block::default()
                .title(section.title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .wrap(Wrap { trim: false })
        .render(chunks[1], buf);
}

struct GallerySection {
    title: &'static str,
    summary: &'static str,
    spec: WidgetSpec,
}

fn gallery_sections() -> Vec<GallerySection> {
    vec![
        section(
            "Chat Composer",
            "Composer state with editable text, cursor, attachments, pending queue, and footer.",
            ChatComposerSpec {
                lines: vec!["refactor display protocol widgets".to_string()],
                cursor_col: 8,
                attachments: vec!["src/tui/app.rs".to_string()],
                pending: Some(PendingInputPreviewSpec::new(vec![
                    "run cargo check".to_string(),
                    "commit feature".to_string(),
                ])),
                footer: footer("gpt-5", "Ctrl+J newline"),
                ..ChatComposerSpec::new(Vec::new())
            }
            .into(),
        ),
        section(
            "Text Area",
            "Multiline text area state with cursor, viewport, placeholder, and focus.",
            TextAreaSpec {
                lines: vec!["First line".to_string(), "Second editable line".to_string()],
                cursor_line: 1,
                cursor_col: 6,
                focused: true,
                ..TextAreaSpec::new(Vec::new())
            }
            .into(),
        ),
        section(
            "Command Popup",
            "Slash-command selection popup with query, selected row, descriptions, and footer.",
            selection_popup(SelectionPopupKindSpec::Command, "Commands").into(),
        ),
        section(
            "File Search Popup",
            "File-search popup using the same selection protocol with a different kind.",
            selection_popup(SelectionPopupKindSpec::FileSearch, "Files").into(),
        ),
        section(
            "Skill Mention Popup",
            "Skill or mention popup with shared row shape and backend-owned presentation.",
            selection_popup(SelectionPopupKindSpec::Skill, "Skills").into(),
        ),
        section(
            "Multi Select Picker",
            "Picker rows with checked state and confirm/cancel hints.",
            MultiSelectPickerSpec::new(
                "Enable skills",
                vec![
                    checked_row("imagegen", "imagegen", "Generate bitmap assets"),
                    row("openai-docs", "openai-docs", "Official OpenAI docs"),
                    row("trellis-check", "trellis-check", "Quality verification"),
                ],
            )
            .into(),
        ),
        section(
            "Approval Overlay",
            "Approval surface for commands, patches, permissions, network, and cross-thread requests.",
            ApprovalOverlaySpec {
                command: Some("cargo test -p display-tui".to_string()),
                choices: vec![
                    "Allow".to_string(),
                    "Deny".to_string(),
                    "Always allow".to_string(),
                ],
                ..ApprovalOverlaySpec::new(
                    ApprovalKindSpec::Exec,
                    "Run command?",
                    "The backend requests permission to execute this command.",
                )
            }
            .into(),
        ),
        section(
            "Request User Input",
            "Structured prompt with form fields, choice buttons, selected choice, and errors.",
            RequestUserInputOverlaySpec {
                fields: vec![field("branch", "Branch", "feature/widgets")],
                choices: vec!["Continue".to_string(), "Cancel".to_string()],
                ..RequestUserInputOverlaySpec::new("Input required", "Choose a branch name.")
            }
            .into(),
        ),
        section(
            "MCP Elicitation",
            "MCP server form request with fields and persistence options.",
            McpElicitationOverlaySpec {
                tool: Some("create_issue".to_string()),
                fields: vec![field("title", "Title", "TUI parity")],
                persist_options: vec!["Remember this server decision".to_string()],
                ..McpElicitationOverlaySpec::new("github", "Server needs confirmation.")
            }
            .into(),
        ),
        section(
            "Pending Approvals",
            "Cross-thread approval queue with selected row.",
            PendingThreadApprovalsSpec::new(vec![
                "thread A: cargo check".to_string(),
                "thread B: apply patch".to_string(),
            ])
            .into(),
        ),
        section(
            "App Link",
            "App-link suggestion or URL elicitation surface.",
            AppLinkViewSpec {
                reason: Some("Connect the local app server before continuing.".to_string()),
                ..AppLinkViewSpec::new(
                    AppLinkSuggestionTypeSpec::Enable,
                    "Enable local app",
                    "http://localhost:7891",
                )
            }
            .into(),
        ),
        section(
            "Feedback",
            "Feedback form state with diagnostics and upload consent rows.",
            FeedbackViewSpec {
                include_diagnostics: true,
                upload_consent_items: vec!["terminal log".to_string(), "session id".to_string()],
                ..FeedbackViewSpec::new("bad-result", "Widget output does not match reference.")
            }
            .into(),
        ),
        section(
            "Transcript",
            "Transcript composed from typed history cells.",
            TranscriptSpec::new(vec![
                HistoryCellSpec::new(HistoryCellKindSpec::User, vec!["implement all".to_string()]),
                HistoryCellSpec::new(
                    HistoryCellKindSpec::Agent,
                    vec!["Added backend-neutral surface specs.".to_string()],
                ),
                HistoryCellSpec::new(
                    HistoryCellKindSpec::Reasoning,
                    vec!["Protocol owns state; display-tui owns rendering.".to_string()],
                ),
            ])
            .into(),
        ),
        section(
            "Unified Exec",
            "Exec interaction with command, status, output streams, and process sessions.",
            UnifiedExecSpec {
                stdout: vec!["Checking display-tui".to_string()],
                sessions: vec!["session 1 running".to_string()],
                ..UnifiedExecSpec::new("cargo check -p display-tui", "running")
            }
            .into(),
        ),
        section(
            "MCP Tool Call",
            "MCP invocation cell with server, tool, arguments, status, and output.",
            McpToolCallSpec {
                arguments: vec![("path".to_string(), "README.md".to_string())],
                output: vec!["read 42 lines".to_string()],
                ..McpToolCallSpec::new("filesystem", "read_file", "success")
            }
            .into(),
        ),
        section(
            "Patch Cell",
            "Patch summary with changed files and add/remove counts.",
            PatchCellSpec {
                added: 128,
                removed: 4,
                status: "applied".to_string(),
                ..PatchCellSpec::new(vec![
                    "crates/display-protocol-widgets/src/surfaces.rs".to_string(),
                    "crates/display-tui/src/protocol_widgets.rs".to_string(),
                ])
            }
            .into(),
        ),
        section(
            "Plan Cell",
            "Proposed, streaming, or updated plan with an active step.",
            PlanCellSpec {
                active: Some(1),
                ..PlanCellSpec::new(
                    PlanCellKindSpec::Update,
                    vec![
                        "Add protocols".to_string(),
                        "Render in TUI".to_string(),
                        "Verify".to_string(),
                    ],
                )
            }
            .into(),
        ),
        section(
            "Hook Cell",
            "Hook execution history row with status and output.",
            HookCellSpec {
                output: vec!["pre-commit passed".to_string()],
                ..HookCellSpec::new("pre-commit", "success")
            }
            .into(),
        ),
        section(
            "Web Search Cell",
            "Search query and summarized result rows.",
            WebSearchCellSpec::new(
                "agent tui components",
                vec!["official docs".to_string(), "release notes".to_string()],
            )
            .into(),
        ),
        section(
            "Session Info",
            "Session header data: id, cwd, model, and effort.",
            SessionInfoCellSpec::new("abc123", "/data/Workspace/serana-new", "gpt-5").into(),
        ),
        section(
            "Status Indicator",
            "Compact status label with active marker and details.",
            StatusIndicatorSpec {
                active: true,
                details: Some("2 tools running".to_string()),
                ..StatusIndicatorSpec::new("Agent", "working")
            }
            .into(),
        ),
        section(
            "Status Surface",
            "Runtime, rate-limit, token, goal, warning, and startup status details.",
            StatusSurfaceSpec {
                rows: vec![
                    ("model".to_string(), "gpt-5".to_string()),
                    ("approval".to_string(), "on-request".to_string()),
                ],
                ..StatusSurfaceSpec::new(StatusSurfaceKindSpec::Runtime, "Runtime")
            }
            .into(),
        ),
        section(
            "Token Usage",
            "Token usage summary with limit, percent, reset time, and progress bar.",
            TokenUsageSpec {
                limit: Some(200_000),
                percent: Some(37),
                reset: Some("18:00".to_string()),
                ..TokenUsageSpec::new(74_000)
            }
            .into(),
        ),
        section(
            "Menu Surface",
            "Permission, model, goal, or settings menu rows.",
            MenuSurfaceSpec::new(
                "Permissions",
                vec![
                    row("read", "Read files", "allowed"),
                    checked_row("write", "Write workspace", "ask first"),
                ],
            )
            .into(),
        ),
        section(
            "Navigation Overlay",
            "Pager, transcript, static overlay, or live-tail navigation content.",
            NavigationOverlaySpec {
                lines: vec![
                    "Transcript page 1".to_string(),
                    "More rows below".to_string(),
                ],
                ..NavigationOverlaySpec::new(OverlayKindSpec::Pager, "Pager")
            }
            .into(),
        ),
        section(
            "Session Picker",
            "Resume-session picker with filter rows and preview content.",
            SessionPickerSpec {
                preview: vec!["last message: implement all".to_string()],
                ..SessionPickerSpec::new(vec![
                    row("today", "Today 15:20", "serana-new"),
                    row("yesterday", "Yesterday 22:01", "display widgets"),
                ])
            }
            .into(),
        ),
        section(
            "Setup Screen",
            "Startup/setup screen with typed kind, message, rows, and body widgets.",
            SetupScreenSpec {
                rows: vec![
                    checked_row("keep", "Keep current cwd", "/data/Workspace/serana-new"),
                    row("change", "Choose another cwd", "opens picker"),
                ],
                ..SetupScreenSpec::new(
                    SetupScreenKindSpec::CwdPrompt,
                    "Working directory",
                    "Confirm where the session should run.",
                )
            }
            .into(),
        ),
        section(
            "Integration View",
            "Hooks, memories, features, skills, plugins, connectors, or keymap views.",
            IntegrationViewSpec {
                rows: vec![
                    checked_row("hooks", "pre-commit", "managed"),
                    row("skills", "imagegen", "disabled"),
                ],
                details: vec!["Review integration state before enabling.".to_string()],
                ..IntegrationViewSpec::new(IntegrationViewKindSpec::HooksBrowser, "Hooks")
            }
            .into(),
        ),
        section(
            "Terminal Hyperlink",
            "Terminal hyperlink state without hard-coding escape sequences in the protocol crate.",
            TerminalHyperlinkSpec::new("Open docs", "https://platform.openai.com/docs").into(),
        ),
        section(
            "Markdown Stream",
            "Committed markdown plus streaming tail and holdback count.",
            MarkdownStreamSpec::new(
                "# Streaming\n- committed row\n",
                "- tail row still arriving",
            )
            .into(),
        ),
        section(
            "Animation",
            "Named frame sequence rendered by backend/app timing.",
            AnimationSpec {
                frame: 2,
                ..AnimationSpec::new(
                    "Thinking",
                    vec![
                        ".".to_string(),
                        "..".to_string(),
                        "...".to_string(),
                        "....".to_string(),
                    ],
                )
            }
            .into(),
        ),
        section(
            "Voice Meter",
            "Voice capture level and recording state.",
            VoiceMeterSpec {
                recording: true,
                ..VoiceMeterSpec::new("Voice", 64)
            }
            .into(),
        ),
    ]
}

fn section(title: &'static str, summary: &'static str, spec: WidgetSpec) -> GallerySection {
    GallerySection {
        title,
        summary,
        spec,
    }
}

fn selection_popup(kind: SelectionPopupKindSpec, title: &'static str) -> SelectionPopupSpec {
    SelectionPopupSpec {
        query: "mo".to_string(),
        rows: vec![
            row("model", "model", "switch active model"),
            row("memory", "memory", "open memories"),
            row("mcp", "mcp", "list servers"),
        ],
        selected: 1,
        footer: Some("Enter select · Esc close".to_string()),
        ..SelectionPopupSpec::new(kind, title)
    }
}

fn row(id: &str, label: &str, description: &str) -> SelectionRowSpec {
    SelectionRowSpec {
        description: Some(description.to_string()),
        ..SelectionRowSpec::new(id, label)
    }
}

fn checked_row(id: &str, label: &str, description: &str) -> SelectionRowSpec {
    SelectionRowSpec {
        checked: true,
        ..row(id, label, description)
    }
}

fn field(id: &str, label: &str, value: &str) -> FormFieldSpec {
    FormFieldSpec {
        value: value.to_string(),
        required: true,
        ..FormFieldSpec::new(id, label)
    }
}

fn footer(model: &str, hint: &str) -> FooterSurfaceSpec {
    FooterSurfaceSpec {
        left: vec![model.to_string()],
        right: vec![hint.to_string()],
        mode: Some("chat".to_string()),
        goal: Some("active".to_string()),
    }
}
