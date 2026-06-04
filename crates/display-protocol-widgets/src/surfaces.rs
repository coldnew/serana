//! Agent TUI surface protocols.
//!
//! These specs intentionally describe state and layout intent only. Backends
//! render them with their own widgets while callers avoid depending on
//! application-specific runtime types.

use serde::{Deserialize, Serialize};

use crate::protocol::WidgetSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComposerModeSpec {
    Chat,
    Plan,
    Shell,
    Search,
    Overlay,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatComposerSpec {
    pub mode: ComposerModeSpec,
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub attachments: Vec<String>,
    pub pending: Option<PendingInputPreviewSpec>,
    pub popup: Option<SelectionPopupSpec>,
    pub footer: FooterSurfaceSpec,
    pub active: bool,
}

impl ChatComposerSpec {
    pub fn new(lines: Vec<String>) -> Self {
        Self {
            mode: ComposerModeSpec::Chat,
            lines,
            cursor_line: 0,
            cursor_col: 0,
            attachments: Vec::new(),
            pending: None,
            popup: None,
            footer: FooterSurfaceSpec::default(),
            active: true,
        }
    }
}

impl From<ChatComposerSpec> for WidgetSpec {
    fn from(spec: ChatComposerSpec) -> Self {
        Self::ChatComposer(Box::new(spec))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextAreaSpec {
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub scroll_top: usize,
    pub height: u16,
    pub placeholder: Option<String>,
    pub focused: bool,
}

impl TextAreaSpec {
    pub fn new(lines: Vec<String>) -> Self {
        Self {
            lines,
            cursor_line: 0,
            cursor_col: 0,
            scroll_top: 0,
            height: 4,
            placeholder: None,
            focused: false,
        }
    }
}

impl From<TextAreaSpec> for WidgetSpec {
    fn from(spec: TextAreaSpec) -> Self {
        Self::TextArea(spec)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionPopupKindSpec {
    Command,
    FileSearch,
    Skill,
    Mention,
    Generic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionRowSpec {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub matched: Option<String>,
    pub checked: bool,
    pub disabled: bool,
}

impl SelectionRowSpec {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
            matched: None,
            checked: false,
            disabled: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionPopupSpec {
    pub kind: SelectionPopupKindSpec,
    pub title: String,
    pub query: String,
    pub rows: Vec<SelectionRowSpec>,
    pub selected: usize,
    pub max_visible: usize,
    pub footer: Option<String>,
}

impl SelectionPopupSpec {
    pub fn new(kind: SelectionPopupKindSpec, title: impl Into<String>) -> Self {
        Self {
            kind,
            title: title.into(),
            query: String::new(),
            rows: Vec::new(),
            selected: 0,
            max_visible: 8,
            footer: None,
        }
    }
}

impl From<SelectionPopupSpec> for WidgetSpec {
    fn from(spec: SelectionPopupSpec) -> Self {
        Self::SelectionPopup(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiSelectPickerSpec {
    pub title: String,
    pub rows: Vec<SelectionRowSpec>,
    pub selected: usize,
    pub confirm_label: String,
    pub cancel_label: String,
}

impl MultiSelectPickerSpec {
    pub fn new(title: impl Into<String>, rows: Vec<SelectionRowSpec>) -> Self {
        Self {
            title: title.into(),
            rows,
            selected: 0,
            confirm_label: "Enter to confirm".to_string(),
            cancel_label: "Esc to cancel".to_string(),
        }
    }
}

impl From<MultiSelectPickerSpec> for WidgetSpec {
    fn from(spec: MultiSelectPickerSpec) -> Self {
        Self::MultiSelectPicker(spec)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalKindSpec {
    Exec,
    Patch,
    Permission,
    Network,
    CrossThread,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalOverlaySpec {
    pub kind: ApprovalKindSpec,
    pub title: String,
    pub message: String,
    pub command: Option<String>,
    pub choices: Vec<String>,
    pub selected: usize,
}

impl ApprovalOverlaySpec {
    pub fn new(
        kind: ApprovalKindSpec,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            title: title.into(),
            message: message.into(),
            command: None,
            choices: vec!["Approve".to_string(), "Deny".to_string()],
            selected: 0,
        }
    }
}

impl From<ApprovalOverlaySpec> for WidgetSpec {
    fn from(spec: ApprovalOverlaySpec) -> Self {
        Self::ApprovalOverlay(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormFieldSpec {
    pub id: String,
    pub label: String,
    pub value: String,
    pub placeholder: Option<String>,
    pub required: bool,
    pub error: Option<String>,
}

impl FormFieldSpec {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value: String::new(),
            placeholder: None,
            required: false,
            error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestUserInputOverlaySpec {
    pub title: String,
    pub prompt: String,
    pub fields: Vec<FormFieldSpec>,
    pub choices: Vec<String>,
    pub selected: usize,
}

impl RequestUserInputOverlaySpec {
    pub fn new(title: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            prompt: prompt.into(),
            fields: Vec::new(),
            choices: Vec::new(),
            selected: 0,
        }
    }
}

impl From<RequestUserInputOverlaySpec> for WidgetSpec {
    fn from(spec: RequestUserInputOverlaySpec) -> Self {
        Self::RequestUserInputOverlay(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpElicitationOverlaySpec {
    pub server: String,
    pub tool: Option<String>,
    pub message: String,
    pub fields: Vec<FormFieldSpec>,
    pub persist_options: Vec<String>,
}

impl McpElicitationOverlaySpec {
    pub fn new(server: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            server: server.into(),
            tool: None,
            message: message.into(),
            fields: Vec::new(),
            persist_options: Vec::new(),
        }
    }
}

impl From<McpElicitationOverlaySpec> for WidgetSpec {
    fn from(spec: McpElicitationOverlaySpec) -> Self {
        Self::McpElicitationOverlay(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingInputPreviewSpec {
    pub title: String,
    pub messages: Vec<String>,
    pub interrupt_hint: Option<String>,
    pub max_visible: usize,
}

impl PendingInputPreviewSpec {
    pub fn new(messages: Vec<String>) -> Self {
        Self {
            title: "Queued input".to_string(),
            messages,
            interrupt_hint: None,
            max_visible: 3,
        }
    }
}

impl From<PendingInputPreviewSpec> for WidgetSpec {
    fn from(spec: PendingInputPreviewSpec) -> Self {
        Self::PendingInputPreview(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingThreadApprovalsSpec {
    pub title: String,
    pub approvals: Vec<String>,
    pub selected: usize,
}

impl PendingThreadApprovalsSpec {
    pub fn new(approvals: Vec<String>) -> Self {
        Self {
            title: "Pending approvals".to_string(),
            approvals,
            selected: 0,
        }
    }
}

impl From<PendingThreadApprovalsSpec> for WidgetSpec {
    fn from(spec: PendingThreadApprovalsSpec) -> Self {
        Self::PendingThreadApprovals(spec)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppLinkSuggestionTypeSpec {
    Auth,
    Enable,
    Install,
    GenericUrl,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppLinkViewSpec {
    pub suggestion_type: AppLinkSuggestionTypeSpec,
    pub title: String,
    pub url: String,
    pub reason: Option<String>,
    pub confirmation: Option<String>,
}

impl AppLinkViewSpec {
    pub fn new(
        suggestion_type: AppLinkSuggestionTypeSpec,
        title: impl Into<String>,
        url: impl Into<String>,
    ) -> Self {
        Self {
            suggestion_type,
            title: title.into(),
            url: url.into(),
            reason: None,
            confirmation: None,
        }
    }
}

impl From<AppLinkViewSpec> for WidgetSpec {
    fn from(spec: AppLinkViewSpec) -> Self {
        Self::AppLinkView(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedbackViewSpec {
    pub category: String,
    pub note: String,
    pub include_diagnostics: bool,
    pub upload_consent_items: Vec<String>,
}

impl FeedbackViewSpec {
    pub fn new(category: impl Into<String>, note: impl Into<String>) -> Self {
        Self {
            category: category.into(),
            note: note.into(),
            include_diagnostics: false,
            upload_consent_items: Vec::new(),
        }
    }
}

impl From<FeedbackViewSpec> for WidgetSpec {
    fn from(spec: FeedbackViewSpec) -> Self {
        Self::FeedbackView(spec)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryCellKindSpec {
    Plain,
    User,
    Agent,
    Reasoning,
    StreamingTail,
    Exec,
    Mcp,
    Patch,
    Plan,
    Hook,
    WebSearch,
    Session,
    Separator,
    Approval,
    RequestUserInputResult,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryCellSpec {
    pub kind: HistoryCellKindSpec,
    pub title: Option<String>,
    pub lines: Vec<String>,
    pub children: Vec<WidgetSpec>,
    pub metadata: Vec<(String, String)>,
}

impl HistoryCellSpec {
    pub fn new(kind: HistoryCellKindSpec, lines: Vec<String>) -> Self {
        Self {
            kind,
            title: None,
            lines,
            children: Vec::new(),
            metadata: Vec::new(),
        }
    }
}

impl From<HistoryCellSpec> for WidgetSpec {
    fn from(spec: HistoryCellSpec) -> Self {
        Self::HistoryCell(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptSpec {
    pub title: String,
    pub cells: Vec<HistoryCellSpec>,
    pub scroll_top: usize,
    pub height: u16,
}

impl TranscriptSpec {
    pub fn new(cells: Vec<HistoryCellSpec>) -> Self {
        Self {
            title: "Transcript".to_string(),
            cells,
            scroll_top: 0,
            height: 20,
        }
    }
}

impl From<TranscriptSpec> for WidgetSpec {
    fn from(spec: TranscriptSpec) -> Self {
        Self::Transcript(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnifiedExecSpec {
    pub title: String,
    pub command: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
    pub sessions: Vec<String>,
}

impl UnifiedExecSpec {
    pub fn new(command: impl Into<String>, status: impl Into<String>) -> Self {
        Self {
            title: "Exec".to_string(),
            command: command.into(),
            status: status.into(),
            exit_code: None,
            stdout: Vec::new(),
            stderr: Vec::new(),
            sessions: Vec::new(),
        }
    }
}

impl From<UnifiedExecSpec> for WidgetSpec {
    fn from(spec: UnifiedExecSpec) -> Self {
        Self::UnifiedExec(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpToolCallSpec {
    pub server: String,
    pub tool: String,
    pub status: String,
    pub arguments: Vec<(String, String)>,
    pub output: Vec<String>,
}

impl McpToolCallSpec {
    pub fn new(
        server: impl Into<String>,
        tool: impl Into<String>,
        status: impl Into<String>,
    ) -> Self {
        Self {
            server: server.into(),
            tool: tool.into(),
            status: status.into(),
            arguments: Vec::new(),
            output: Vec::new(),
        }
    }
}

impl From<McpToolCallSpec> for WidgetSpec {
    fn from(spec: McpToolCallSpec) -> Self {
        Self::McpToolCall(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchCellSpec {
    pub title: String,
    pub files: Vec<String>,
    pub added: usize,
    pub removed: usize,
    pub status: String,
}

impl PatchCellSpec {
    pub fn new(files: Vec<String>) -> Self {
        Self {
            title: "Patch".to_string(),
            files,
            added: 0,
            removed: 0,
            status: "pending".to_string(),
        }
    }
}

impl From<PatchCellSpec> for WidgetSpec {
    fn from(spec: PatchCellSpec) -> Self {
        Self::PatchCell(spec)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanCellKindSpec {
    Proposed,
    Streaming,
    Update,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanCellSpec {
    pub kind: PlanCellKindSpec,
    pub steps: Vec<String>,
    pub active: Option<usize>,
}

impl PlanCellSpec {
    pub fn new(kind: PlanCellKindSpec, steps: Vec<String>) -> Self {
        Self {
            kind,
            steps,
            active: None,
        }
    }
}

impl From<PlanCellSpec> for WidgetSpec {
    fn from(spec: PlanCellSpec) -> Self {
        Self::PlanCell(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookCellSpec {
    pub hook: String,
    pub status: String,
    pub output: Vec<String>,
}

impl HookCellSpec {
    pub fn new(hook: impl Into<String>, status: impl Into<String>) -> Self {
        Self {
            hook: hook.into(),
            status: status.into(),
            output: Vec::new(),
        }
    }
}

impl From<HookCellSpec> for WidgetSpec {
    fn from(spec: HookCellSpec) -> Self {
        Self::HookCell(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchCellSpec {
    pub query: String,
    pub results: Vec<String>,
}

impl WebSearchCellSpec {
    pub fn new(query: impl Into<String>, results: Vec<String>) -> Self {
        Self {
            query: query.into(),
            results,
        }
    }
}

impl From<WebSearchCellSpec> for WidgetSpec {
    fn from(spec: WebSearchCellSpec) -> Self {
        Self::WebSearchCell(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfoCellSpec {
    pub session_id: String,
    pub cwd: String,
    pub model: String,
    pub effort: Option<String>,
}

impl SessionInfoCellSpec {
    pub fn new(
        session_id: impl Into<String>,
        cwd: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            cwd: cwd.into(),
            model: model.into(),
            effort: None,
        }
    }
}

impl From<SessionInfoCellSpec> for WidgetSpec {
    fn from(spec: SessionInfoCellSpec) -> Self {
        Self::SessionInfoCell(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusIndicatorSpec {
    pub label: String,
    pub state: String,
    pub details: Option<String>,
    pub active: bool,
}

impl StatusIndicatorSpec {
    pub fn new(label: impl Into<String>, state: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            state: state.into(),
            details: None,
            active: false,
        }
    }
}

impl From<StatusIndicatorSpec> for WidgetSpec {
    fn from(spec: StatusIndicatorSpec) -> Self {
        Self::StatusIndicator(spec)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusSurfaceKindSpec {
    Runtime,
    RateLimit,
    TokenUsage,
    Goal,
    Warning,
    McpStartup,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusSurfaceSpec {
    pub kind: StatusSurfaceKindSpec,
    pub title: String,
    pub rows: Vec<(String, String)>,
    pub body: Vec<WidgetSpec>,
}

impl StatusSurfaceSpec {
    pub fn new(kind: StatusSurfaceKindSpec, title: impl Into<String>) -> Self {
        Self {
            kind,
            title: title.into(),
            rows: Vec::new(),
            body: Vec::new(),
        }
    }
}

impl From<StatusSurfaceSpec> for WidgetSpec {
    fn from(spec: StatusSurfaceSpec) -> Self {
        Self::StatusSurface(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsageSpec {
    pub used: u64,
    pub limit: Option<u64>,
    pub percent: Option<u8>,
    pub reset: Option<String>,
}

impl TokenUsageSpec {
    pub fn new(used: u64) -> Self {
        Self {
            used,
            limit: None,
            percent: None,
            reset: None,
        }
    }
}

impl From<TokenUsageSpec> for WidgetSpec {
    fn from(spec: TokenUsageSpec) -> Self {
        Self::TokenUsage(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MenuSurfaceSpec {
    pub title: String,
    pub rows: Vec<SelectionRowSpec>,
    pub selected: usize,
}

impl MenuSurfaceSpec {
    pub fn new(title: impl Into<String>, rows: Vec<SelectionRowSpec>) -> Self {
        Self {
            title: title.into(),
            rows,
            selected: 0,
        }
    }
}

impl From<MenuSurfaceSpec> for WidgetSpec {
    fn from(spec: MenuSurfaceSpec) -> Self {
        Self::MenuSurface(spec)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlayKindSpec {
    Transcript,
    Pager,
    Static,
    LiveTail,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavigationOverlaySpec {
    pub kind: OverlayKindSpec,
    pub title: String,
    pub lines: Vec<String>,
    pub body: Vec<WidgetSpec>,
    pub scroll_top: usize,
}

impl NavigationOverlaySpec {
    pub fn new(kind: OverlayKindSpec, title: impl Into<String>) -> Self {
        Self {
            kind,
            title: title.into(),
            lines: Vec::new(),
            body: Vec::new(),
            scroll_top: 0,
        }
    }
}

impl From<NavigationOverlaySpec> for WidgetSpec {
    fn from(spec: NavigationOverlaySpec) -> Self {
        Self::NavigationOverlay(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionPickerSpec {
    pub title: String,
    pub query: String,
    pub rows: Vec<SelectionRowSpec>,
    pub selected: usize,
    pub preview: Vec<String>,
}

impl SessionPickerSpec {
    pub fn new(rows: Vec<SelectionRowSpec>) -> Self {
        Self {
            title: "Resume session".to_string(),
            query: String::new(),
            rows,
            selected: 0,
            preview: Vec::new(),
        }
    }
}

impl From<SessionPickerSpec> for WidgetSpec {
    fn from(spec: SessionPickerSpec) -> Self {
        Self::SessionPicker(spec)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetupScreenKindSpec {
    CwdPrompt,
    UpdatePrompt,
    ModelMigration,
    ExternalAgentMigration,
    StartupHooksReview,
    OssProviderSelection,
    StatusLineSetup,
    TerminalTitleSetup,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetupScreenSpec {
    pub kind: SetupScreenKindSpec,
    pub title: String,
    pub message: String,
    pub rows: Vec<SelectionRowSpec>,
    pub body: Vec<WidgetSpec>,
    pub selected: usize,
}

impl SetupScreenSpec {
    pub fn new(
        kind: SetupScreenKindSpec,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            title: title.into(),
            message: message.into(),
            rows: Vec::new(),
            body: Vec::new(),
            selected: 0,
        }
    }
}

impl From<SetupScreenSpec> for WidgetSpec {
    fn from(spec: SetupScreenSpec) -> Self {
        Self::SetupScreen(spec)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntegrationViewKindSpec {
    HooksBrowser,
    MemoriesSettings,
    ExperimentalFeatures,
    SkillsToggle,
    Plugins,
    Connectors,
    KeymapPicker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationViewSpec {
    pub kind: IntegrationViewKindSpec,
    pub title: String,
    pub rows: Vec<SelectionRowSpec>,
    pub selected: usize,
    pub details: Vec<String>,
}

impl IntegrationViewSpec {
    pub fn new(kind: IntegrationViewKindSpec, title: impl Into<String>) -> Self {
        Self {
            kind,
            title: title.into(),
            rows: Vec::new(),
            selected: 0,
            details: Vec::new(),
        }
    }
}

impl From<IntegrationViewSpec> for WidgetSpec {
    fn from(spec: IntegrationViewSpec) -> Self {
        Self::IntegrationView(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalHyperlinkSpec {
    pub label: String,
    pub uri: String,
}

impl TerminalHyperlinkSpec {
    pub fn new(label: impl Into<String>, uri: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            uri: uri.into(),
        }
    }
}

impl From<TerminalHyperlinkSpec> for WidgetSpec {
    fn from(spec: TerminalHyperlinkSpec) -> Self {
        Self::TerminalHyperlink(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownStreamSpec {
    pub committed: String,
    pub streaming_tail: String,
    pub holdback_lines: usize,
}

impl MarkdownStreamSpec {
    pub fn new(committed: impl Into<String>, streaming_tail: impl Into<String>) -> Self {
        Self {
            committed: committed.into(),
            streaming_tail: streaming_tail.into(),
            holdback_lines: 0,
        }
    }
}

impl From<MarkdownStreamSpec> for WidgetSpec {
    fn from(spec: MarkdownStreamSpec) -> Self {
        Self::MarkdownStream(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimationSpec {
    pub name: String,
    pub frames: Vec<String>,
    pub frame: usize,
}

impl AnimationSpec {
    pub fn new(name: impl Into<String>, frames: Vec<String>) -> Self {
        Self {
            name: name.into(),
            frames,
            frame: 0,
        }
    }
}

impl From<AnimationSpec> for WidgetSpec {
    fn from(spec: AnimationSpec) -> Self {
        Self::Animation(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceMeterSpec {
    pub label: String,
    pub level: u8,
    pub recording: bool,
}

impl VoiceMeterSpec {
    pub fn new(label: impl Into<String>, level: u8) -> Self {
        Self {
            label: label.into(),
            level: level.min(100),
            recording: false,
        }
    }
}

impl From<VoiceMeterSpec> for WidgetSpec {
    fn from(spec: VoiceMeterSpec) -> Self {
        Self::VoiceMeter(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FooterSurfaceSpec {
    pub left: Vec<String>,
    pub right: Vec<String>,
    pub mode: Option<String>,
    pub goal: Option<String>,
}
