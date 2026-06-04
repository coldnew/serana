use display_protocol_widgets::{
    AppLinkSuggestionTypeSpec, AppLinkViewSpec, ApprovalKindSpec, ApprovalOverlaySpec,
    ChatComposerSpec, FeedbackViewSpec, FormFieldSpec, HistoryCellKindSpec, HistoryCellSpec,
    HookCellSpec, IntegrationViewKindSpec, IntegrationViewSpec, MarkdownStreamSpec,
    McpElicitationOverlaySpec, McpToolCallSpec, MenuSurfaceSpec, MultiSelectPickerSpec,
    NavigationOverlaySpec, OverlayKindSpec, PatchCellSpec, PendingInputPreviewSpec,
    PendingThreadApprovalsSpec, PlanCellKindSpec, PlanCellSpec, RequestUserInputOverlaySpec,
    SelectionPopupKindSpec, SelectionPopupSpec, SelectionRowSpec, SessionInfoCellSpec,
    SessionPickerSpec, SetupScreenKindSpec, SetupScreenSpec, StatusIndicatorSpec,
    StatusSurfaceKindSpec, StatusSurfaceSpec, TerminalHyperlinkSpec, TextAreaSpec, TokenUsageSpec,
    TranscriptSpec, UnifiedExecSpec, VoiceMeterSpec, WebSearchCellSpec, WidgetSpec,
};
use display_tui::{render_widget_spec_to_lines, trim_rendered_lines};

fn main() {
    for (title, spec) in gallery() {
        println!("\n=== {title} ===");
        let mut lines = render_widget_spec_to_lines(&spec, 88);
        trim_rendered_lines(&mut lines);
        for line in lines {
            println!("{line}");
        }
    }
}

fn gallery() -> Vec<(&'static str, WidgetSpec)> {
    vec![
        (
            "Chat Composer",
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
        (
            "Text Area",
            TextAreaSpec {
                lines: vec!["First line".to_string(), "Second editable line".to_string()],
                cursor_line: 1,
                cursor_col: 6,
                focused: true,
                ..TextAreaSpec::new(Vec::new())
            }
            .into(),
        ),
        (
            "Command Popup",
            selection_popup(SelectionPopupKindSpec::Command, "Commands").into(),
        ),
        (
            "File Search Popup",
            selection_popup(SelectionPopupKindSpec::FileSearch, "Files").into(),
        ),
        (
            "Skill Mention Popup",
            selection_popup(SelectionPopupKindSpec::Skill, "Skills").into(),
        ),
        (
            "Multi Select Picker",
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
        (
            "Approval Overlay",
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
        (
            "Request User Input",
            RequestUserInputOverlaySpec {
                fields: vec![field("branch", "Branch", "feature/widgets")],
                choices: vec!["Continue".to_string(), "Cancel".to_string()],
                ..RequestUserInputOverlaySpec::new("Input required", "Choose a branch name.")
            }
            .into(),
        ),
        (
            "MCP Elicitation",
            McpElicitationOverlaySpec {
                tool: Some("create_issue".to_string()),
                fields: vec![field("title", "Title", "TUI parity")],
                persist_options: vec!["Remember this server decision".to_string()],
                ..McpElicitationOverlaySpec::new("github", "Server needs confirmation.")
            }
            .into(),
        ),
        (
            "Pending Thread Approvals",
            PendingThreadApprovalsSpec::new(vec![
                "thread A: cargo check".to_string(),
                "thread B: apply patch".to_string(),
            ])
            .into(),
        ),
        (
            "App Link",
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
        (
            "Feedback",
            FeedbackViewSpec {
                include_diagnostics: true,
                upload_consent_items: vec!["terminal log".to_string(), "session id".to_string()],
                ..FeedbackViewSpec::new("bad-result", "Widget output does not match reference.")
            }
            .into(),
        ),
        (
            "History Cells",
            TranscriptSpec::new(vec![
                HistoryCellSpec::new(HistoryCellKindSpec::User, vec!["implement all".to_string()]),
                HistoryCellSpec::new(
                    HistoryCellKindSpec::Agent,
                    vec!["Added backend-neutral Codex widget specs.".to_string()],
                ),
                HistoryCellSpec::new(
                    HistoryCellKindSpec::Reasoning,
                    vec!["Protocol owns state; display-tui owns rendering.".to_string()],
                ),
            ])
            .into(),
        ),
        (
            "Unified Exec",
            UnifiedExecSpec {
                stdout: vec!["Checking display-tui".to_string()],
                sessions: vec!["session 1 running".to_string()],
                ..UnifiedExecSpec::new("cargo check -p display-tui", "running")
            }
            .into(),
        ),
        (
            "MCP Tool Call",
            McpToolCallSpec {
                arguments: vec![("path".to_string(), "README.md".to_string())],
                output: vec!["read 42 lines".to_string()],
                ..McpToolCallSpec::new("filesystem", "read_file", "success")
            }
            .into(),
        ),
        (
            "Patch Cell",
            PatchCellSpec {
                added: 128,
                removed: 4,
                status: "applied".to_string(),
                ..PatchCellSpec::new(vec![
                    "crates/display-protocol-widgets/src/codex.rs".to_string(),
                    "crates/display-tui/src/protocol_widgets.rs".to_string(),
                ])
            }
            .into(),
        ),
        (
            "Plan Cell",
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
        (
            "Hook Cell",
            HookCellSpec {
                output: vec!["pre-commit passed".to_string()],
                ..HookCellSpec::new("pre-commit", "success")
            }
            .into(),
        ),
        (
            "Web Search Cell",
            WebSearchCellSpec::new(
                "codex tui components",
                vec!["official docs".to_string(), "release notes".to_string()],
            )
            .into(),
        ),
        (
            "Session Info",
            SessionInfoCellSpec::new("abc123", "/data/Workspace/serana-new", "gpt-5").into(),
        ),
        (
            "Status Indicator",
            StatusIndicatorSpec {
                active: true,
                details: Some("2 tools running".to_string()),
                ..StatusIndicatorSpec::new("Agent", "working")
            }
            .into(),
        ),
        (
            "Status Surface",
            StatusSurfaceSpec {
                rows: vec![
                    ("model".to_string(), "gpt-5".to_string()),
                    ("approval".to_string(), "on-request".to_string()),
                ],
                ..StatusSurfaceSpec::new(StatusSurfaceKindSpec::Runtime, "Runtime")
            }
            .into(),
        ),
        (
            "Token Usage",
            TokenUsageSpec {
                limit: Some(200_000),
                percent: Some(37),
                reset: Some("18:00".to_string()),
                ..TokenUsageSpec::new(74_000)
            }
            .into(),
        ),
        (
            "Menu Surface",
            MenuSurfaceSpec::new(
                "Permissions",
                vec![
                    row("read", "Read files", "allowed"),
                    checked_row("write", "Write workspace", "ask first"),
                ],
            )
            .into(),
        ),
        (
            "Navigation Overlay",
            NavigationOverlaySpec {
                lines: vec![
                    "Transcript page 1".to_string(),
                    "More rows below".to_string(),
                ],
                ..NavigationOverlaySpec::new(OverlayKindSpec::Pager, "Pager")
            }
            .into(),
        ),
        (
            "Session Picker",
            SessionPickerSpec {
                preview: vec!["last message: implement all".to_string()],
                ..SessionPickerSpec::new(vec![
                    row("today", "Today 15:20", "serana-new"),
                    row("yesterday", "Yesterday 22:01", "display widgets"),
                ])
            }
            .into(),
        ),
        (
            "Setup Screen",
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
        (
            "Integration View",
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
        (
            "Terminal Hyperlink",
            TerminalHyperlinkSpec::new("Open docs", "https://platform.openai.com/docs").into(),
        ),
        (
            "Markdown Stream",
            MarkdownStreamSpec::new(
                "# Streaming\n- committed row\n",
                "- tail row still arriving",
            )
            .into(),
        ),
        (
            "Animation",
            display_protocol_widgets::AnimationSpec {
                frame: 1,
                ..display_protocol_widgets::AnimationSpec::new(
                    "Thinking",
                    vec![".".to_string(), "..".to_string(), "...".to_string()],
                )
            }
            .into(),
        ),
        (
            "Voice Meter",
            VoiceMeterSpec {
                recording: true,
                ..VoiceMeterSpec::new("Voice", 64)
            }
            .into(),
        ),
    ]
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

fn footer(model: &str, hint: &str) -> display_protocol_widgets::FooterSurfaceSpec {
    display_protocol_widgets::FooterSurfaceSpec {
        left: vec![model.to_string()],
        right: vec![hint.to_string()],
        mode: Some("chat".to_string()),
        goal: Some("active".to_string()),
    }
}
