//! Backend-neutral widget specifications.
//!
//! These structs describe the shared widget state that backend crates render.
//! `display-tui` owns the terminal rendering, `display-wgpu` can render the same
//! specs graphically, and core crates can build specs without depending on a
//! concrete backend.

use serde::{Deserialize, Serialize};

use crate::surfaces::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WidgetSpec {
    Text(TextSpec),
    Box(BoxSpec),
    Spacer(SpacerSpec),
    TruncatedText(TruncatedTextSpec),
    Input(InputSpec),
    Loader(LoaderSpec),
    CancellableLoader(CancellableLoaderSpec),
    SelectList(SelectListSpec),
    SettingsList(SettingsListSpec),
    Markdown(MarkdownSpec),
    Editor(EditorSpec),
    Image(ImageSpec),
    TabBar(TabBarSpec),
    MessageFrame(MessageFrameSpec),
    AssistantMessage(AssistantMessageSpec),
    UserMessage(UserMessageSpec),
    ToolExecution(ToolExecutionSpec),
    BashExecution(BashExecutionSpec),
    Diff(DiffSpec),
    StatusLine(StatusLineSpec),
    Footer(FooterSpec),
    KeybindingHints(KeybindingHintsSpec),
    ModelSelector(SelectorSpec),
    SessionSelector(SelectorSpec),
    SettingsSelector(SelectorSpec),
    ThemeSelector(SelectorSpec),
    TreeSelector(TreeSelectorSpec),
    LoginDialog(LoginDialogSpec),
    HistorySearch(HistorySearchSpec),
    CountdownTimer(CountdownTimerSpec),
    TodoReminder(TodoReminderSpec),
    Welcome(WelcomeSpec),
    BorderedLoader(BorderedLoaderSpec),
    DynamicBorder(DynamicBorderSpec),
    VisualTruncate(VisualTruncateSpec),
    ChatComposer(Box<ChatComposerSpec>),
    TextArea(TextAreaSpec),
    SelectionPopup(SelectionPopupSpec),
    MultiSelectPicker(MultiSelectPickerSpec),
    ApprovalOverlay(ApprovalOverlaySpec),
    RequestUserInputOverlay(RequestUserInputOverlaySpec),
    McpElicitationOverlay(McpElicitationOverlaySpec),
    PendingInputPreview(PendingInputPreviewSpec),
    PendingThreadApprovals(PendingThreadApprovalsSpec),
    AppLinkView(AppLinkViewSpec),
    FeedbackView(FeedbackViewSpec),
    HistoryCell(HistoryCellSpec),
    Transcript(TranscriptSpec),
    UnifiedExec(UnifiedExecSpec),
    McpToolCall(McpToolCallSpec),
    PatchCell(PatchCellSpec),
    PlanCell(PlanCellSpec),
    HookCell(HookCellSpec),
    WebSearchCell(WebSearchCellSpec),
    SessionInfoCell(SessionInfoCellSpec),
    StatusIndicator(StatusIndicatorSpec),
    StatusSurface(StatusSurfaceSpec),
    TokenUsage(TokenUsageSpec),
    MenuSurface(MenuSurfaceSpec),
    NavigationOverlay(NavigationOverlaySpec),
    SessionPicker(SessionPickerSpec),
    SetupScreen(SetupScreenSpec),
    IntegrationView(IntegrationViewSpec),
    TerminalHyperlink(TerminalHyperlinkSpec),
    MarkdownStream(MarkdownStreamSpec),
    Animation(AnimationSpec),
    VoiceMeter(VoiceMeterSpec),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextSpec {
    pub text: String,
    pub padding_x: u16,
    pub padding_y: u16,
}

impl TextSpec {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            padding_x: 1,
            padding_y: 1,
        }
    }

    pub fn padding(mut self, x: u16, y: u16) -> Self {
        self.padding_x = x;
        self.padding_y = y;
        self
    }
}

impl From<TextSpec> for WidgetSpec {
    fn from(spec: TextSpec) -> Self {
        Self::Text(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoxSpec {
    pub children: Vec<WidgetSpec>,
    pub padding_x: u16,
    pub padding_y: u16,
}

impl BoxSpec {
    pub fn new(children: Vec<WidgetSpec>) -> Self {
        Self {
            children,
            padding_x: 1,
            padding_y: 1,
        }
    }

    pub fn padding(mut self, x: u16, y: u16) -> Self {
        self.padding_x = x;
        self.padding_y = y;
        self
    }
}

impl From<BoxSpec> for WidgetSpec {
    fn from(spec: BoxSpec) -> Self {
        Self::Box(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpacerSpec {
    pub lines: u16,
}

impl SpacerSpec {
    pub fn new(lines: u16) -> Self {
        Self { lines }
    }
}

impl From<SpacerSpec> for WidgetSpec {
    fn from(spec: SpacerSpec) -> Self {
        Self::Spacer(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TruncatedTextSpec {
    pub text: String,
    pub padding_x: u16,
    pub padding_y: u16,
}

impl TruncatedTextSpec {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            padding_x: 0,
            padding_y: 0,
        }
    }

    pub fn padding(mut self, x: u16, y: u16) -> Self {
        self.padding_x = x;
        self.padding_y = y;
        self
    }
}

impl From<TruncatedTextSpec> for WidgetSpec {
    fn from(spec: TruncatedTextSpec) -> Self {
        Self::TruncatedText(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputSpec {
    pub value: String,
    pub cursor: usize,
    pub focused: bool,
    pub prompt: String,
}

impl InputSpec {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        Self {
            cursor: value.len(),
            value,
            focused: false,
            prompt: "> ".to_string(),
        }
    }

    pub fn cursor(mut self, cursor: usize) -> Self {
        self.cursor = cursor.min(self.value.len());
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self
    }
}

impl From<InputSpec> for WidgetSpec {
    fn from(spec: InputSpec) -> Self {
        Self::Input(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoaderSpec {
    pub message: String,
    pub frame: usize,
    pub frames: Vec<String>,
}

impl LoaderSpec {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            frame: 0,
            frames: default_spinner_frames(),
        }
    }

    pub fn frame(mut self, frame: usize) -> Self {
        self.frame = frame;
        self
    }

    pub fn frames(mut self, frames: Vec<String>) -> Self {
        self.frames = frames;
        self
    }
}

impl From<LoaderSpec> for WidgetSpec {
    fn from(spec: LoaderSpec) -> Self {
        Self::Loader(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancellableLoaderSpec {
    pub loader: LoaderSpec,
    pub cancel_hint: String,
}

impl CancellableLoaderSpec {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            loader: LoaderSpec::new(message),
            cancel_hint: "Esc to cancel".to_string(),
        }
    }

    pub fn frame(mut self, frame: usize) -> Self {
        self.loader.frame = frame;
        self
    }

    pub fn cancel_hint(mut self, hint: impl Into<String>) -> Self {
        self.cancel_hint = hint.into();
        self
    }
}

impl From<CancellableLoaderSpec> for WidgetSpec {
    fn from(spec: CancellableLoaderSpec) -> Self {
        Self::CancellableLoader(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectItemSpec {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

impl SelectItemSpec {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            description: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectListSpec {
    pub items: Vec<SelectItemSpec>,
    pub selected: usize,
    pub max_visible: usize,
    pub min_primary_column_width: usize,
    pub max_primary_column_width: usize,
}

impl SelectListSpec {
    pub fn new(items: Vec<SelectItemSpec>) -> Self {
        Self {
            items,
            selected: 0,
            max_visible: 5,
            min_primary_column_width: 32,
            max_primary_column_width: 32,
        }
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }

    pub fn max_visible(mut self, max_visible: usize) -> Self {
        self.max_visible = max_visible.max(1);
        self
    }
}

impl From<SelectListSpec> for WidgetSpec {
    fn from(spec: SelectListSpec) -> Self {
        Self::SelectList(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingItemSpec {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub current_value: String,
    pub values: Vec<String>,
}

impl SettingItemSpec {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        current_value: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
            current_value: current_value.into(),
            values: Vec::new(),
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn values(mut self, values: Vec<String>) -> Self {
        self.values = values;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsListSpec {
    pub items: Vec<SettingItemSpec>,
    pub selected: usize,
    pub max_visible: usize,
    pub show_hint: bool,
    pub hint: String,
}

impl SettingsListSpec {
    pub fn new(items: Vec<SettingItemSpec>) -> Self {
        Self {
            items,
            selected: 0,
            max_visible: 8,
            show_hint: true,
            hint: "Enter/Space to change · Esc to cancel".to_string(),
        }
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }

    pub fn max_visible(mut self, max_visible: usize) -> Self {
        self.max_visible = max_visible.max(1);
        self
    }
}

impl From<SettingsListSpec> for WidgetSpec {
    fn from(spec: SettingsListSpec) -> Self {
        Self::SettingsList(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownSpec {
    pub text: String,
    pub padding_x: u16,
    pub padding_y: u16,
}

impl MarkdownSpec {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            padding_x: 1,
            padding_y: 0,
        }
    }

    pub fn padding(mut self, x: u16, y: u16) -> Self {
        self.padding_x = x;
        self.padding_y = y;
        self
    }
}

impl From<MarkdownSpec> for WidgetSpec {
    fn from(spec: MarkdownSpec) -> Self {
        Self::Markdown(spec)
    }
}

fn default_spinner_frames() -> Vec<String> {
    ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
        .into_iter()
        .map(String::from)
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorSpec {
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub scroll_top: usize,
    pub height: u16,
    pub show_gutter: bool,
    pub focused: bool,
}

impl EditorSpec {
    pub fn new(lines: Vec<String>) -> Self {
        Self {
            lines,
            cursor_line: 0,
            cursor_col: 0,
            scroll_top: 0,
            height: 12,
            show_gutter: true,
            focused: false,
        }
    }

    pub fn cursor(mut self, line: usize, col: usize) -> Self {
        self.cursor_line = line;
        self.cursor_col = col;
        self
    }

    pub fn scroll_top(mut self, scroll_top: usize) -> Self {
        self.scroll_top = scroll_top;
        self
    }

    pub fn height(mut self, height: u16) -> Self {
        self.height = height;
        self
    }

    pub fn gutter(mut self, show: bool) -> Self {
        self.show_gutter = show;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl From<EditorSpec> for WidgetSpec {
    fn from(spec: EditorSpec) -> Self {
        Self::Editor(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageSpec {
    pub frame_id: String,
    pub width: u16,
    pub height: u16,
    pub alt: Option<String>,
}

impl ImageSpec {
    pub fn new(frame_id: impl Into<String>, width: u16, height: u16) -> Self {
        Self {
            frame_id: frame_id.into(),
            width,
            height,
            alt: None,
        }
    }

    pub fn alt(mut self, alt: impl Into<String>) -> Self {
        self.alt = Some(alt.into());
        self
    }
}

impl From<ImageSpec> for WidgetSpec {
    fn from(spec: ImageSpec) -> Self {
        Self::Image(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabItemSpec {
    pub id: String,
    pub label: String,
    pub modified: bool,
}

impl TabItemSpec {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            modified: false,
        }
    }

    pub fn modified(mut self, modified: bool) -> Self {
        self.modified = modified;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabBarSpec {
    pub items: Vec<TabItemSpec>,
    pub active: usize,
}

impl TabBarSpec {
    pub fn new(items: Vec<TabItemSpec>) -> Self {
        Self { items, active: 0 }
    }

    pub fn active(mut self, active: usize) -> Self {
        self.active = active;
        self
    }
}

impl From<TabBarSpec> for WidgetSpec {
    fn from(spec: TabBarSpec) -> Self {
        Self::TabBar(spec)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRoleSpec {
    Assistant,
    User,
    System,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageFrameSpec {
    pub title: String,
    pub role: MessageRoleSpec,
    pub body: Vec<WidgetSpec>,
}

impl MessageFrameSpec {
    pub fn new(title: impl Into<String>, role: MessageRoleSpec, body: Vec<WidgetSpec>) -> Self {
        Self {
            title: title.into(),
            role,
            body,
        }
    }
}

impl From<MessageFrameSpec> for WidgetSpec {
    fn from(spec: MessageFrameSpec) -> Self {
        Self::MessageFrame(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantMessageSpec {
    pub text: String,
    pub model: Option<String>,
}

impl AssistantMessageSpec {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            model: None,
        }
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

impl From<AssistantMessageSpec> for WidgetSpec {
    fn from(spec: AssistantMessageSpec) -> Self {
        Self::AssistantMessage(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserMessageSpec {
    pub text: String,
}

impl UserMessageSpec {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl From<UserMessageSpec> for WidgetSpec {
    fn from(spec: UserMessageSpec) -> Self {
        Self::UserMessage(spec)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatusSpec {
    Pending,
    Running,
    Success,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolExecutionSpec {
    pub title: String,
    pub status: ExecutionStatusSpec,
    pub command: Option<String>,
    pub output: Vec<String>,
}

impl ToolExecutionSpec {
    pub fn new(title: impl Into<String>, status: ExecutionStatusSpec) -> Self {
        Self {
            title: title.into(),
            status,
            command: None,
            output: Vec::new(),
        }
    }

    pub fn command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    pub fn output(mut self, output: Vec<String>) -> Self {
        self.output = output;
        self
    }
}

impl From<ToolExecutionSpec> for WidgetSpec {
    fn from(spec: ToolExecutionSpec) -> Self {
        Self::ToolExecution(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BashExecutionSpec {
    pub command: String,
    pub status: ExecutionStatusSpec,
    pub exit_code: Option<i32>,
    pub output: Vec<String>,
}

impl BashExecutionSpec {
    pub fn new(command: impl Into<String>, status: ExecutionStatusSpec) -> Self {
        Self {
            command: command.into(),
            status,
            exit_code: None,
            output: Vec::new(),
        }
    }

    pub fn exit_code(mut self, exit_code: i32) -> Self {
        self.exit_code = Some(exit_code);
        self
    }

    pub fn output(mut self, output: Vec<String>) -> Self {
        self.output = output;
        self
    }
}

impl From<BashExecutionSpec> for WidgetSpec {
    fn from(spec: BashExecutionSpec) -> Self {
        Self::BashExecution(spec)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffLineKindSpec {
    Context,
    Added,
    Removed,
    Hunk,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffLineSpec {
    pub kind: DiffLineKindSpec,
    pub text: String,
}

impl DiffLineSpec {
    pub fn new(kind: DiffLineKindSpec, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffSpec {
    pub title: String,
    pub lines: Vec<DiffLineSpec>,
}

impl DiffSpec {
    pub fn new(title: impl Into<String>, lines: Vec<DiffLineSpec>) -> Self {
        Self {
            title: title.into(),
            lines,
        }
    }
}

impl From<DiffSpec> for WidgetSpec {
    fn from(spec: DiffSpec) -> Self {
        Self::Diff(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusSegmentSpec {
    pub label: String,
    pub value: Option<String>,
}

impl StatusSegmentSpec {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: None,
        }
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusLineSpec {
    pub left: Vec<StatusSegmentSpec>,
    pub right: Vec<StatusSegmentSpec>,
}

impl StatusLineSpec {
    pub fn new(left: Vec<StatusSegmentSpec>, right: Vec<StatusSegmentSpec>) -> Self {
        Self { left, right }
    }
}

impl From<StatusLineSpec> for WidgetSpec {
    fn from(spec: StatusLineSpec) -> Self {
        Self::StatusLine(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FooterSpec {
    pub items: Vec<String>,
}

impl FooterSpec {
    pub fn new(items: Vec<String>) -> Self {
        Self { items }
    }
}

impl From<FooterSpec> for WidgetSpec {
    fn from(spec: FooterSpec) -> Self {
        Self::Footer(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeybindingHintSpec {
    pub key: String,
    pub label: String,
}

impl KeybindingHintSpec {
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeybindingHintsSpec {
    pub hints: Vec<KeybindingHintSpec>,
}

impl KeybindingHintsSpec {
    pub fn new(hints: Vec<KeybindingHintSpec>) -> Self {
        Self { hints }
    }
}

impl From<KeybindingHintsSpec> for WidgetSpec {
    fn from(spec: KeybindingHintsSpec) -> Self {
        Self::KeybindingHints(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectorOptionSpec {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
}

impl SelectorOptionSpec {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectorSpec {
    pub title: String,
    pub options: Vec<SelectorOptionSpec>,
    pub selected: usize,
    pub max_visible: usize,
}

impl SelectorSpec {
    pub fn new(title: impl Into<String>, options: Vec<SelectorOptionSpec>) -> Self {
        Self {
            title: title.into(),
            options,
            selected: 0,
            max_visible: 8,
        }
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }

    pub fn max_visible(mut self, max_visible: usize) -> Self {
        self.max_visible = max_visible.max(1);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeNodeSpec {
    pub id: String,
    pub label: String,
    pub children: Vec<TreeNodeSpec>,
    pub expanded: bool,
}

impl TreeNodeSpec {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            children: Vec::new(),
            expanded: false,
        }
    }

    pub fn children(mut self, children: Vec<TreeNodeSpec>) -> Self {
        self.children = children;
        self
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeSelectorSpec {
    pub title: String,
    pub roots: Vec<TreeNodeSpec>,
    pub selected_path: Vec<usize>,
}

impl TreeSelectorSpec {
    pub fn new(title: impl Into<String>, roots: Vec<TreeNodeSpec>) -> Self {
        Self {
            title: title.into(),
            roots,
            selected_path: Vec::new(),
        }
    }

    pub fn selected_path(mut self, path: Vec<usize>) -> Self {
        self.selected_path = path;
        self
    }
}

impl From<TreeSelectorSpec> for WidgetSpec {
    fn from(spec: TreeSelectorSpec) -> Self {
        Self::TreeSelector(spec)
    }
}

impl From<SelectorSpec> for WidgetSpec {
    fn from(spec: SelectorSpec) -> Self {
        Self::ModelSelector(spec)
    }
}

impl SelectorSpec {
    pub fn into_model_selector(self) -> WidgetSpec {
        WidgetSpec::ModelSelector(self)
    }

    pub fn into_session_selector(self) -> WidgetSpec {
        WidgetSpec::SessionSelector(self)
    }

    pub fn into_settings_selector(self) -> WidgetSpec {
        WidgetSpec::SettingsSelector(self)
    }

    pub fn into_theme_selector(self) -> WidgetSpec {
        WidgetSpec::ThemeSelector(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginDialogSpec {
    pub provider: String,
    pub verification_uri: String,
    pub user_code: String,
    pub status: String,
}

impl LoginDialogSpec {
    pub fn new(
        provider: impl Into<String>,
        verification_uri: impl Into<String>,
        user_code: impl Into<String>,
    ) -> Self {
        Self {
            provider: provider.into(),
            verification_uri: verification_uri.into(),
            user_code: user_code.into(),
            status: "Waiting for browser login".to_string(),
        }
    }

    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self
    }
}

impl From<LoginDialogSpec> for WidgetSpec {
    fn from(spec: LoginDialogSpec) -> Self {
        Self::LoginDialog(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistorySearchSpec {
    pub query: String,
    pub results: Vec<String>,
    pub selected: usize,
}

impl HistorySearchSpec {
    pub fn new(query: impl Into<String>, results: Vec<String>) -> Self {
        Self {
            query: query.into(),
            results,
            selected: 0,
        }
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }
}

impl From<HistorySearchSpec> for WidgetSpec {
    fn from(spec: HistorySearchSpec) -> Self {
        Self::HistorySearch(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountdownTimerSpec {
    pub label: String,
    pub remaining_seconds: u64,
}

impl CountdownTimerSpec {
    pub fn new(label: impl Into<String>, remaining_seconds: u64) -> Self {
        Self {
            label: label.into(),
            remaining_seconds,
        }
    }
}

impl From<CountdownTimerSpec> for WidgetSpec {
    fn from(spec: CountdownTimerSpec) -> Self {
        Self::CountdownTimer(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoReminderSpec {
    pub items: Vec<String>,
}

impl TodoReminderSpec {
    pub fn new(items: Vec<String>) -> Self {
        Self { items }
    }
}

impl From<TodoReminderSpec> for WidgetSpec {
    fn from(spec: TodoReminderSpec) -> Self {
        Self::TodoReminder(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WelcomeSpec {
    pub title: String,
    pub subtitle: String,
    pub actions: Vec<String>,
}

impl WelcomeSpec {
    pub fn new(title: impl Into<String>, subtitle: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: subtitle.into(),
            actions: Vec::new(),
        }
    }

    pub fn actions(mut self, actions: Vec<String>) -> Self {
        self.actions = actions;
        self
    }
}

impl From<WelcomeSpec> for WidgetSpec {
    fn from(spec: WelcomeSpec) -> Self {
        Self::Welcome(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BorderedLoaderSpec {
    pub title: String,
    pub loader: LoaderSpec,
}

impl BorderedLoaderSpec {
    pub fn new(title: impl Into<String>, loader: LoaderSpec) -> Self {
        Self {
            title: title.into(),
            loader,
        }
    }
}

impl From<BorderedLoaderSpec> for WidgetSpec {
    fn from(spec: BorderedLoaderSpec) -> Self {
        Self::BorderedLoader(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DynamicBorderSpec {
    pub title: String,
    pub active: bool,
    pub child: Box<WidgetSpec>,
}

impl DynamicBorderSpec {
    pub fn new(title: impl Into<String>, child: WidgetSpec) -> Self {
        Self {
            title: title.into(),
            active: false,
            child: Box::new(child),
        }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

impl From<DynamicBorderSpec> for WidgetSpec {
    fn from(spec: DynamicBorderSpec) -> Self {
        Self::DynamicBorder(spec)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualTruncateSpec {
    pub text: String,
    pub suffix: String,
}

impl VisualTruncateSpec {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            suffix: "…".to_string(),
        }
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }
}

impl From<VisualTruncateSpec> for WidgetSpec {
    fn from(spec: VisualTruncateSpec) -> Self {
        Self::VisualTruncate(spec)
    }
}
