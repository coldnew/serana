use display_protocol::{InputEvent, KeyCode};
use display_protocol_widgets::{
    AssistantMessageSpec, BashExecutionSpec, BorderedLoaderSpec, BoxSpec, CancellableLoaderSpec,
    CountdownTimerSpec, DiffLineKindSpec, DiffLineSpec, DiffSpec, DynamicBorderSpec, EditorSpec,
    ExecutionStatusSpec, FooterSpec, HistorySearchSpec, ImageSpec, InputSpec, KeybindingHintSpec,
    KeybindingHintsSpec, LoaderSpec, LoginDialogSpec, MarkdownSpec, MessageFrameSpec,
    MessageRoleSpec, SelectItemSpec, SelectListSpec, SelectorOptionSpec, SelectorSpec,
    SettingItemSpec, SettingsListSpec, SpacerSpec, StatusLineSpec, StatusSegmentSpec, TabBarSpec,
    TabItemSpec, TextSpec, TodoReminderSpec, ToolExecutionSpec, TreeNodeSpec, TreeSelectorSpec,
    TruncatedTextSpec, UserMessageSpec, VisualTruncateSpec, WelcomeSpec, WidgetSpec,
};
use display_tui::{render_widget_spec_to_lines, TuiTerminal};
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
                        selected = (selected + 1).min(sections.len().saturating_sub(1))
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
        .title(" display-tui widget gallery ")
        .title_bottom(" q/Esc quit · ↑/↓ select ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));
    let inner = root.inner(area);
    root.render(area, buf);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(20)])
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
        .block(
            Block::default()
                .title(" Components ")
                .borders(Borders::RIGHT),
        )
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
    display_tui::trim_rendered_lines(&mut lines);
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
        GallerySection {
            title: "Text",
            summary: "Pi-style Text: wraps text, applies horizontal and vertical padding.",
            spec: TextSpec::new("Serana can share one widget protocol while each backend owns its renderer.")
                .padding(2, 1)
                .into(),
        },
        GallerySection {
            title: "Box",
            summary: "Box is a protocol container. TUI rendering pads child lines to the available width.",
            spec: BoxSpec::new(vec![
                TextSpec::new("Model: gpt-5").padding(0, 0).into(),
                SpacerSpec::new(1).into(),
                TextSpec::new("Approval: on-request").padding(0, 0).into(),
            ])
            .padding(2, 1)
            .into(),
        },
        GallerySection {
            title: "Input",
            summary: "Input carries prompt, value, cursor, and focus state; display-tui renders the cursor.",
            spec: InputSpec::new("explain display protocol widgets")
                .cursor(8)
                .focused(true)
                .into(),
        },
        GallerySection {
            title: "Loader",
            summary: "Loader protocol stores message and animation frame; display-tui chooses terminal glyphs.",
            spec: LoaderSpec::new("Loading conversation context").frame(3).into(),
        },
        GallerySection {
            title: "Cancellable Loader",
            summary: "CancellableLoader extends Loader state with a backend-neutral cancel hint.",
            spec: CancellableLoaderSpec::new("Calling tool")
                .frame(5)
                .cancel_hint("Esc to cancel")
                .into(),
        },
        GallerySection {
            title: "Select List",
            summary: "SelectList mirrors ref/pi: selected cursor, primary column, optional description column.",
            spec: SelectListSpec::new(vec![
                SelectItemSpec::new("model", "Model").description("Change active model"),
                SelectItemSpec::new("login", "Login").description("Authenticate provider"),
                SelectItemSpec::new("export", "Export").description("Write session to disk"),
                SelectItemSpec::new("copy", "Copy").description("Copy last assistant output"),
            ])
            .selected(1)
            .max_visible(4)
            .into(),
        },
        GallerySection {
            title: "Settings List",
            summary: "SettingsList carries labels, values, selected item, description, and hint text.",
            spec: SettingsListSpec::new(vec![
                SettingItemSpec::new("model", "Model", "gpt-5"),
                SettingItemSpec::new("reasoning", "Reasoning", "high"),
                SettingItemSpec::new("approval", "Approval Mode", "on-request")
                    .description("Ask before commands that need elevated permissions."),
            ])
            .selected(2)
            .into(),
        },
        GallerySection {
            title: "Markdown",
            summary: "Markdown is protocol text. TUI backend handles terminal markdown conventions.",
            spec: MarkdownSpec::new("# Release notes\n- Shared widget protocol\n> Rendered by display-tui")
                .padding(1, 1)
                .into(),
        },
        GallerySection {
            title: "Truncated Text",
            summary: "TruncatedText keeps long terminal rows bounded by backend width.",
            spec: TruncatedTextSpec::new(
                "very-long-workspace-path/ref/pi/packages/tui/src/components/select-list.ts",
            )
            .padding(1, 1)
            .into(),
        },
        GallerySection {
            title: "Editor",
            summary: "Editor carries buffer lines, cursor, viewport, gutter, and focus state.",
            spec: EditorSpec::new(vec![
                "fn main() {".to_string(),
                "    println!(\"serana\");".to_string(),
                "}".to_string(),
            ])
            .cursor(1, 12)
            .height(5)
            .focused(true)
            .into(),
        },
        GallerySection {
            title: "Image",
            summary: "Image reserves a frame for backends that can draw pixels while TUI shows useful metadata.",
            spec: ImageSpec::new("session-preview", 44, 6)
                .alt("terminal preview image")
                .into(),
        },
        GallerySection {
            title: "Tab Bar",
            summary: "TabBar mirrors editor/session tabs with active and modified state.",
            spec: TabBarSpec::new(vec![
                TabItemSpec::new("chat", "Chat"),
                TabItemSpec::new("files", "Files"),
                TabItemSpec::new("git", "Git").modified(true),
            ])
            .active(1)
            .into(),
        },
        GallerySection {
            title: "Message Frame",
            summary: "MessageFrame is the generic role-aware container for conversation rows.",
            spec: MessageFrameSpec::new(
                "System",
                MessageRoleSpec::System,
                vec![TextSpec::new("Follow project coding rules.").padding(0, 0).into()],
            )
            .into(),
        },
        GallerySection {
            title: "Assistant Message",
            summary: "AssistantMessage carries assistant markdown and optional model metadata.",
            spec: AssistantMessageSpec::new("Implemented shared widget specs.\n- protocol first\n- backend renderers own visuals")
                .model("gpt-5")
                .into(),
        },
        GallerySection {
            title: "User Message",
            summary: "UserMessage carries the prompt content separately from assistant/tool frames.",
            spec: UserMessageSpec::new("implement all missing TUI components").into(),
        },
        GallerySection {
            title: "Tool Execution",
            summary: "ToolExecution covers non-shell tool calls with status, command label, and output.",
            spec: ToolExecutionSpec::new("apply_patch", ExecutionStatusSpec::Success)
                .command("update display-tui renderer")
                .output(vec!["2 files changed".to_string()])
                .into(),
        },
        GallerySection {
            title: "Bash Execution",
            summary: "BashExecution covers terminal commands with status, exit code, and output lines.",
            spec: BashExecutionSpec::new("cargo check -p display-tui --examples", ExecutionStatusSpec::Running)
                .output(vec!["checking display-tui".to_string()])
                .into(),
        },
        GallerySection {
            title: "Diff",
            summary: "Diff carries semantic diff lines for TUI, WGPU, and log backends.",
            spec: DiffSpec::new(
                "protocol_widgets.rs",
                vec![
                    DiffLineSpec::new(DiffLineKindSpec::Hunk, "@@ renderer @@"),
                    DiffLineSpec::new(DiffLineKindSpec::Removed, "WidgetSpec::Markdown only"),
                    DiffLineSpec::new(DiffLineKindSpec::Added, "WidgetSpec::Editor"),
                ],
            )
            .into(),
        },
        GallerySection {
            title: "Status Line",
            summary: "StatusLine stores left and right status segments for terminal chrome.",
            spec: StatusLineSpec::new(
                vec![
                    StatusSegmentSpec::new("NORMAL"),
                    StatusSegmentSpec::new("model").value("gpt-5"),
                ],
                vec![StatusSegmentSpec::new("Rust"), StatusSegmentSpec::new("UTF-8")],
            )
            .into(),
        },
        GallerySection {
            title: "Footer",
            summary: "Footer is compact command/help text for the bottom of an app surface.",
            spec: FooterSpec::new(vec![
                "q quit".to_string(),
                "Enter select".to_string(),
                "Esc cancel".to_string(),
            ])
            .into(),
        },
        GallerySection {
            title: "Keybinding Hints",
            summary: "KeybindingHints carries structured key-label pairs instead of preformatted text.",
            spec: KeybindingHintsSpec::new(vec![
                KeybindingHintSpec::new("Ctrl+K", "commands"),
                KeybindingHintSpec::new("Ctrl+L", "login"),
                KeybindingHintSpec::new("?", "help"),
            ])
            .into(),
        },
        GallerySection {
            title: "Model Selector",
            summary: "ModelSelector is the typed selector used by model switching UI.",
            spec: selector("Model", &["gpt-5", "gpt-5-mini", "local/qwen"])
                .selected(1)
                .into_model_selector(),
        },
        GallerySection {
            title: "Session Selector",
            summary: "SessionSelector uses the same selector protocol with a distinct semantic variant.",
            spec: selector("Session", &["current", "refactor-serana", "display-protocol"])
                .selected(2)
                .into_session_selector(),
        },
        GallerySection {
            title: "Settings Selector",
            summary: "SettingsSelector represents focused setting choices.",
            spec: selector("Approval Mode", &["never", "on-request", "always"])
                .selected(1)
                .into_settings_selector(),
        },
        GallerySection {
            title: "Theme Selector",
            summary: "ThemeSelector represents terminal theme choices.",
            spec: selector("Theme", &["system", "light", "dark"])
                .selected(2)
                .into_theme_selector(),
        },
        GallerySection {
            title: "Tree Selector",
            summary: "TreeSelector carries hierarchical choices and expanded state.",
            spec: TreeSelectorSpec::new(
                "Files",
                vec![
                    TreeNodeSpec::new("src", "src")
                        .expanded(true)
                        .children(vec![
                            TreeNodeSpec::new("app", "app.rs"),
                            TreeNodeSpec::new("tui", "tui").expanded(true).children(vec![
                                TreeNodeSpec::new("widgets", "protocol_widgets.rs"),
                            ]),
                        ]),
                    TreeNodeSpec::new("cargo", "Cargo.toml"),
                ],
            )
            .selected_path(vec![0, 1, 0])
            .into(),
        },
        GallerySection {
            title: "Login Dialog",
            summary: "LoginDialog covers the /login device flow state.",
            spec: LoginDialogSpec::new("Codex", "https://auth.openai.com/device", "ABCD-EFGH")
                .status("Waiting for confirmation")
                .into(),
        },
        GallerySection {
            title: "History Search",
            summary: "HistorySearch carries query, results, and selected result.",
            spec: HistorySearchSpec::new(
                "display",
                vec![
                    "refactor display protocol widgets".to_string(),
                    "render widget gallery in tui".to_string(),
                    "fix model persistence".to_string(),
                ],
            )
            .selected(1)
            .into(),
        },
        GallerySection {
            title: "Countdown Timer",
            summary: "CountdownTimer is for time-bounded confirmation or retry UI.",
            spec: CountdownTimerSpec::new("Retry in", 12).into(),
        },
        GallerySection {
            title: "Todo Reminder",
            summary: "TodoReminder renders compact pending task state.",
            spec: TodoReminderSpec::new(vec![
                "finish protocol coverage".to_string(),
                "run cargo checks".to_string(),
                "commit focused diff".to_string(),
            ])
            .into(),
        },
        GallerySection {
            title: "Welcome",
            summary: "Welcome is the first-run/start screen protocol from oh-my-pi style surfaces.",
            spec: WelcomeSpec::new("Serana", "Shared terminal interface")
                .actions(vec!["/login authenticate".to_string(), "/model select model".to_string()])
                .into(),
        },
        GallerySection {
            title: "Bordered Loader",
            summary: "BorderedLoader combines loader state with a titled terminal frame.",
            spec: BorderedLoaderSpec::new("Thinking", LoaderSpec::new("reasoning").frame(4)).into(),
        },
        GallerySection {
            title: "Dynamic Border",
            summary: "DynamicBorder carries active/inactive frame state around any child widget.",
            spec: DynamicBorderSpec::new(
                "Focused",
                TextSpec::new("Active input area").padding(0, 0).into(),
            )
            .active(true)
            .into(),
        },
        GallerySection {
            title: "Visual Truncate",
            summary: "VisualTruncate uses a suffix-aware truncation contract for paths and labels.",
            spec: VisualTruncateSpec::new("ref/pi/packages/tui/src/components/cancellable-loader.ts")
                .suffix("…")
                .into(),
        },
    ]
}

fn selector(title: &'static str, labels: &[&'static str]) -> SelectorSpec {
    SelectorSpec::new(
        title,
        labels
            .iter()
            .map(|label| SelectorOptionSpec::new(*label, *label).description("available"))
            .collect(),
    )
}
