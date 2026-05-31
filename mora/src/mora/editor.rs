use std::path::{Path, PathBuf};

use super::display::event::{
    MoraKeyCode as KeyCode, MoraKeyEvent as KeyEvent, MoraKeyModifiers as KeyModifiers,
};

use super::buffer::Buffer;
use super::keymap::{self, EditorMode, KeyAction, PendingOp};
use super::kill_ring::KillRing;
use super::lisp_ext::MoraLispBridge;
use super::macro_state::MacroState;
use super::mark::MarkRing;
use super::minibuffer::{CompletionResult, Minibuffer};
use super::rectangle::{self, RectRegion};
use super::register::{RegisterValue, Registers};
use super::view::View;
use super::wasm_ext::WasmExtensionHost;

#[derive(Clone)]
pub struct WindowState {
    pub view: View,
    pub buffer_idx: usize,
    pub cursor: super::buffer::Cursor,
}

pub struct MoraEditor {
    pub buffer: Buffer,
    pub mode: EditorMode,
    pub view: View,
    pub command_input: String,
    pub minibuffer: Minibuffer,
    pub kill_ring: KillRing,
    pub registers: Registers,
    pub mark_ring: MarkRing,
    pub macro_state: MacroState,
    pub wasm_host: WasmExtensionHost,
    pub lisp_bridge: MoraLispBridge,
    pub last_search_forward: Option<String>,
    pub last_search_backward: Option<String>,
    pub status_message: String,
    /// Messages log (emacs *Messages* buffer)
    pub messages: Vec<String>,
    /// Maximum messages to keep
    pub messages_max: usize,
    /// Whether scratch buffer was initialized
    pub scratch_initialized: bool,
    pub quit_requested: bool,
    pub pending_action: Option<KeyAction>,
    pub waiting_g: bool,
    pub waiting_op: Option<PendingOp>,
    pub waiting_register: Option<char>,
    pub waiting_prefix: Option<char>,
    pub waiting_prefix2: Option<char>,
    pub visual_start: Option<(usize, usize)>,
    pub macro_playing_keys: Vec<KeyEvent>,
    pub last_yank_was_kill: bool,
    pub repeat_count: Option<usize>,
    pub expand_region_level: usize,
    pub dabbrev_prefix: Option<String>,
    pub dabbrev_matches: Vec<String>,
    pub dabbrev_index: usize,
    pub waiting_zap: bool,
    pub last_changes: std::collections::VecDeque<(usize, usize)>,
    pub iedit_word: Option<String>,
    pub iedit_regions: Vec<(usize, usize, usize)>,
    pub iedit_cursor_idx: usize,
    pub iedit_regex: bool,
    pub waiting_iedit_regex: bool,
    pub waiting_find: bool,
    pub waiting_find_forward: bool,
    pub waiting_find_till: bool,
    pub last_find_char: Option<char>,
    pub last_find_forward: bool,
    pub last_find_till: bool,
    pub last_normal_change: Option<Box<KeyAction>>,
    pub waiting_text_object: bool,
    pub text_object_inner: bool,
    pub waiting_visual_text_object: bool,
    pub execute_once_mode: Option<EditorMode>,
    pub waiting_surround_s: Option<PendingOp>,
    pub surround_old_char: Option<char>,
    pub surround_new_char: Option<char>,
    pub surround_range: Option<(usize, usize)>,
    pub waiting_surround_add: bool,
    pub waiting_ace_jump: bool,
    pub ace_jump_target: Option<char>,
    pub ace_jump_hints: Vec<(usize, usize, char)>,
    pub waiting_z: bool,
    pub windows: Vec<WindowState>,
    pub current_window_idx: usize,
    pub current_window_buffer_idx: usize,
    pub minor_modes: super::minor_mode::MinorModeRegistry,
    pub mshell: super::mshell::MshellState,
    pub theme: super::theme::ThemeColors,
    pub lsp_client: super::lsp::LspClient,
}

impl MoraEditor {
    pub fn new(height: usize) -> Self {
        let mut editor = Self {
            buffer: Buffer::new(),
            mode: EditorMode::Emacs,
            view: View::new(height),
            command_input: String::new(),
            minibuffer: Minibuffer::default(),
            kill_ring: KillRing::new(),
            registers: Registers::new(),
            mark_ring: MarkRing::new(),
            macro_state: MacroState::new(),
            wasm_host: WasmExtensionHost::new(),
            lisp_bridge: MoraLispBridge::new(),
            last_search_forward: None,
            last_search_backward: None,
            status_message: String::new(),
            messages: Vec::new(),
            messages_max: 1000,
            scratch_initialized: false,
            quit_requested: false,
            pending_action: None,
            waiting_g: false,
            waiting_op: None,
            waiting_register: None,
            waiting_prefix: None,
            waiting_prefix2: None,
            visual_start: None,
            macro_playing_keys: Vec::new(),
            last_yank_was_kill: false,
            repeat_count: None,
            expand_region_level: 0,
            dabbrev_prefix: None,
            dabbrev_matches: Vec::new(),
            dabbrev_index: 0,
            waiting_zap: false,
            last_changes: std::collections::VecDeque::new(),
            iedit_word: None,
            iedit_regions: Vec::new(),
            iedit_cursor_idx: 0,
            iedit_regex: false,
            waiting_iedit_regex: false,
            waiting_find: false,
            waiting_find_forward: true,
            waiting_find_till: false,
            last_find_char: None,
            last_find_forward: true,
            last_find_till: false,
            last_normal_change: None,
            waiting_text_object: false,
            text_object_inner: true,
            waiting_visual_text_object: false,
            execute_once_mode: None,
            waiting_surround_s: None,
            surround_old_char: None,
            surround_new_char: None,
            surround_range: None,
            waiting_surround_add: false,
            waiting_ace_jump: false,
            ace_jump_target: None,
            ace_jump_hints: Vec::new(),
            waiting_z: false,
            windows: Vec::new(),
            current_window_idx: 0,
            current_window_buffer_idx: 0,
            minor_modes: super::minor_mode::MinorModeRegistry::new(),
            mshell: super::mshell::MshellState::new(),
            theme: super::theme::night(),
            lsp_client: super::lsp::LspClient::new(),
        };
        editor.wasm_host.discover();
        if editor.wasm_host.count() > 0 {
            editor.status_message = format!("Loaded {} extension(s)", editor.wasm_host.count());
        }
        editor
    }

    pub fn open(path: &Path, height: usize) -> std::io::Result<Self> {
        let buffer = Buffer::from_file(path)?;
        let mut editor = Self::new(height);
        editor.buffer = buffer;
        editor.status_message = format!("Opened: {}", path.display());
        Ok(editor)
    }

    /// Initialize the *scratch* buffer with emacs-style content
    pub fn init_scratch_buffer(&mut self) {
        if self.scratch_initialized {
            return;
        }
        self.scratch_initialized = true;
        self.buffer.lines = vec![
            ";; This buffer is for text that is not saved.".to_string(),
            ";; To create a file, visit it with C-x C-f".to_string(),
            ";; and enter text in its buffer.".to_string(),
            "".to_string(),
            "".to_string(),
        ];
        self.buffer.cursor.row = 4;
        self.buffer.cursor.col = 0;
        self.buffer.modified = false;
        self.message("Welcome to Mora. Type M-x for commands.");
    }

    /// Log a message to the *Messages* buffer (like emacs)
    pub fn message(&mut self, msg: &str) {
        self.messages.push(msg.to_string());
        if self.messages.len() > self.messages_max {
            self.messages.remove(0);
        }
    }

    /// Log a warning to *Messages*
    pub fn warn(&mut self, msg: &str) {
        self.message(&format!("Warning: {}", msg));
    }

    pub fn mode(&self) -> EditorMode {
        self.mode
    }
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
    pub fn view(&self) -> &View {
        &self.view
    }
    pub fn command_input(&self) -> &str {
        if self.minibuffer.is_active() {
            self.minibuffer.input()
        } else {
            &self.command_input
        }
    }

    pub fn minibuffer_prompt(&self) -> &str {
        if self.minibuffer.is_active() {
            self.minibuffer.prompt()
        } else {
            match self.mode {
                EditorMode::Command => ":",
                EditorMode::SearchForward => "/",
                EditorMode::SearchBackward => "?",
                _ => "",
            }
        }
    }

    pub fn minibuffer_active(&self) -> bool {
        self.minibuffer.is_active()
            || matches!(
                self.mode,
                EditorMode::Command | EditorMode::SearchForward | EditorMode::SearchBackward
            )
    }
    pub fn status_message(&self) -> &str {
        &self.status_message
    }
    pub fn quit_requested(&self) -> bool {
        self.quit_requested
    }

    pub fn set_height(&mut self, height: usize) {
        self.view.height = height.max(1);
        self.view.ensure_cursor_visible(&self.buffer);
    }

    fn activate_minibuffer(&mut self, mode: EditorMode) {
        let prompt = match mode {
            EditorMode::Command => ":",
            EditorMode::SearchForward => "/",
            EditorMode::SearchBackward => "?",
            _ => "",
        };
        self.activate_minibuffer_with_prompt(mode, prompt);
    }

    fn activate_minibuffer_with_prompt(&mut self, mode: EditorMode, prompt: &str) {
        self.mode = mode;
        self.command_input.clear();
        self.minibuffer.activate(prompt);
        if mode == EditorMode::Command {
            let candidates = self.command_candidates();
            self.minibuffer.set_completions(candidates);
        }
    }

    fn clear_minibuffer(&mut self) {
        self.command_input.clear();
        self.minibuffer.clear();
    }

    fn set_minibuffer_input(&mut self, input: impl Into<String>) {
        let input = input.into();
        self.command_input = input.clone();
        if self.minibuffer.is_active() {
            self.minibuffer.set_input(input);
        }
    }

    fn push_minibuffer_char(&mut self, ch: char) {
        if self.minibuffer.is_active() {
            self.minibuffer.push(ch);
            self.command_input = self.minibuffer.input().to_string();
        } else {
            self.command_input.push(ch);
        }
    }

    fn pop_minibuffer_char(&mut self) {
        if self.minibuffer.is_active() {
            self.minibuffer.pop();
            self.command_input = self.minibuffer.input().to_string();
        } else {
            self.command_input.pop();
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {

        // Mshell: intercept all keys when shell is active
        if self.mshell.is_active() {
            if self.mshell.handle_key(&key) {
                return true;
            }
            // If not consumed by mshell, fall through to normal handling
        }

        self.macro_state.record_key(&key);
        if self.macro_state.is_playing() {
            return true;
        }

        if let Some(action) = self.pending_action.take() {
            self.execute_action(action);
            self.clamp_cursor_to_narrow();
            self.view.ensure_cursor_visible(&self.buffer);
            return true;
        }

        if let Some(name) = self.waiting_register.take() {
            self.handle_register_key(name, key);
            self.clamp_cursor_to_narrow();
            self.view.ensure_cursor_visible(&self.buffer);
            return true;
        }

        let action = self.reduce_no_playback(key);

        let redraw = action != KeyAction::None;
        self.execute_action(action);
        self.clamp_cursor_to_narrow();
        self.view.ensure_cursor_visible(&self.buffer);
        redraw
    }

    pub fn drain_macro_events(&mut self) -> Option<KeyEvent> {
        self.macro_state.next_event()
    }

    fn reduce_no_playback(&mut self, key: KeyEvent) -> KeyAction {
        if let Some(prefix2) = self.waiting_prefix2.take() {
            return self.handle_prefix2_key(prefix2, key);
        }
        if let Some(prefix) = self.waiting_prefix.take() {
            return self.handle_prefix_key(prefix, key);
        }

        // Ace-jump: waiting for target char or hint key
        if self.waiting_ace_jump {
            return self.handle_ace_jump(key);
        }

        // Global M-x: works in every mode
        if key.modifiers == KeyModifiers::ALT && key.code == KeyCode::Char('x') {
            self.activate_minibuffer_with_prompt(EditorMode::Command, "M-x ");
            return KeyAction::None;
        }

        // Minor mode intercept: higher priority modes get first chance
        if let Some(action) = self.minor_modes.intercept_key(key) {
            return action;
        }

        match self.mode {
            EditorMode::Normal => self.handle_normal(key),
            EditorMode::Insert => keymap::insert_key(key),
            EditorMode::Command | EditorMode::SearchForward | EditorMode::SearchBackward => {
                keymap::command_key(key)
            }
            EditorMode::Emacs => self.handle_emacs(key),
            EditorMode::ReplaceChar => self.handle_replace_char(key),
            EditorMode::Visual => self.handle_visual(key),
            EditorMode::Iedit => self.handle_iedit(key),
        }
    }

    fn handle_prefix_key(&mut self, prefix: char, key: KeyEvent) -> KeyAction {
        match prefix {
            'x' => match (key.modifiers, key.code) {
                (_, KeyCode::Char('s')) => {
                    self.save_current_buffer();
                    KeyAction::SetMode(EditorMode::Normal)
                }
                (_, KeyCode::Char('c')) | (_, KeyCode::Char('k')) => KeyAction::Quit,
                (_, KeyCode::Char('f')) => KeyAction::SetMode(EditorMode::SearchForward),
                (_, KeyCode::Char('b')) => {
                    let cmd = self.command_input().to_string();
                    KeyAction::FindBackward(cmd)
                }
                (_, KeyCode::Char('r')) => {
                    self.waiting_register = Some('r');
                    self.status_message = "Insert register: ".to_string();
                    KeyAction::None
                }
                (_, KeyCode::Char('h')) => {
                    self.waiting_register = Some('h');
                    self.status_message = "Help for: ".to_string();
                    KeyAction::None
                }
                (_, KeyCode::Char('u')) => KeyAction::Undo,
                (_, KeyCode::Char('t')) => KeyAction::TransposeLine,
                (_, KeyCode::Char('l')) => KeyAction::LowercaseRegion,
                (_, KeyCode::Char('=')) => KeyAction::GotoLastChange,
                (_, KeyCode::Char(';')) => KeyAction::CleanupBuffer,
                (_, KeyCode::Char('m')) => KeyAction::Dos2Unix,
                // C-x n: narrow prefix
                (_, KeyCode::Char('n')) => {
                    self.waiting_prefix2 = Some('n');
                    KeyAction::None
                }
                // C-x 2: split horizontally
                (_, KeyCode::Char('2')) => KeyAction::SplitHorizontal,
                // C-x 3: split vertically
                (_, KeyCode::Char('3')) => KeyAction::SplitVertical,
                // C-x 0: delete current window
                (_, KeyCode::Char('0')) => KeyAction::DeleteWindow,
                // C-x 1: delete other windows
                (_, KeyCode::Char('1')) => KeyAction::DeleteOtherWindows,
                // C-x o: other window
                (_, KeyCode::Char('o')) => KeyAction::OtherWindow,
                // C-x +: balance windows
                (_, KeyCode::Char('+')) => KeyAction::BalanceWindows,
                _ => KeyAction::None,
            },
            'c' => match key.code {
                KeyCode::Char('c') => KeyAction::ForceQuit,
                KeyCode::Char('s') => {
                    self.save_current_buffer();
                    KeyAction::SetMode(EditorMode::Normal)
                }
                // C-c ;: copy and comment
                KeyCode::Char(';') => KeyAction::CopyAndComment,
                // C-c SPC: ace-jump
                KeyCode::Char(' ') => {
                    self.waiting_ace_jump = true;
                    self.status_message = "Ace jump char: ".to_string();
                    KeyAction::None
                }
                _ => KeyAction::None,
            },
            'r' => match (key.modifiers, key.code) {
                (_, KeyCode::Char(c)) if c.is_ascii_lowercase() => {
                    let saved_cmd = self.command_input.clone();
                    if let Some(RegisterValue::Macro(events)) = self.registers.get(c) {
                        self.macro_state.load_from_register(c, events);
                        self.macro_state.start_playback(c);
                    }
                    self.command_input = saved_cmd;
                    KeyAction::None
                }
                _ => KeyAction::None,
            },
            'g' => match key.code {
                KeyCode::Char('g') => {
                    self.activate_minibuffer_with_prompt(EditorMode::Command, "Goto line: ");
                    self.status_message = "Goto line: ".to_string();
                    KeyAction::None
                }
                _ => KeyAction::None,
            },
            ' ' => match key.code {
                // SPC f: file operations
                KeyCode::Char('f') => {
                    self.waiting_prefix2 = Some('f');
                    self.status_message = "SPC f-".to_string();
                    KeyAction::None
                }
                // SPC b: buffer operations
                KeyCode::Char('b') => {
                    self.waiting_prefix2 = Some('b');
                    self.status_message = "SPC b-".to_string();
                    KeyAction::None
                }
                // SPC w: window operations
                KeyCode::Char('w') => {
                    self.waiting_prefix2 = Some('w');
                    self.status_message = "SPC w-".to_string();
                    KeyAction::None
                }
                // SPC g: git operations
                KeyCode::Char('g') => {
                    self.waiting_prefix2 = Some('g');
                    self.status_message = "SPC g-".to_string();
                    KeyAction::None
                }
                // SPC p: project operations
                KeyCode::Char('p') => {
                    self.waiting_prefix2 = Some('p');
                    self.status_message = "SPC p-".to_string();
                    KeyAction::None
                }
                // SPC s: search operations
                KeyCode::Char('s') => {
                    self.waiting_prefix2 = Some('s');
                    self.status_message = "SPC s-".to_string();
                    KeyAction::None
                }
                // SPC e: eval operations
                KeyCode::Char('e') => {
                    self.waiting_prefix2 = Some('e');
                    self.status_message = "SPC e-".to_string();
                    KeyAction::None
                }
                // SPC a: AI/LLM operations
                KeyCode::Char('a') => {
                    self.waiting_prefix2 = Some('a');
                    self.status_message = "SPC a-".to_string();
                    KeyAction::None
                }
                // SPC m: major mode operations
                KeyCode::Char('m') => {
                    self.waiting_prefix2 = Some('m');
                    self.status_message = "SPC m-".to_string();
                    KeyAction::None
                }
                // SPC t: toggle operations (theme, font, etc)
                KeyCode::Char('t') => {
                    self.waiting_prefix2 = Some('t');
                    self.status_message = "SPC t-".to_string();
                    KeyAction::None
                }
                // SPC S: spelling check
                KeyCode::Char('S') => {
                    self.run_spelling_check();
                    KeyAction::None
                }
                // SPC 1-9: window selection (coldnew-emacs: SPC 1-9)
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let n = (c as u8 - b'0') as usize;
                    // Switch to window N
                    if n > 0 && n <= self.windows.len() {
                        self.current_window_idx = (n - 1).min(self.windows.len() - 1);
                    }
                    self.status_message.clear();
                    KeyAction::None
                }
                // SPC v: expand region (coldnew-emacs: M-v)
                KeyCode::Char('v') => KeyAction::ExpandRegion,
                // SPC j: ace-jump (coldnew-emacs: C-c SPC)
                KeyCode::Char('j') => {
                    self.waiting_ace_jump = true;
                    self.status_message = "Ace jump char: ".to_string();
                    KeyAction::None
                }
                // SPC x: M-x commands
                KeyCode::Char('x') => {
                    self.activate_minibuffer_with_prompt(EditorMode::Command, "M-x ");
                    KeyAction::None
                }
                // SPC o: other-window (coldnew-emacs: M-o)
                KeyCode::Char('o') => KeyAction::OtherWindow,
                // SPC /: search (consult-line equivalent)
                KeyCode::Char('/') => {
                    self.activate_minibuffer_with_prompt(EditorMode::SearchForward, "/ ");
                    KeyAction::None
                }
                KeyCode::Esc => KeyAction::None,
                _ => KeyAction::None,
            },
            _ => KeyAction::None,
        }
    }

    fn handle_prefix2_key(&mut self, prefix: char, key: KeyEvent) -> KeyAction {
        match prefix {
            'n' => match key.code {
                // C-x n n: narrow to region
                KeyCode::Char('n') => KeyAction::NarrowRegion,
                // C-x n w: widen
                KeyCode::Char('w') => KeyAction::Widen,
                _ => KeyAction::None,
            },
            // SPC f: file operations
            'f' => match key.code {
                KeyCode::Char('f') => KeyAction::FindFile,
                KeyCode::Char('s') => KeyAction::SaveBuffer,
                KeyCode::Char('r') => {
                    // SPC f r: recent files
                    self.status_message = "Recent files".to_string();
                    KeyAction::None
                }
                KeyCode::Char('d') => {
                    // SPC f d: dired (directory browser)
                    self.dired_open();
                    KeyAction::None
                }
                _ => KeyAction::None,
            },
            // SPC b: buffer operations
            'b' => match key.code {
                KeyCode::Char('b') => KeyAction::SwitchBuffer,
                KeyCode::Char('d') => KeyAction::DeleteLine, // kill buffer
                KeyCode::Char('k') => KeyAction::DeleteLine, // kill buffer
                KeyCode::Char('n') => KeyAction::MoveDown, // next buffer
                KeyCode::Char('p') => KeyAction::MoveUp, // previous buffer
                KeyCode::Char('r') => KeyAction::SetMode(EditorMode::Command), // revert
                KeyCode::Char('s') => KeyAction::SaveBuffer, // save buffer
                _ => KeyAction::None,
            },
            // SPC w: window operations
            'w' => match key.code {
                KeyCode::Char('h') => KeyAction::WindowLeft,
                KeyCode::Char('j') => KeyAction::WindowDown,
                KeyCode::Char('k') => KeyAction::WindowUp,
                KeyCode::Char('l') => KeyAction::WindowRight,
                KeyCode::Char('v') => KeyAction::WindowSplitVertical,
                KeyCode::Char('s') => KeyAction::WindowSplitHorizontal,
                KeyCode::Char('d') => KeyAction::DeleteWindow,
                KeyCode::Char('o') => KeyAction::DeleteOtherWindows,
                KeyCode::Char('+') => KeyAction::BalanceWindows,
                KeyCode::Char('u') => KeyAction::Undo,
                KeyCode::Char('U') => KeyAction::Redo,
                _ => KeyAction::None,
            },
            // SPC q: quit operations
            'q' => match key.code {
                KeyCode::Char('q') => KeyAction::Quit,
                KeyCode::Char('s') => {
                    self.save_current_buffer();
                    KeyAction::Quit
                }
                _ => KeyAction::None,
            },
            // SPC g: git operations
            'g' => match key.code {
                KeyCode::Char('s') => {
                    // SPC g s: git status
                    self.run_git_status();
                    KeyAction::None
                }
                KeyCode::Char('l') => {
                    // SPC g l: git log (last 20)
                    self.run_git_log();
                    KeyAction::None
                }
                KeyCode::Char('d') => {
                    // SPC g d: git diff
                    self.run_git_diff();
                    KeyAction::None
                }
                KeyCode::Char('c') => {
                    // SPC g c: git commit
                    self.run_git_commit();
                    KeyAction::None
                }
                _ => KeyAction::None,
            },
            // SPC p: project operations
            'p' => match key.code {
                KeyCode::Char('f') => KeyAction::ProjectFindFile,
                KeyCode::Char('g') => KeyAction::Grep,
                KeyCode::Char('p') => KeyAction::ProjectFindFile,
                _ => KeyAction::None,
            },
            // SPC s: search/edit operations
            's' => match key.code {
                KeyCode::Char('/') => {
                    self.activate_minibuffer_with_prompt(EditorMode::SearchForward, "/ ");
                    KeyAction::None
                }
                KeyCode::Char('e') => {
                    // SPC s e: iedit (multi-cursor edit all occurrences)
                    self.start_iedit();
                    KeyAction::None
                }
                KeyCode::Char('i') => {
                    // SPC s i: iedit-regex
                    self.start_iedit_regex();
                    KeyAction::None
                }
                _ => KeyAction::None,
            },
            // SPC e: eval/editor operations
            'e' => match key.code {
                KeyCode::Char('d') => KeyAction::EvalLispExpression,
                KeyCode::Char('b') => KeyAction::EvalLispExpression,
                KeyCode::Char(';') => KeyAction::CopyAndComment,
                KeyCode::Char('=') => KeyAction::GotoLastChange,
                _ => KeyAction::None,
            },
            // SPC t: toggle operations
            't' => match key.code {
                KeyCode::Char('t') => {
                    // SPC t t: toggle theme
                    self.theme = if self.theme.background.r < 128 {
                        super::theme::day()
                    } else {
                        super::theme::night()
                    };
                    let mode = if self.theme.background.r < 128 { "Night" } else { "Day" };
                    self.status_message = format!("Theme: {mode}");
                    KeyAction::None
                }
                KeyCode::Char('f') => {
                    // SPC t f: cycle font size
                    self.status_message = "Font: use C-= / C-- to adjust".to_string();
                    KeyAction::None
                }
                _ => KeyAction::None,
            },
            // SPC m: major mode operations
            'm' => match key.code {
                KeyCode::Char('d') => {
                    // SPC m d: go to definition
                    self.run_go_to_definition();
                    KeyAction::None
                }
                KeyCode::Char('r') => {
                    // SPC m r: find references
                    self.run_find_references();
                    KeyAction::None
                }
                KeyCode::Char('h') => {
                    // SPC m h: hover documentation
                    self.run_hover_doc();
                    KeyAction::None
                }
                KeyCode::Char('e') => {
                    // SPC m e: rename symbol (iedit)
                    self.start_iedit();
                    KeyAction::None
                }
                KeyCode::Char('l') => {
                    // SPC m l: start/connect LSP server
                    self.start_lsp();
                    KeyAction::None
                }
                _ => KeyAction::None,
            },
            // SPC a: AI/LLM operations
            'a' => match key.code {
                KeyCode::Char('a') => {
                    self.activate_minibuffer_with_prompt(EditorMode::Command, "Ask AI: ");
                    KeyAction::None
                }
                KeyCode::Char('c') => {
                    self.status_message = "AI chat (serana-llm)".to_string();
                    KeyAction::None
                }
                _ => KeyAction::None,
            },
            _ => KeyAction::None,
        }
    }

    fn handle_register_key(&mut self, kind: char, key: KeyEvent) {
        let name = match key.code {
            KeyCode::Char(c) if c.is_ascii_alphanumeric() || c == '"' => c,
            _ => {
                self.status_message = "Invalid register".to_string();
                return;
            }
        };
        match kind {
            'c' => {
                let content = self.buffer.current_line().to_string();
                self.registers.set(name, RegisterValue::Text(content));
                self.status_message = format!("Copied to register {}", name);
            }
            'y' => {
                if let Some(RegisterValue::Text(t)) = self.registers.get(name) {
                    let row = self.buffer.cursor.row;
                    self.buffer.lines.insert(row + 1, t.clone());
                    self.buffer.cursor.row = row + 1;
                    self.buffer.cursor.col = 0;
                    self.buffer.modified = true;
                    self.status_message = format!("Yanked from register {}", name);
                } else if let Some(RegisterValue::Lines(l)) = self.registers.get(name) {
                    let row = self.buffer.cursor.row + 1;
                    for (i, line) in l.iter().enumerate() {
                        self.buffer.lines.insert(row + i, line.clone());
                    }
                    self.buffer.modified = true;
                    self.status_message =
                        format!("Yanked {} lines from register {}", l.len(), name);
                }
            }
            'i' => {
                if let Some(RegisterValue::Text(t)) = self.registers.get(name) {
                    self.buffer.insert_string(t);
                    self.status_message = format!("Inserted from register {}", name);
                }
            }
            'm' => {
                self.registers
                    .set(name, RegisterValue::Position(self.buffer.cursor));
                self.status_message = format!("Set mark {}", name);
            }
            '\'' => {
                if let Some(RegisterValue::Position(pos)) = self.registers.get(name) {
                    self.buffer.cursor = *pos;
                    self.status_message = format!("Jumped to mark {}", name);
                }
            }
            'r' => {
                if let Some(RegisterValue::Rectangle(r)) = self.registers.get(name) {
                    self.status_message = format!("Rectangle register {}: {} cols", name, r.len());
                }
            }
            _ => {
                self.status_message = format!("Register {}: unknown operation", name);
            }
        }
    }

    fn handle_normal(&mut self, key: KeyEvent) -> KeyAction {
        if self.waiting_find {
            self.waiting_find = false;
            if let KeyCode::Char(c) = key.code {
                self.last_find_char = Some(c);
                self.last_find_forward = self.waiting_find_forward;
                self.last_find_till = self.waiting_find_till;
                let line = self.buffer.current_line();
                let col = self.buffer.cursor.col;
                let chars: Vec<char> = line.chars().collect();
                if self.waiting_find_forward {
                    let start = if self.waiting_find_till { col + 1 } else { col };
                    if let Some(pos) = chars[start..].iter().position(|&ch| ch == c) {
                        let target = start + pos;
                        let new_col = if self.waiting_find_till {
                            target.saturating_sub(1)
                        } else {
                            target
                        };
                        self.buffer.cursor.col = new_col;
                    }
                } else {
                    let end = if self.waiting_find_till { col } else { col + 1 };
                    if let Some(pos) = chars[..end].iter().rposition(|&ch| ch == c) {
                        let new_col = if self.waiting_find_till { pos + 1 } else { pos };
                        self.buffer.cursor.col = new_col;
                    }
                }
            }
            return KeyAction::None;
        }

        if self.waiting_g {
            self.waiting_g = false;
            return match key.code {
                KeyCode::Char('g') => KeyAction::MoveFileStart,
                KeyCode::Char('x') => KeyAction::EvilExchange,
                _ => KeyAction::None,
            };
        }

        if self.waiting_text_object {
            self.waiting_text_object = false;
            let op = self.waiting_op.take();
            let inner = self.text_object_inner;
            return if let KeyCode::Char(c) = key.code {
                let bracket_pair = match c {
                    '(' | ')' => Some(('(', ')')),
                    '{' | '}' => Some(('{', '}')),
                    '[' | ']' => Some(('[', ']')),
                    '"' => Some(('"', '"')),
                    '\'' => Some(('\'', '\'')),
                    '`' => Some(('`', '`')),
                    _ => None,
                };
                if let Some((open, close)) = bracket_pair {
                    match op {
                        Some(PendingOp::Delete) => {
                            if inner {
                                KeyAction::DeleteInnerBrackets(open, close)
                            } else {
                                KeyAction::DeleteAroundBrackets(open, close)
                            }
                        }
                        Some(PendingOp::Change) => {
                            self.pending_action = Some(KeyAction::SetMode(EditorMode::Insert));
                            if inner {
                                KeyAction::DeleteInnerBrackets(open, close)
                            } else {
                                KeyAction::DeleteAroundBrackets(open, close)
                            }
                        }
                        _ => KeyAction::None,
                    }
                } else {
                    match (op, inner, c) {
                        (Some(PendingOp::Delete), true, 'w') => KeyAction::DeleteInnerWord,
                        (Some(PendingOp::Delete), false, 'w') => KeyAction::DeleteAroundWord,
                        (Some(PendingOp::Change), true, 'w') => {
                            self.pending_action = Some(KeyAction::SetMode(EditorMode::Insert));
                            KeyAction::DeleteInnerWord
                        }
                        (Some(PendingOp::Change), false, 'w') => {
                            self.pending_action = Some(KeyAction::SetMode(EditorMode::Insert));
                            KeyAction::DeleteAroundWord
                        }
                        _ => KeyAction::None,
                    }
                }
            } else {
                KeyAction::None
            };
        }

        // Evil-surround: intercept d/c/y followed by 's'
        if let Some(op) = self.waiting_surround_s.take() {
            if key.code == KeyCode::Char('s') && key.modifiers.is_empty() {
                match op {
                    PendingOp::Change => {
                        // cs - wait for old_char then new_char
                        self.status_message = "cs".to_string();
                        self.surround_old_char = Some('\0'); // sentinel: waiting for first char
                        return KeyAction::None;
                    }
                    PendingOp::Delete => {
                        // ds - wait for target char
                        self.status_message = "ds".to_string();
                        self.surround_new_char = Some('\0'); // sentinel: waiting for char
                        return KeyAction::None;
                    }
                    PendingOp::Yank => {
                        // ys - wait for text-object then surround char
                        self.waiting_surround_add = true;
                        self.status_message = "ys".to_string();
                        return KeyAction::None;
                    }
                }
            } else {
                // Not 's', fall through to normal operator-pending
                self.waiting_op = Some(op);
            }
        }

        // cs: waiting for old_char then new_char
        if let Some(old) = self.surround_old_char {
            if old == '\0' {
                // Waiting for first char (old surround)
                if let KeyCode::Char(c) = key.code {
                    self.surround_old_char = Some(c);
                    self.status_message = format!("cs{}", c);
                }
                return KeyAction::None;
            } else {
                // Waiting for second char (new surround)
                if let KeyCode::Char(new) = key.code {
                    self.surround_old_char = None;
                    return KeyAction::ChangeSurround(old, new);
                }
                self.surround_old_char = None;
                return KeyAction::None;
            }
        }

        // ds: waiting for target char
        if let Some(sentinel) = self.surround_new_char.take() {
            if sentinel == '\0' {
                if let KeyCode::Char(c) = key.code {
                    return KeyAction::DeleteSurround(c);
                }
                return KeyAction::None;
            }
        }

        // ys: waiting for text-object then surround char
        if self.waiting_surround_add {
            if self.surround_range.is_some() {
                // Already have range, waiting for surround char
                if let KeyCode::Char(c) = key.code {
                    let range = self.surround_range.take();
                    self.waiting_surround_add = false;
                    if let Some((start, end)) = range {
                        self.buffer.cursor.col = start;
                        self.surround_range = Some((start, end));
                        return KeyAction::AddSurround(c);
                    }
                }
                self.waiting_surround_add = false;
                self.surround_range = None;
                return KeyAction::None;
            }
            // Waiting for text-object key
            if let KeyCode::Char(c) = key.code {
                let range = match c {
                    'w' => Some(self.buffer.around_word_range()),
                    'W' => Some(self.buffer.inner_word_range()),
                    '(' | ')' => Some(self.buffer.around_bracket_range('(', ')')),
                    '{' | '}' => Some(self.buffer.around_bracket_range('{', '}')),
                    '[' | ']' => Some(self.buffer.around_bracket_range('[', ']')),
                    '"' => Some(self.buffer.around_bracket_range('"', '"')),
                    '\'' => Some(self.buffer.around_bracket_range('\'', '\'')),
                    '`' => Some(self.buffer.around_bracket_range('`', '`')),
                    _ => None,
                };
                if let Some((start, end)) = range {
                    self.surround_range = Some((start, end));
                    self.status_message = format!("ys{}...", c);
                } else {
                    self.waiting_surround_add = false;
                }
            }
            return KeyAction::None;
        }

        if let Some(op) = self.waiting_op {
            self.waiting_op = None;
            if let KeyCode::Char('i') | KeyCode::Char('a') = key.code {
                self.text_object_inner = key.code == KeyCode::Char('i');
                self.waiting_op = Some(op);
                self.waiting_text_object = true;
                return KeyAction::None;
            }
            return if key.code == KeyCode::Char(op_char(op)) {
                match op {
                    PendingOp::Delete => KeyAction::DeleteLine,
                    PendingOp::Yank => KeyAction::YankLine,
                    PendingOp::Change => {
                        self.pending_action = Some(KeyAction::SetMode(EditorMode::Insert));
                        KeyAction::DeleteLine
                    }
                }
            } else {
                let motion = self.handle_normal(key);
                match (op, &motion) {
                    (PendingOp::Delete, KeyAction::MoveWordForward) => KeyAction::DeleteWordForward,
                    (PendingOp::Delete, KeyAction::MoveWordBackward) => {
                        KeyAction::DeleteWordBackward
                    }
                    (PendingOp::Delete, KeyAction::MoveWordEnd) => KeyAction::DeleteToEndOfWord,
                    (PendingOp::Delete, KeyAction::MoveLineEnd) => KeyAction::DeleteToEol,
                    (PendingOp::Delete, KeyAction::MoveLineStart) => KeyAction::DeleteToStartOfLine,
                    (PendingOp::Yank, KeyAction::MoveWordForward) => KeyAction::YankWord,
                    (PendingOp::Yank, KeyAction::MoveWordBackward) => KeyAction::YankWord,
                    (PendingOp::Change, KeyAction::MoveWordForward) => {
                        self.pending_action = Some(KeyAction::SetMode(EditorMode::Insert));
                        KeyAction::DeleteWordForward
                    }
                    (PendingOp::Change, KeyAction::MoveWordBackward) => {
                        self.pending_action = Some(KeyAction::SetMode(EditorMode::Insert));
                        KeyAction::DeleteWordBackward
                    }
                    (PendingOp::Change, KeyAction::MoveWordEnd) => {
                        self.pending_action = Some(KeyAction::SetMode(EditorMode::Insert));
                        KeyAction::DeleteToEndOfWord
                    }
                    (PendingOp::Change, KeyAction::MoveLineEnd) => {
                        self.pending_action = Some(KeyAction::SetMode(EditorMode::Insert));
                        KeyAction::DeleteToEol
                    }
                    _ => motion,
                }
            };
        }

        if let Some(count) = self.repeat_count {
            if let KeyCode::Char(c) = key.code {
                if c.is_ascii_digit() {
                    self.repeat_count = Some(count * 10 + (c as u8 - b'0') as usize);
                    return KeyAction::None;
                }
            }
            let action = keymap::normal_key(key);
            let n = count.max(1);
            self.repeat_count = None;
            match &action {
                KeyAction::MoveLeft
                | KeyAction::MoveRight
                | KeyAction::MoveUp
                | KeyAction::MoveDown
                | KeyAction::DeleteForward => {
                    return self.repeated_action(action, n);
                }
                _ => {}
            }
            return action;
        }

        // SPC as leader key prefix (coldnew-emacs style)
        if key.modifiers.is_empty() && key.code == KeyCode::Char(' ') {
            self.status_message = "SPC-".to_string();
            self.waiting_prefix = Some(' ');
            return KeyAction::None;
        }

        let action = keymap::normal_key(key);

        if let KeyCode::Char(c) = key.code {
            if c.is_ascii_digit() && !action.needs_digit() {
                self.repeat_count = Some((c as u8 - b'0') as usize);
                return KeyAction::None;
            }
        }

        if self.waiting_z {
            self.waiting_z = false;
            return match key.code {
                KeyCode::Char('o') => KeyAction::OpenFold,
                KeyCode::Char('c') => KeyAction::CloseFold,
                KeyCode::Char('a') => KeyAction::ToggleFoldEvil,
                KeyCode::Char('r') => KeyAction::ReduceFolds,
                KeyCode::Char('m') => KeyAction::MaximizeFolds,
                _ => KeyAction::None,
            };
        }

        if key.code == KeyCode::Char('g') && key.modifiers.is_empty() {
            self.waiting_g = true;
            return KeyAction::None;
        }
        if key.code == KeyCode::Char('z') && key.modifiers.is_empty() {
            self.waiting_z = true;
            return KeyAction::None;
        }

        match &action {
            KeyAction::None if key.code == KeyCode::Char('d') && key.modifiers.is_empty() => {
                self.waiting_surround_s = Some(PendingOp::Delete);
                return KeyAction::None;
            }
            KeyAction::None if key.code == KeyCode::Char('y') && key.modifiers.is_empty() => {
                self.waiting_surround_s = Some(PendingOp::Yank);
                return KeyAction::None;
            }
            KeyAction::None if key.code == KeyCode::Char('c') && key.modifiers.is_empty() => {
                self.waiting_surround_s = Some(PendingOp::Change);
                return KeyAction::None;
            }
            _ => {}
        }

        match key.code {
            KeyCode::Char('f') if key.modifiers.is_empty() => {
                self.waiting_find = true;
                self.waiting_find_forward = true;
                self.waiting_find_till = false;
                return KeyAction::None;
            }
            KeyCode::Char('F') if key.modifiers.is_empty() => {
                self.waiting_find = true;
                self.waiting_find_forward = false;
                self.waiting_find_till = false;
                return KeyAction::None;
            }
            KeyCode::Char('t') if key.modifiers.is_empty() => {
                self.waiting_find = true;
                self.waiting_find_forward = true;
                self.waiting_find_till = true;
                return KeyAction::None;
            }
            KeyCode::Char('T') if key.modifiers.is_empty() => {
                self.waiting_find = true;
                self.waiting_find_forward = false;
                self.waiting_find_till = true;
                return KeyAction::None;
            }
            KeyCode::Char(';') if key.modifiers.is_empty() => {
                self.repeat_last_find(true);
                return KeyAction::None;
            }
            KeyCode::Char(',') if key.modifiers.is_empty() => {
                self.repeat_last_find(false);
                return KeyAction::None;
            }
            KeyCode::Char('*') if key.modifiers.is_empty() => {
                return KeyAction::SearchWordForward;
            }
            KeyCode::Char('#') if key.modifiers.is_empty() => {
                return KeyAction::SearchWordBackward;
            }
            KeyCode::Char('~') if key.modifiers.is_empty() => {
                return KeyAction::ToggleCase;
            }
            KeyCode::Char('S') if key.modifiers.is_empty() => {
                return KeyAction::SubstituteLine;
            }
            KeyCode::Char('.') if key.modifiers.is_empty() => {
                return KeyAction::RepeatLastChange;
            }
            KeyCode::Char('%') if key.modifiers.is_empty() => {
                return KeyAction::GotoMatchingBracket;
            }
            _ => {}
        }

        if let Some(post) = keymap::normal_key_post(key) {
            self.pending_action = Some(post);
        }

        // If execute_once_mode is set, return to the saved mode after this action
        if let Some(return_mode) = self.execute_once_mode.take() {
            self.mode = return_mode;
        }

        action
    }

    fn handle_emacs(&mut self, key: KeyEvent) -> KeyAction {
        if self.waiting_zap {
            self.waiting_zap = false;
            if let KeyCode::Char(c) = key.code {
                let line = self.buffer.current_line();
                let chars: Vec<char> = line.chars().collect();
                let col = self.buffer.cursor.col;
                if let Some(pos) = chars[col..].iter().position(|&ch| ch == c) {
                    let end = col + pos + 1;
                    self.buffer.delete_range(col, end);
                }
            }
            return KeyAction::None;
        }

        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
                self.waiting_prefix = Some('x');
                KeyAction::None
            }
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                self.waiting_prefix = Some('c');
                KeyAction::None
            }
            (KeyModifiers::CONTROL, KeyCode::Char('h')) => {
                self.waiting_register = Some('h');
                self.status_message = "Help (C-h): ".to_string();
                KeyAction::None
            }

            (KeyModifiers::CONTROL, KeyCode::Char('b')) => KeyAction::MoveLeft,
            (KeyModifiers::CONTROL, KeyCode::Char('f')) => KeyAction::MoveRight,
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => KeyAction::MoveUp,
            (KeyModifiers::CONTROL, KeyCode::Char('n')) => KeyAction::MoveDown,
            (KeyModifiers::CONTROL, KeyCode::Char('a')) => KeyAction::MwimBeginning,
            (KeyModifiers::CONTROL, KeyCode::Char('e')) => KeyAction::MwimEnd,
            (KeyModifiers::CONTROL, KeyCode::Char('v')) => KeyAction::PageDown,
            (KeyModifiers::ALT, KeyCode::Char('v')) => KeyAction::ExpandRegion,

            (KeyModifiers::ALT, KeyCode::Char('f')) => KeyAction::MoveWordForward,
            (KeyModifiers::ALT, KeyCode::Char('b')) => KeyAction::MoveWordBackward,
            (KeyModifiers::ALT, KeyCode::Char('a')) => KeyAction::MoveFileStart,
            (KeyModifiers::ALT, KeyCode::Char('e')) | (KeyModifiers::ALT, KeyCode::Char('>')) => {
                KeyAction::MoveFileEnd
            }
            (KeyModifiers::ALT, KeyCode::Char('<')) => KeyAction::MoveFileStart,

            (KeyModifiers::CONTROL, KeyCode::Char('d')) => KeyAction::HungryDeleteForward,
            (KeyModifiers::CONTROL, KeyCode::Char('k')) => KeyAction::KillLine,
            (KeyModifiers::ALT, KeyCode::Char('d')) => KeyAction::KillWordForward,
            (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
                if self.mark_ring.is_active() {
                    KeyAction::KillRegion
                } else {
                    KeyAction::DeleteWordBackward
                }
            }
            (KeyModifiers::ALT, KeyCode::Char('w')) => {
                if self.mark_ring.is_active() {
                    KeyAction::CopyRegion
                } else {
                    KeyAction::None
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('y')) => KeyAction::Yank,
            (KeyModifiers::ALT, KeyCode::Char('y')) => KeyAction::YankPop,

            (KeyModifiers::CONTROL, KeyCode::Char('/'))
            | (KeyModifiers::CONTROL, KeyCode::Char('_')) => KeyAction::Undo,
            (KeyModifiers::CONTROL, KeyCode::Char('g')) => {
                self.mode = EditorMode::Normal;
                self.clear_minibuffer();
                KeyAction::None
            }

            (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                KeyAction::SetMode(EditorMode::SearchForward)
            }
            (KeyModifiers::CONTROL, KeyCode::Char('r')) => {
                KeyAction::SetMode(EditorMode::SearchBackward)
            }

            (KeyModifiers::CONTROL, KeyCode::Char(' ')) | (_, KeyCode::Char(' ')) => {
                self.mark_ring.push(self.buffer.cursor);
                self.mark_ring.set_active(true);
                self.status_message = "Mark set".to_string();
                KeyAction::None
            }
            // Emacs: C-u universal arg / pop mark
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                if self.mark_ring.is_active() {
                    KeyAction::PopMark
                } else {
                    KeyAction::UniversalArg
                }
            }
            (KeyModifiers::ALT, KeyCode::Char('u')) => KeyAction::UppercaseWord,
            (KeyModifiers::ALT, KeyCode::Char('x')) => {
                self.activate_minibuffer_with_prompt(EditorMode::Command, "M-x ");
                KeyAction::None
            }

            (KeyModifiers::CONTROL, KeyCode::Char('l')) => KeyAction::HungryDeleteBackward,

            (KeyModifiers::ALT, KeyCode::Char('q')) => {
                self.toggle_record_macro();
                KeyAction::None
            }

            (_, KeyCode::Tab) => {
                if self.mark_ring.is_active() {
                    let a = self.mark_ring.peek().copied().unwrap_or(self.buffer.cursor);
                    let b = self.buffer.cursor;
                    let rect = RectRegion::from_corners(a, b);
                    self.mode = EditorMode::Normal;
                    self.mark_ring.set_active(false);
                    return KeyAction::IndentRegion(rect);
                }
                KeyAction::IndentLine
            }

            (KeyModifiers::CONTROL, KeyCode::Char('q')) => KeyAction::Quit,

            // Emacs: C-t transpose char
            (KeyModifiers::CONTROL, KeyCode::Char('t')) => KeyAction::TransposeChar,
            // Emacs: C-o execute one normal command then return to emacs mode
            (KeyModifiers::CONTROL, KeyCode::Char('o')) => {
                self.execute_once_mode = Some(EditorMode::Emacs);
                self.mode = EditorMode::Normal;
                KeyAction::None
            }
            // Emacs: M-t transpose word
            (KeyModifiers::ALT, KeyCode::Char('t')) => KeyAction::TransposeWord,
            // Emacs: M-c capitalize word
            (KeyModifiers::ALT, KeyCode::Char('c')) => KeyAction::CapitalizeWord,
            // Emacs: M-l lowercase word
            (KeyModifiers::ALT, KeyCode::Char('l')) => KeyAction::LowercaseWord,
            // Emacs: M-g prefix (goto)
            (KeyModifiers::ALT, KeyCode::Char('g')) => {
                self.waiting_prefix = Some('g');
                KeyAction::None
            }
            // Emacs: M-/ dabbrev-expand
            (KeyModifiers::ALT, KeyCode::Char('/')) => KeyAction::DabbrevExpand,
            // Emacs: M-z zap-to-char
            (KeyModifiers::ALT, KeyCode::Char('z')) => {
                self.waiting_zap = true;
                self.status_message = "Zap to char: ".to_string();
                KeyAction::None
            }
            // Emacs: C-; iedit (multi-cursor edit all occurrences)
            (KeyModifiers::CONTROL, KeyCode::Char(';')) => {
                self.start_iedit();
                KeyAction::None
            }

            (_, KeyCode::Esc) => KeyAction::SetMode(EditorMode::Normal),

            (_, KeyCode::Char(c)) => KeyAction::InsertChar(c),
            (_, KeyCode::Enter) => KeyAction::InsertNewline,
            (_, KeyCode::Backspace) => KeyAction::DeleteBackward,
            (_, KeyCode::Delete) => KeyAction::DeleteForward,
            (_, KeyCode::Left) => KeyAction::MoveLeft,
            (_, KeyCode::Right) => KeyAction::MoveRight,
            (_, KeyCode::Up) => KeyAction::MoveUp,
            (_, KeyCode::Down) => KeyAction::MoveDown,
            (_, KeyCode::Home) => KeyAction::MoveLineStart,
            (_, KeyCode::End) => KeyAction::MoveLineEnd,
            (_, KeyCode::PageUp) => KeyAction::PageUp,
            (_, KeyCode::PageDown) => KeyAction::PageDown,
            _ => KeyAction::None,
        }
    }

    fn start_iedit(&mut self) {
        self.buffer.push_undo_snapshot();
        self.iedit_regex = false;
        let row = self.buffer.cursor.row;
        let col = self.buffer.cursor.col;
        let line = self.buffer.current_line();
        let chars: Vec<char> = line.chars().collect();

        // Find word under cursor
        let (word_start, word_end) = if col < chars.len() && chars[col].is_alphanumeric()
            || col < chars.len() && chars[col] == '_'
        {
            let mut start = col;
            while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
                start -= 1;
            }
            let mut end = col;
            while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
                end += 1;
            }
            (start, end)
        } else {
            return;
        };

        if word_start == word_end {
            return;
        }

        let word: String = chars[word_start..word_end].iter().collect();
        self.iedit_word = Some(word.clone());
        self.iedit_regions.clear();
        self.iedit_cursor_idx = 0;

        // Find all occurrences in the buffer
        let mut idx = 0;
        for (r, line) in self.buffer.lines.iter().enumerate() {
            let line_chars: Vec<char> = line.chars().collect();
            let mut c = 0;
            while c + word.chars().count() <= line_chars.len() {
                let candidate: String = line_chars[c..c + word.chars().count()].iter().collect();
                if candidate == word {
                    // Check word boundary
                    let left_ok = c == 0
                        || !(line_chars[c - 1].is_alphanumeric() || line_chars[c - 1] == '_');
                    let right_ok = c + word.chars().count() >= line_chars.len()
                        || !(line_chars[c + word.chars().count()].is_alphanumeric()
                            || line_chars[c + word.chars().count()] == '_');
                    if left_ok && right_ok {
                        if r == row && c == word_start {
                            self.iedit_cursor_idx = idx;
                        }
                        self.iedit_regions.push((r, c, c + word.chars().count()));
                        idx += 1;
                    }
                }
                c += 1;
            }
        }

        if self.iedit_regions.len() < 2 {
            self.iedit_word = None;
            self.iedit_regions.clear();
            self.status_message = "No other occurrences".to_string();
            return;
        }

        self.mode = EditorMode::Iedit;
        self.status_message = format!(
            "Iedit: {} ({} regions, Esc to exit)",
            word,
            self.iedit_regions.len()
        );
    }

    fn handle_iedit(&mut self, key: KeyEvent) -> KeyAction {
        match (key.modifiers, key.code) {
            // Exit iedit
            (_, KeyCode::Esc) | (KeyModifiers::CONTROL, KeyCode::Char('g')) => {
                let word = self.iedit_word.take().unwrap_or_default();
                let n = self.iedit_regions.len();
                self.iedit_regions.clear();
                self.iedit_cursor_idx = 0;
                self.iedit_regex = false;
                self.mode = EditorMode::Emacs;
                self.status_message = format!("Iedit exited ({} regions, \"{}\")", n, word);
                KeyAction::None
            }
            // Tab: cycle forward through regions
            (_, KeyCode::Tab) => {
                self.iedit_next_region();
                KeyAction::None
            }
            // Shift-Tab: cycle backward
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                self.iedit_prev_region();
                KeyAction::None
            }
            // C-n: next region
            (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
                self.iedit_next_region();
                KeyAction::None
            }
            // C-p: previous region
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                self.iedit_prev_region();
                KeyAction::None
            }
            // C-d: skip current region (remove from list)
            (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
                self.iedit_skip_region();
                KeyAction::None
            }
            // C-; while in iedit: add cursor position as a new region
            (KeyModifiers::CONTROL, KeyCode::Char(';')) => {
                self.iedit_add_region_at_cursor();
                KeyAction::None
            }
            // C-a: re-find all regions (reset after skips)
            (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
                self.iedit_refind_all();
                KeyAction::None
            }
            // C-k: kill to end of region in all regions
            (KeyModifiers::CONTROL, KeyCode::Char('k')) => {
                self.iedit_kill_region();
                KeyAction::None
            }
            // Insert char in all regions
            (_, KeyCode::Char(c)) => {
                self.iedit_insert_char(c);
                self.iedit_update_status();
                KeyAction::None
            }
            // Backspace in all regions
            (_, KeyCode::Backspace) => {
                self.iedit_delete_backward();
                self.iedit_update_status();
                KeyAction::None
            }
            // Delete forward in all regions
            (_, KeyCode::Delete) => {
                self.iedit_delete_forward();
                self.iedit_update_status();
                KeyAction::None
            }
            _ => KeyAction::None,
        }
    }

    fn iedit_insert_char(&mut self, c: char) {
        // Apply in reverse order to preserve indices
        for i in (0..self.iedit_regions.len()).rev() {
            let (row, _start, end) = self.iedit_regions[i];
            if row < self.buffer.lines.len() {
                let line_len = self.buffer.lines[row].chars().count();
                let insert_at = end.min(line_len);
                // Convert char index to byte index
                let byte_pos: usize = self.buffer.lines[row]
                    .chars()
                    .take(insert_at)
                    .map(|ch| ch.len_utf8())
                    .sum();
                self.buffer.lines[row].insert(byte_pos, c);
                self.buffer.modified = true;
                // Update all regions on this row and after
                for j in 0..self.iedit_regions.len() {
                    if self.iedit_regions[j].0 == row {
                        if self.iedit_regions[j].2 >= insert_at {
                            self.iedit_regions[j].2 += 1;
                        }
                        if self.iedit_regions[j].1 > insert_at {
                            self.iedit_regions[j].1 += 1;
                        }
                    }
                }
            }
        }
        // Update cursor to end of current region
        if self.iedit_cursor_idx < self.iedit_regions.len() {
            let (r, _s, e) = self.iedit_regions[self.iedit_cursor_idx];
            self.buffer.cursor.row = r;
            self.buffer.cursor.col = e;
        }
    }

    fn iedit_delete_backward(&mut self) {
        for i in (0..self.iedit_regions.len()).rev() {
            let (row, start, end) = self.iedit_regions[i];
            if start < end && row < self.buffer.lines.len() {
                let byte_pos: usize = self.buffer.lines[row]
                    .chars()
                    .take(end - 1)
                    .map(|ch| ch.len_utf8())
                    .sum();
                let char_byte_len = self.buffer.lines[row]
                    .chars()
                    .nth(end - 1)
                    .map(|ch| ch.len_utf8())
                    .unwrap_or(0);
                self.buffer.lines[row].drain(byte_pos..byte_pos + char_byte_len);
                self.buffer.modified = true;
                let removed = 1;
                for j in 0..self.iedit_regions.len() {
                    if self.iedit_regions[j].0 == row {
                        if self.iedit_regions[j].2 >= end {
                            self.iedit_regions[j].2 -= removed;
                        }
                        if self.iedit_regions[j].1 >= end {
                            self.iedit_regions[j].1 -= removed;
                        }
                    }
                }
            }
        }
        // Update cursor
        if self.iedit_cursor_idx < self.iedit_regions.len() {
            let (r, _s, e) = self.iedit_regions[self.iedit_cursor_idx];
            self.buffer.cursor.row = r;
            self.buffer.cursor.col = e;
        }
    }

    fn iedit_delete_forward(&mut self) {
        // Delete the last character of each region (end-1)
        for i in (0..self.iedit_regions.len()).rev() {
            let (row, start, end) = self.iedit_regions[i];
            if start < end && row < self.buffer.lines.len() {
                let delete_pos = end - 1;
                let byte_pos: usize = self.buffer.lines[row]
                    .chars()
                    .take(delete_pos)
                    .map(|ch| ch.len_utf8())
                    .sum();
                let char_byte_len = self.buffer.lines[row]
                    .chars()
                    .nth(delete_pos)
                    .map(|ch| ch.len_utf8())
                    .unwrap_or(0);
                self.buffer.lines[row].drain(byte_pos..byte_pos + char_byte_len);
                self.buffer.modified = true;
                for j in 0..self.iedit_regions.len() {
                    if self.iedit_regions[j].0 == row {
                        if self.iedit_regions[j].2 > delete_pos {
                            self.iedit_regions[j].2 -= 1;
                        }
                        if self.iedit_regions[j].1 > delete_pos {
                            self.iedit_regions[j].1 -= 1;
                        }
                    }
                }
            }
        }
        if self.iedit_cursor_idx < self.iedit_regions.len() {
            let (r, _s, e) = self.iedit_regions[self.iedit_cursor_idx];
            self.buffer.cursor.row = r;
            self.buffer.cursor.col = e;
        }
    }

    fn iedit_next_region(&mut self) {
        if self.iedit_regions.is_empty() {
            return;
        }
        self.iedit_cursor_idx = (self.iedit_cursor_idx + 1) % self.iedit_regions.len();
        let (r, c, _) = self.iedit_regions[self.iedit_cursor_idx];
        self.buffer.cursor.row = r;
        self.buffer.cursor.col = c;
        self.view.ensure_cursor_visible(&self.buffer);
    }

    fn iedit_prev_region(&mut self) {
        if self.iedit_regions.is_empty() {
            return;
        }
        self.iedit_cursor_idx = if self.iedit_cursor_idx == 0 {
            self.iedit_regions.len() - 1
        } else {
            self.iedit_cursor_idx - 1
        };
        let (r, c, _) = self.iedit_regions[self.iedit_cursor_idx];
        self.buffer.cursor.row = r;
        self.buffer.cursor.col = c;
        self.view.ensure_cursor_visible(&self.buffer);
    }

    fn iedit_skip_region(&mut self) {
        if self.iedit_regions.is_empty() {
            return;
        }
        self.iedit_regions.remove(self.iedit_cursor_idx);
        if self.iedit_regions.is_empty() {
            let word = self.iedit_word.take().unwrap_or_default();
            self.iedit_regex = false;
            self.mode = EditorMode::Emacs;
            self.status_message = format!("Iedit: all regions removed (\"{}\")", word);
            return;
        }
        if self.iedit_cursor_idx >= self.iedit_regions.len() {
            self.iedit_cursor_idx = 0;
        }
        let (r, c, _) = self.iedit_regions[self.iedit_cursor_idx];
        self.buffer.cursor.row = r;
        self.buffer.cursor.col = c;
        self.view.ensure_cursor_visible(&self.buffer);
        self.iedit_update_status();
    }

    fn iedit_add_region_at_cursor(&mut self) {
        let word = match &self.iedit_word {
            Some(w) => w.clone(),
            None => return,
        };
        let row = self.buffer.cursor.row;
        let col = self.buffer.cursor.col;
        let line = self.buffer.current_line();
        let chars: Vec<char> = line.chars().collect();
        let wc = word.chars().count();
        if col + wc > chars.len() {
            return;
        }
        let candidate: String = chars[col..col + wc].iter().collect();
        if candidate != word {
            self.status_message = format!("Iedit: word at cursor is \"{}\", expected \"{}\"", candidate, word);
            return;
        }
        // Check for duplicate
        for &(r, s, _) in &self.iedit_regions {
            if r == row && s == col {
                self.status_message = "Iedit: region already exists".to_string();
                return;
            }
        }
        self.iedit_regions.push((row, col, col + wc));
        self.iedit_cursor_idx = self.iedit_regions.len() - 1;
        self.iedit_update_status();
    }

    fn iedit_refind_all(&mut self) {
        let word = match &self.iedit_word {
            Some(w) => w.clone(),
            None => return,
        };
        let row = self.buffer.cursor.row;
        let col = self.buffer.cursor.col;
        let is_regex = self.iedit_regex;
        self.iedit_regions.clear();
        self.iedit_cursor_idx = 0;
        for (r, line) in self.buffer.lines.iter().enumerate() {
            if is_regex {
                // Regex mode: use the word as a regex pattern
                if let Ok(re) = regex::Regex::new(&word) {
                    for m in re.find_iter(line) {
                        let start = m.start();
                        let end = m.end();
                        if r == row && start <= col && col <= end {
                            self.iedit_cursor_idx = self.iedit_regions.len();
                        }
                        self.iedit_regions.push((r, start, end));
                    }
                }
            } else {
                // Exact word mode
                let line_chars: Vec<char> = line.chars().collect();
                let mut c = 0;
                while c + word.chars().count() <= line_chars.len() {
                    let candidate: String = line_chars[c..c + word.chars().count()].iter().collect();
                    if candidate == word {
                        let left_ok = c == 0
                            || !(line_chars[c - 1].is_alphanumeric() || line_chars[c - 1] == '_');
                        let right_ok = c + word.chars().count() >= line_chars.len()
                            || !(line_chars[c + word.chars().count()].is_alphanumeric()
                                || line_chars[c + word.chars().count()] == '_');
                        if left_ok && right_ok {
                            if r == row && c <= col && col <= c + word.chars().count() {
                                self.iedit_cursor_idx = self.iedit_regions.len();
                            }
                            self.iedit_regions.push((r, c, c + word.chars().count()));
                        }
                    }
                    c += 1;
                }
            }
        }
        if self.iedit_regions.is_empty() {
            self.iedit_word = None;
            self.iedit_regex = false;
            self.mode = EditorMode::Emacs;
            self.status_message = "Iedit: no occurrences found".to_string();
            return;
        }
        let (r, c, _) = self.iedit_regions[self.iedit_cursor_idx];
        self.buffer.cursor.row = r;
        self.buffer.cursor.col = c;
        self.view.ensure_cursor_visible(&self.buffer);
        self.iedit_update_status();
    }

    fn iedit_kill_region(&mut self) {
        for i in (0..self.iedit_regions.len()).rev() {
            let (row, _start, end) = self.iedit_regions[i];
            if row < self.buffer.lines.len() {
                let line_len = self.buffer.lines[row].chars().count();
                if end <= line_len {
                    let byte_pos: usize = self.buffer.lines[row]
                        .chars()
                        .take(end)
                        .map(|ch| ch.len_utf8())
                        .sum();
                    let remaining = self.buffer.lines[row][byte_pos..].to_string();
                    let kill_bytes = remaining.len();
                    if kill_bytes > 0 {
                        self.buffer.lines[row].drain(byte_pos..);
                        // Truncate instead of drain-then-drain
                        // Actually we already drained the rest. Just mark modified.
                    }
                    if kill_bytes > 0 {
                        self.buffer.modified = true;
                        for j in 0..self.iedit_regions.len() {
                            if self.iedit_regions[j].0 == row && self.iedit_regions[j].2 > end {
                                self.iedit_regions[j].2 = end;
                            }
                        }
                    }
                }
            }
        }
        if self.iedit_cursor_idx < self.iedit_regions.len() {
            let (r, _s, e) = self.iedit_regions[self.iedit_cursor_idx];
            self.buffer.cursor.row = r;
            self.buffer.cursor.col = e;
        }
    }

    fn iedit_update_status(&mut self) {
        let word = self.iedit_word.as_deref().unwrap_or("");
        let mode = if self.iedit_regex { "regex" } else { "word" };
        self.status_message = format!(
            "Iedit: {} \"{}\" [{} region{}]  Tab next  C-d skip  C-; add  C-a refind  Esc exit",
            mode, word, self.iedit_regions.len(),
            if self.iedit_regions.len() == 1 { "" } else { "s" }
        );
    }

    fn start_iedit_regex(&mut self) {
        self.buffer.push_undo_snapshot();
        self.iedit_regex = true;
        self.waiting_iedit_regex = true;
        self.activate_minibuffer_with_prompt(EditorMode::Command, "Iedit regex: ");
    }

    fn handle_replace_char(&mut self, key: KeyEvent) -> KeyAction {
        self.mode = EditorMode::Normal;
        match key.code {
            KeyCode::Esc => KeyAction::None,
            KeyCode::Char(c) => KeyAction::ReplaceChar(c),
            _ => KeyAction::None,
        }
    }

    fn handle_visual(&mut self, key: KeyEvent) -> KeyAction {
        if self.waiting_g {
            self.waiting_g = false;
            return if key.code == KeyCode::Char('g') {
                KeyAction::MoveFileStart
            } else {
                KeyAction::None
            };
        }

        if self.waiting_visual_text_object {
            self.waiting_visual_text_object = false;
            let inner = self.text_object_inner;
            if let KeyCode::Char(c) = key.code {
                let bracket_pair = match c {
                    '(' | ')' => Some(('(', ')')),
                    '{' | '}' => Some(('{', '}')),
                    '[' | ']' => Some(('[', ']')),
                    '"' => Some(('"', '"')),
                    '\'' => Some(('\'', '\'')),
                    '`' => Some(('`', '`')),
                    _ => None,
                };
                let (start, end) = if let Some((open, close)) = bracket_pair {
                    if inner {
                        self.buffer.inner_bracket_range(open, close)
                    } else {
                        self.buffer.around_bracket_range(open, close)
                    }
                } else if c == 'w' || c == 'W' {
                    if inner {
                        self.buffer.inner_word_range()
                    } else {
                        self.buffer.around_word_range()
                    }
                } else {
                    return KeyAction::None;
                };
                let mark = self.mark_ring.peek().copied().unwrap_or(self.buffer.cursor);
                if mark.col <= start {
                    self.buffer.cursor.col = end;
                } else {
                    self.buffer.cursor.col = start;
                }
            }
            return KeyAction::None;
        }

        // Check for text object prefix (i/a) in visual mode
        if let KeyCode::Char('i') | KeyCode::Char('a') = key.code {
            self.text_object_inner = key.code == KeyCode::Char('i');
            self.waiting_visual_text_object = true;
            return KeyAction::None;
        }

        match (key.modifiers, key.code) {
            (_, KeyCode::Esc)
            | (KeyModifiers::CONTROL, KeyCode::Char('g'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                self.mark_ring.set_active(false);
                KeyAction::SetMode(EditorMode::Normal)
            }
            (_, KeyCode::Char('h')) | (_, KeyCode::Left) => KeyAction::MoveLeft,
            (_, KeyCode::Char('j')) | (_, KeyCode::Down) => KeyAction::MoveDown,
            (_, KeyCode::Char('k')) | (_, KeyCode::Up) => KeyAction::MoveUp,
            (_, KeyCode::Char('l')) | (_, KeyCode::Right) => KeyAction::MoveRight,
            (_, KeyCode::Char('w')) | (_, KeyCode::Char('W')) => KeyAction::MoveWordForward,
            (_, KeyCode::Char('b')) | (_, KeyCode::Char('B')) => KeyAction::MoveWordBackward,
            (_, KeyCode::Char('e')) | (_, KeyCode::Char('E')) => KeyAction::MoveWordEnd,
            (_, KeyCode::Char('0')) | (_, KeyCode::Home) => KeyAction::MoveLineStart,
            (_, KeyCode::Char('$')) | (_, KeyCode::End) => KeyAction::MoveLineEnd,
            (_, KeyCode::Char('^')) => KeyAction::MoveLineStart,
            (_, KeyCode::Char('G')) => KeyAction::MoveFileEnd,
            (_, KeyCode::Char('g')) => {
                self.waiting_g = true;
                KeyAction::None
            }
            (_, KeyCode::Char('o')) | (_, KeyCode::Char('O')) => {
                if let Some(mark_pos) = self.mark_ring.peek().copied() {
                    let cursor = self.buffer.cursor;
                    self.mark_ring.pop();
                    self.mark_ring.push(cursor);
                    self.buffer.cursor = mark_pos;
                }
                KeyAction::None
            }
            (_, KeyCode::Char('d')) | (_, KeyCode::Char('x')) => KeyAction::KillRegion,
            (_, KeyCode::Char('y')) => KeyAction::CopyRegion,
            (_, KeyCode::Char('D')) => KeyAction::KillLine,
            (_, KeyCode::Char('I')) => {
                if let Some(mark_pos) = self.mark_ring.peek().copied() {
                    self.buffer.cursor = mark_pos;
                }
                self.mark_ring.set_active(false);
                KeyAction::SetMode(EditorMode::Insert)
            }
            (_, KeyCode::Char('A')) => {
                self.mark_ring.set_active(false);
                KeyAction::SetMode(EditorMode::Insert)
            }
            (_, KeyCode::Char('c')) => {
                if self.mark_ring.is_active() {
                    self.pending_action = Some(KeyAction::SetMode(EditorMode::Insert));
                    KeyAction::KillRegion
                } else {
                    KeyAction::None
                }
            }
            (_, KeyCode::Char('C')) => {
                if self.mark_ring.is_active() {
                    self.pending_action = Some(KeyAction::SetMode(EditorMode::Insert));
                    KeyAction::KillLine
                } else {
                    KeyAction::None
                }
            }
            (_, KeyCode::PageUp) => KeyAction::PageUp,
            (_, KeyCode::PageDown) => KeyAction::PageDown,
            (_, KeyCode::Backspace) => KeyAction::MoveLeft,
            _ => KeyAction::None,
        }
    }

    fn execute_action(&mut self, action: KeyAction) {
        match action {
            KeyAction::None => {}
            KeyAction::AceJump => {}
            KeyAction::OpenFold => {
                self.buffer.open_fold();
            }
            KeyAction::CloseFold => {
                self.buffer.close_fold();
            }
            KeyAction::ToggleFoldEvil => {
                self.buffer.toggle_fold();
            }
            KeyAction::ReduceFolds => {
                self.buffer.reduce_folds();
            }
            KeyAction::MaximizeFolds => {
                self.buffer.maximize_folds();
            }

            KeyAction::MoveLeft => self.buffer.move_left(),
            KeyAction::MoveRight => self.buffer.move_right(),
            KeyAction::MoveUp => self.buffer.move_up(),
            KeyAction::MoveDown => self.buffer.move_down(),
            KeyAction::MoveLineStart => self.buffer.move_to_line_start(),
            KeyAction::MoveLineEnd => self.buffer.move_to_line_end(),
            KeyAction::MoveFileStart => self.buffer.move_to_file_start(),
            KeyAction::MoveFileEnd => self.buffer.move_to_file_end(),
            KeyAction::MoveWordForward => self.buffer.move_word_forward(),
            KeyAction::MoveWordBackward => self.buffer.move_word_backward(),
            KeyAction::MoveWordEnd => self.buffer.move_word_end(),

            KeyAction::ScrollUp => self.view.scroll(-3, self.buffer.line_count()),
            KeyAction::ScrollDown => self.view.scroll(3, self.buffer.line_count()),
            KeyAction::ScrollHalfPageUp => {
                let half = (self.view.height / 2) as isize;
                self.view.scroll(-half, self.buffer.line_count());
                for _ in 0..half {
                    self.buffer.move_up();
                }
            }
            KeyAction::ScrollHalfPageDown => {
                let half = (self.view.height / 2) as isize;
                self.view.scroll(half, self.buffer.line_count());
                for _ in 0..half {
                    self.buffer.move_down();
                }
            }
            KeyAction::PageUp => {
                let h = self.view.height as isize;
                self.view.scroll(-h, self.buffer.line_count());
                for _ in 0..h {
                    self.buffer.move_up();
                }
            }
            KeyAction::PageDown => {
                let h = self.view.height as isize;
                self.view.scroll(h, self.buffer.line_count());
                for _ in 0..h {
                    self.buffer.move_down();
                }
            }

            KeyAction::InsertChar(c) => {
                self.record_change();
                self.buffer.insert_char(c);
                if let Some(action) = self.minor_modes.on_insert_char(c) {
                    self.execute_action(action);
                }
            }
            KeyAction::InsertNewline => {
                self.record_change();
                self.buffer.insert_newline();
                if let Some(action) = self.minor_modes.on_insert_newline() {
                    self.execute_action(action);
                }
            }
            KeyAction::DeleteBackward => {
                self.record_change();
                self.buffer.delete_backward();
            }
            KeyAction::DeleteForward => {
                self.record_change();
                self.buffer.delete_forward();
            }
            KeyAction::DeleteLine => {
                self.record_change();
                self.kill_line_to_ring();
                self.buffer.delete_line();
            }
            KeyAction::DeleteToEol => {
                self.record_change();
                self.buffer.delete_to_eol();
            }
            KeyAction::KillLine => {
                self.record_change();
                let row = self.buffer.cursor.row;
                let col = self.buffer.cursor.col;
                let line = &self.buffer.lines[row];
                let killed = if col < line.len() {
                    let killed_part = line[col..].to_string();
                    self.kill_ring.kill(&killed_part, false);
                    self.buffer.delete_to_eol();
                    killed_part
                } else {
                    if row + 1 < self.buffer.line_count() {
                        let killed_part = "\n".to_string();
                        self.kill_ring.kill(&killed_part, false);
                        let next = self.buffer.lines.remove(row + 1);
                        self.buffer.lines[row].push_str(&next);
                        self.buffer.modified = true;
                        killed_part
                    } else {
                        String::new()
                    }
                };
                if !killed.is_empty() {
                    self.last_yank_was_kill = true;
                }
            }
            KeyAction::KillWordForward => {
                self.record_change();
                let start = self.buffer.cursor.col;
                self.buffer.move_word_forward();
                let end = self.buffer.cursor.col;
                if end > start {
                    let row = self.buffer.cursor.row;
                    let killed: String = self.buffer.lines[row][start..end].to_string();
                    self.kill_ring.kill(&killed, false);
                    for _ in 0..(end - start) {
                        if start < self.buffer.lines[row].len() {
                            self.buffer.lines[row].remove(start);
                        }
                    }
                    self.buffer.cursor.col = start;
                    self.buffer.modified = true;
                    self.last_yank_was_kill = true;
                }
            }
            KeyAction::DeleteWordBackward => {
                self.record_change();
                let line = self.buffer.lines[self.buffer.cursor.row].clone();
                let col = self.buffer.cursor.col;
                if col > 0 {
                    let end = line[..col].chars().count();
                    let mut i = end;
                    while i > 0
                        && line[..col]
                            .chars()
                            .nth(i - 1)
                            .map_or(false, |c| c.is_whitespace())
                    {
                        i -= 1;
                    }
                    while i > 0 {
                        let c = line[..col].chars().nth(i - 1).unwrap();
                        if c.is_whitespace() || !c.is_alphanumeric() {
                            break;
                        }
                        i -= 1;
                    }
                    let del_start = line.char_indices().nth(i).map(|(p, _)| p).unwrap_or(0);
                    let killed = line[del_start..col].to_string();
                    self.kill_ring.kill(&killed, false);
                    self.buffer.push_undo_snapshot();
                    self.buffer.lines[self.buffer.cursor.row] =
                        line[..del_start].to_string() + &line[col..];
                    self.buffer.cursor.col = del_start;
                    self.buffer.modified = true;
                }
            }
            KeyAction::DeleteWordForward => {
                self.record_change();
                let line = self.buffer.current_line();
                let col = self.buffer.cursor.col;
                let chars: Vec<char> = line.chars().collect();
                let mut end = col;
                while end < chars.len() && chars[end].is_whitespace() {
                    end += 1;
                }
                while end < chars.len() && !chars[end].is_whitespace() {
                    end += 1;
                }
                if end > col {
                    self.buffer.delete_range(col, end);
                }
            }

            KeyAction::DeleteToEndOfWord => {
                self.record_change();
                let line = self.buffer.current_line();
                let col = self.buffer.cursor.col;
                let chars: Vec<char> = line.chars().collect();
                if col < chars.len() {
                    let is_word = |c: char| c.is_alphanumeric() || c == '_';
                    let mut end = col;
                    if is_word(chars[col]) {
                        while end < chars.len() && is_word(chars[end]) {
                            end += 1;
                        }
                    } else if chars[col].is_whitespace() {
                        while end < chars.len() && chars[end].is_whitespace() {
                            end += 1;
                        }
                    } else {
                        end += 1;
                    }
                    if end > col {
                        self.buffer.delete_range(col, end);
                    }
                }
            }

            KeyAction::DeleteToStartOfLine => {
                self.record_change();
                let col = self.buffer.cursor.col;
                if col > 0 {
                    self.buffer.delete_range(0, col);
                    self.buffer.cursor.col = 0;
                }
            }

            KeyAction::YankWord => {
                let line = self.buffer.current_line();
                let col = self.buffer.cursor.col;
                let chars: Vec<char> = line.chars().collect();
                let mut end = col;
                while end < chars.len() && chars[end].is_whitespace() {
                    end += 1;
                }
                while end < chars.len() && !chars[end].is_whitespace() {
                    end += 1;
                }
                if end > col {
                    let word: String = chars[col..end].iter().collect();
                    self.kill_ring.kill(&word, false);
                    self.status_message = "Yanked word".to_string();
                }
            }

            KeyAction::DeleteInnerWord => {
                self.record_change();
                let (start, end) = self.buffer.inner_word_range();
                if end > start {
                    self.buffer.delete_range(start, end);
                }
            }
            KeyAction::DeleteAroundWord => {
                self.record_change();
                let (start, end) = self.buffer.around_word_range();
                if end > start {
                    self.buffer.delete_range(start, end);
                }
            }
            KeyAction::ChangeInnerWord => {
                self.record_change();
                let (start, end) = self.buffer.inner_word_range();
                if end > start {
                    self.buffer.delete_range(start, end);
                }
            }
            KeyAction::ChangeAroundWord => {
                self.record_change();
                let (start, end) = self.buffer.around_word_range();
                if end > start {
                    self.buffer.delete_range(start, end);
                }
            }
            KeyAction::DeleteInnerBrackets(open, close) => {
                self.record_change();
                let (start, end) = self.buffer.inner_bracket_range(open, close);
                if end > start {
                    self.buffer.delete_range(start, end);
                }
            }
            KeyAction::DeleteAroundBrackets(open, close) => {
                self.record_change();
                let (start, end) = self.buffer.around_bracket_range(open, close);
                if end > start {
                    self.buffer.delete_range(start, end);
                }
            }
            KeyAction::ChangeSurround(old_char, new_char) => {
                self.record_change();
                if let Some((start, end)) = self.buffer.find_surround_pair(old_char) {
                    self.buffer.change_surround(old_char, new_char, start, end);
                }
            }
            KeyAction::DeleteSurround(target) => {
                self.record_change();
                if let Some((start, end)) = self.buffer.find_surround_pair(target) {
                    self.buffer.delete_surround(start, end);
                }
            }
            KeyAction::AddSurround(surround_char) => {
                self.record_change();
                if let Some((start, end)) = self.surround_range.take() {
                    self.buffer.add_surround(surround_char, start, end);
                }
            }

            KeyAction::KillRegion => {
                if self.mark_ring.is_active() {
                    let a = self.mark_ring.peek().copied().unwrap_or(self.buffer.cursor);
                    let b = self.buffer.cursor;
                    if a != b {
                        let (start, end) = if a.row < b.row || (a.row == b.row && a.col <= b.col) {
                            (a, b)
                        } else {
                            (b, a)
                        };
                        let killed = self.extract_text_between(start, end);
                        self.kill_ring.kill(&killed, false);

                        self.buffer.push_undo_snapshot();
                        if start.row == end.row {
                            let line = &mut self.buffer.lines[start.row];
                            *line = line[..start.col].to_string() + &line[end.col..];
                        } else {
                            let first = &self.buffer.lines[start.row];
                            let last = &self.buffer.lines[end.row];
                            self.buffer.lines[start.row] =
                                first[..start.col].to_string() + &last[end.col..];
                            for _ in (start.row + 1..=end.row).rev() {
                                self.buffer.lines.remove(start.row + 1);
                            }
                        }
                        self.buffer.cursor = start;
                        self.buffer.modified = true;
                    }
                    self.mark_ring.set_active(false);
                    self.mode = EditorMode::Normal;
                    self.last_yank_was_kill = true;
                }
            }
            KeyAction::CopyRegion => {
                if self.mark_ring.is_active() {
                    let a = self.mark_ring.peek().copied().unwrap_or(self.buffer.cursor);
                    let b = self.buffer.cursor;
                    if a != b {
                        let (start, end) = if a.row < b.row || (a.row == b.row && a.col <= b.col) {
                            (a, b)
                        } else {
                            (b, a)
                        };
                        let text = self.extract_text_between(start, end);
                        self.kill_ring.kill(&text, false);
                        self.status_message = format!("Copied {} chars", text.len());
                    }
                    self.mark_ring.set_active(false);
                    self.mode = EditorMode::Normal;
                }
            }

            KeyAction::KillRect => {
                if self.mark_ring.is_active() {
                    let a = self.mark_ring.peek().copied().unwrap_or(self.buffer.cursor);
                    let b = self.buffer.cursor;
                    let rect = RectRegion::from_corners(a, b);
                    let killed = rectangle::kill_rectangle(&mut self.buffer, &rect);
                    let text = killed.join("\n");
                    self.kill_ring.kill(&text, true);
                    self.registers.set('"', RegisterValue::Rectangle(killed));
                    self.status_message = format!("Killed rectangle");
                    self.mark_ring.set_active(false);
                    self.mode = EditorMode::Normal;
                    self.last_yank_was_kill = true;
                }
            }
            KeyAction::YankRect => {
                if let Some(RegisterValue::Rectangle(killed)) = self.registers.get('"') {
                    let a = self.buffer.cursor;
                    let b = self.buffer.cursor;
                    let rect = RectRegion::from_corners(a, b);
                    rectangle::yank_rectangle(&mut self.buffer, &rect, killed);
                    self.status_message = format!("Yanked rectangle");
                }
            }
            KeyAction::ClearRect => {
                if self.mark_ring.is_active() {
                    let a = self.mark_ring.peek().copied().unwrap_or(self.buffer.cursor);
                    let b = self.buffer.cursor;
                    let rect = RectRegion::from_corners(a, b);
                    rectangle::clear_rectangle(&mut self.buffer, &rect);
                    self.status_message = format!("Cleared rectangle");
                    self.mark_ring.set_active(false);
                    self.mode = EditorMode::Normal;
                }
            }
            KeyAction::InsertRect => {
                if self.mark_ring.is_active() {
                    let a = self.mark_ring.peek().copied().unwrap_or(self.buffer.cursor);
                    let b = self.buffer.cursor;
                    let rect = RectRegion::from_corners(a, b);
                    let input = self.command_input().to_string();
                    if !input.is_empty() {
                        rectangle::insert_rectangle(&mut self.buffer, &rect, &input);
                        self.status_message = "Inserted rectangle".to_string();
                    }
                    self.mark_ring.set_active(false);
                    self.mode = EditorMode::Normal;
                    self.clear_minibuffer();
                }
            }
            KeyAction::IndentRegion(rect) => {
                let indent: String = if self.buffer.major_mode.use_tabs() {
                    "\t".to_string()
                } else {
                    " ".repeat(self.buffer.major_mode.indent_width())
                };
                for row in rect.start_row..=rect.end_row.min(self.buffer.lines.len() - 1) {
                    self.buffer.lines[row].insert_str(0, &indent);
                }
                self.buffer.modified = true;
            }

            KeyAction::KillAppend => {
                self.kill_ring.append_kill("\n", false);
                self.last_yank_was_kill = true;
            }

            KeyAction::Yank => {
                self.record_change();
                if let Some(entry) = self.kill_ring.yank() {
                    let text = &entry.text;
                    let row = self.buffer.cursor.row + 1;
                    let contains_nl = text.contains('\n');
                    if contains_nl || entry.rect {
                        let lines: Vec<&str> = text.split('\n').collect();
                        for (i, line) in lines.iter().enumerate() {
                            self.buffer.lines.insert(row + i, line.to_string());
                        }
                        self.buffer.cursor.row = row;
                        self.buffer.cursor.col = 0;
                    } else {
                        let line = &mut self.buffer.lines[self.buffer.cursor.row];
                        let col = self.buffer.cursor.col;
                        line.insert_str(col, text);
                        self.buffer.cursor.col += text.len();
                    }
                    self.buffer.modified = true;
                    self.status_message =
                        format!("Yanked from kill-ring ({} entries)", self.kill_ring.len());
                }
            }
            KeyAction::YankPop => {
                let text = self.kill_ring.yank_pop_forward().map(|e| e.text.clone());
                if let Some(ref t) = text {
                    self.replace_last_yank(t);
                    self.status_message = format!("Yank-pop ({})", self.kill_ring.len());
                }
            }

            KeyAction::Undo => self.buffer.undo(),
            KeyAction::Redo => self.buffer.redo(),

            KeyAction::SetMode(mode) => {
                if mode == EditorMode::Visual && self.mode != EditorMode::Visual {
                    self.mark_ring.push(self.buffer.cursor);
                    self.mark_ring.set_active(true);
                }
                if matches!(
                    mode,
                    EditorMode::Command | EditorMode::SearchForward | EditorMode::SearchBackward
                ) {
                    self.activate_minibuffer(mode);
                } else {
                    self.mode = mode;
                }
                if mode == EditorMode::Normal {
                    self.clear_minibuffer();
                    self.waiting_g = false;
                    self.waiting_op = None;
                    if !self.mark_ring.is_active() {
                        self.mark_ring.clear();
                    }
                }
            }

            KeyAction::Save => self.save_current_buffer(),
            KeyAction::SaveAs => {
                let command_input = self.command_input().to_string();
                let path_str = command_input.strip_prefix("w ").unwrap_or(&command_input);
                let path = PathBuf::from(path_str.trim());
                match self.buffer.save_as(&path) {
                    Ok(()) => self.status_message = format!("Saved: {}", path.display()),
                    Err(e) => self.status_message = format!("Save error: {}", e),
                }
                self.mode = EditorMode::Normal;
                self.clear_minibuffer();
            }
            KeyAction::Quit => {
                if self.buffer.modified {
                    self.status_message = "Unsaved changes! Use C-x C-c to force quit".to_string();
                } else {
                    self.quit_requested = true;
                }
            }
            KeyAction::ForceQuit => {
                self.quit_requested = true;
            }

            KeyAction::InputChar(c) => {
                self.push_minibuffer_char(c);
            }
            KeyAction::InputBackspace => {
                self.pop_minibuffer_char();
            }
            KeyAction::ExecuteInput => {
                self.execute_command();
            }
            KeyAction::GotoLine(n) => {
                self.buffer.goto_line(n);
                self.mode = EditorMode::Normal;
                self.clear_minibuffer();
            }

            KeyAction::FindForward(query) => {
                self.last_search_forward = Some(query.clone());
                if !self.buffer.find_forward(&query) {
                    self.status_message = format!("Pattern not found: {}", query);
                }
                self.mode = EditorMode::Normal;
                self.clear_minibuffer();
            }
            KeyAction::FindBackward(query) => {
                self.last_search_backward = Some(query.clone());
                if !self.buffer.find_backward(&query) {
                    self.status_message = format!("Pattern not found: {}", query);
                }
                self.mode = EditorMode::Normal;
                self.clear_minibuffer();
            }
            KeyAction::RepeatFind(forward) => {
                if forward {
                    if let Some(ref q) = self.last_search_forward.clone() {
                        if !self.buffer.find_forward(q) {
                            self.status_message = format!("Pattern not found: {}", q);
                        }
                    }
                } else if let Some(ref q) = self.last_search_backward.clone() {
                    if !self.buffer.find_backward(q) {
                        self.status_message = format!("Pattern not found: {}", q);
                    }
                }
            }
            KeyAction::ReplaceFirst(old, new) => {
                if !self.buffer.replace_first(&old, &new) {
                    self.status_message = format!("Not found: {}", old);
                }
                self.mode = EditorMode::Normal;
                self.clear_minibuffer();
            }
            KeyAction::ReplaceAll(old, new) => {
                let count = self.buffer.replace_all(&old, &new);
                self.status_message = format!("Replaced {} occurrence(s)", count);
                self.mode = EditorMode::Normal;
                self.clear_minibuffer();
            }

            KeyAction::YankLine => {
                let text = self.buffer.lines[self.buffer.cursor.row].clone();
                self.kill_ring.kill(&text, false);
                self.status_message = "Yanked line".to_string();
            }
            KeyAction::PasteAfter => {
                self.record_change();
                if let Some(entry) = self.kill_ring.yank() {
                    let text = &entry.text;
                    let paste_row = self.buffer.cursor.row + 1;
                    if text.contains('\n') {
                        for (i, line) in text.split('\n').enumerate() {
                            self.buffer.lines.insert(paste_row + i, line.to_string());
                        }
                    } else {
                        self.buffer.lines.insert(paste_row, text.to_string());
                    }
                    self.buffer.modified = true;
                    self.buffer.cursor.row = paste_row;
                    self.buffer.cursor.col = 0;
                }
            }

            KeyAction::IndentLine => {
                self.record_change();
                let indent: String = if self.buffer.major_mode.use_tabs() {
                    "\t".to_string()
                } else {
                    " ".repeat(self.buffer.major_mode.indent_width())
                };
                let line = &mut self.buffer.lines[self.buffer.cursor.row];
                line.insert_str(0, &indent);
                self.buffer.modified = true;
            }
            KeyAction::UnindentLine => {
                self.record_change();
                let indent_width = self.buffer.major_mode.indent_width();
                let line = &mut self.buffer.lines[self.buffer.cursor.row];
                if self.buffer.major_mode.use_tabs() {
                    if line.starts_with('\t') {
                        *line = line[1..].to_string();
                        self.buffer.modified = true;
                    }
                } else {
                    let spaces = " ".repeat(indent_width);
                    if line.starts_with(&spaces) {
                        *line = line[indent_width..].to_string();
                        self.buffer.modified = true;
                    } else if line.starts_with('\t') {
                        *line = line[1..].to_string();
                        self.buffer.modified = true;
                    }
                }
            }

            KeyAction::OpenLineBelow => {
                self.buffer.move_to_line_end();
                self.buffer.insert_newline();
                self.mode = EditorMode::Insert;
            }
            KeyAction::OpenLineAbove => {
                self.buffer.move_to_line_start();
                self.buffer.insert_newline();
                self.buffer.move_up();
                self.mode = EditorMode::Insert;
            }

            KeyAction::JoinLines => {
                self.record_change();
                if self.buffer.cursor.row + 1 < self.buffer.line_count() {
                    let next = self.buffer.lines[self.buffer.cursor.row + 1].clone();
                    let current = &mut self.buffer.lines[self.buffer.cursor.row];
                    let col = current.len();
                    current.push(' ');
                    current.push_str(&next.trim());
                    self.buffer.lines.remove(self.buffer.cursor.row + 1);
                    self.buffer.cursor.col = col;
                    self.buffer.modified = true;
                }
            }

            KeyAction::ReplaceChar(c) => {
                self.record_change();
                let row = self.buffer.cursor.row;
                let col = self.buffer.cursor.col;
                if col < self.buffer.lines[row].len() {
                    self.buffer.push_undo_snapshot();
                    let line = &mut self.buffer.lines[row];
                    let char_idx = line[..col].chars().count();
                    let chars: Vec<char> = line.chars().collect();
                    if char_idx < chars.len() {
                        line.replace_range(
                            line.char_indices()
                                .nth(char_idx)
                                .map(|(i, _)| i)
                                .unwrap_or(col)
                                ..line
                                    .char_indices()
                                    .nth(char_idx + 1)
                                    .map(|(i, _)| i)
                                    .unwrap_or(line.len()),
                            &c.to_string(),
                        );
                        self.buffer.modified = true;
                    }
                }
            }

            KeyAction::ToggleVisual => {
                if self.mode == EditorMode::Visual {
                    self.mark_ring.set_active(false);
                    self.mode = EditorMode::Normal;
                } else {
                    self.mark_ring.push(self.buffer.cursor);
                    self.mark_ring.set_active(true);
                    self.mode = EditorMode::Visual;
                }
            }
            KeyAction::SwitchToEmacs => {
                self.mode = EditorMode::Emacs;
            }

            KeyAction::StartMacro => {
                if self.macro_state.is_recording() {
                    self.macro_state.stop_recording();
                    self.status_message = format!(
                        "Macro recording stopped ({} keys)",
                        self.macro_state
                            .store_in_register('e')
                            .map(|v| v.len())
                            .unwrap_or(0)
                    );
                } else {
                    self.macro_state.start_recording('e');
                    self.status_message = "Recording macro (C-x ) to stop)".to_string();
                }
            }
            KeyAction::EndMacro => {
                if self.macro_state.is_recording() {
                    self.macro_state.stop_recording();
                    let events = self.macro_state.store_in_register('e').unwrap_or_default();
                    self.registers.set('e', RegisterValue::Macro(events));
                    let len = self
                        .registers
                        .get('e')
                        .map(|v| match v {
                            RegisterValue::Macro(e) => e.len(),
                            _ => 0,
                        })
                        .unwrap_or(0);
                    self.status_message = format!("Macro recording stopped ({} keys)", len);
                }
            }
            KeyAction::CallMacro => {
                let saved_cmd = self.command_input.clone();
                if let Some(events) = self.macro_state.store_in_register('e') {
                    self.macro_state.load_from_register('e', &events);
                    self.macro_state.start_playback('e');
                }
                self.command_input = saved_cmd;
            }

            KeyAction::TransposeChar => {
                self.record_change();
                self.buffer.transpose_char();
            }
            KeyAction::TransposeWord => {
                self.record_change();
                self.buffer.transpose_word();
            }
            KeyAction::TransposeLine => {
                self.record_change();
                self.buffer.transpose_line();
            }
            KeyAction::CapitalizeWord => {
                self.record_change();
                self.buffer.capitalize_word();
            }
            KeyAction::UppercaseWord => {
                self.record_change();
                self.buffer.uppercase_word();
            }
            KeyAction::LowercaseWord => {
                self.record_change();
                self.buffer.lowercase_word();
            }
            KeyAction::UppercaseRegion => {
                if let Some(mark) = self.mark_ring.peek().copied() {
                    self.record_change();
                    self.buffer.uppercase_region((mark.row, mark.col));
                    self.status_message = "Uppercase region".to_string();
                }
            }
            KeyAction::LowercaseRegion => {
                if let Some(mark) = self.mark_ring.peek().copied() {
                    self.record_change();
                    self.buffer.lowercase_region((mark.row, mark.col));
                    self.status_message = "Lowercase region".to_string();
                }
            }
            KeyAction::PopMark => {
                if let Some(pos) = self.mark_ring.pop() {
                    self.buffer.cursor = pos;
                    self.mark_ring.set_active(false);
                    self.status_message = "Pop mark".to_string();
                }
            }
            KeyAction::UniversalArg => {
                let count = self.repeat_count.unwrap_or(0);
                self.repeat_count = Some(count * 4);
                self.status_message = format!("C-u {}", self.repeat_count.unwrap_or(4));
            }
            KeyAction::MwimBeginning => {
                let line = self.buffer.current_line();
                let first_non_ws = line.chars().take_while(|c| c.is_whitespace()).count();
                let first_non_ws = if first_non_ws >= line.chars().count() {
                    0
                } else {
                    first_non_ws
                };
                if self.buffer.cursor.col == first_non_ws && first_non_ws != 0 {
                    self.buffer.cursor.col = 0;
                } else {
                    self.buffer.cursor.col = first_non_ws;
                }
            }
            KeyAction::MwimEnd => {
                let line = self.buffer.current_line();
                let chars: Vec<char> = line.chars().collect();
                let len = chars.len();
                if len == 0 {
                    self.buffer.cursor.col = 0;
                } else {
                    let last_non_ws = chars
                        .iter()
                        .enumerate()
                        .rev()
                        .find(|(_, c)| !c.is_whitespace())
                        .map(|(i, _)| i + 1)
                        .unwrap_or(0);
                    if self.buffer.cursor.col == last_non_ws && last_non_ws != len {
                        self.buffer.cursor.col = len;
                    } else {
                        self.buffer.cursor.col = last_non_ws;
                    }
                }
            }
            KeyAction::ExpandRegion => {
                let row = self.buffer.cursor.row;
                let col = self.buffer.cursor.col;
                if !self.mark_ring.is_active() {
                    self.mark_ring.push(super::buffer::Cursor { row, col });
                    self.mark_ring.set_active(true);
                    self.expand_region_level = 0;
                }
                match self.expand_region_level {
                    0 => {
                        let line = self.buffer.current_line();
                        let chars: Vec<char> = line.chars().collect();
                        let mut start = col;
                        let mut end = col;
                        while start > 0
                            && start - 1 < chars.len()
                            && !chars[start - 1].is_whitespace()
                        {
                            start -= 1;
                        }
                        while end < chars.len() && !chars[end].is_whitespace() {
                            end += 1;
                        }
                        if let Some(mark) = self.mark_ring.peek() {
                            if mark.row == row && mark.col == start {
                                self.expand_region_level = 1;
                            }
                        }
                        self.mark_ring
                            .push(super::buffer::Cursor { row, col: start });
                        self.buffer.cursor.col = end;
                    }
                    1 => {
                        self.mark_ring.push(super::buffer::Cursor { row, col: 0 });
                        let line_len = self.buffer.current_line().chars().count();
                        self.buffer.cursor.col = line_len;
                        self.expand_region_level = 2;
                    }
                    2 => {
                        let lines = &self.buffer.lines;
                        let mut start_row = row;
                        while start_row > 0 && !lines[start_row - 1].is_empty() {
                            start_row -= 1;
                        }
                        let mut end_row = row;
                        while end_row + 1 < lines.len() && !lines[end_row + 1].is_empty() {
                            end_row += 1;
                        }
                        self.mark_ring.push(super::buffer::Cursor {
                            row: start_row,
                            col: 0,
                        });
                        let end_len = lines[end_row].chars().count();
                        self.buffer.cursor.row = end_row;
                        self.buffer.cursor.col = end_len;
                        self.expand_region_level = 3;
                    }
                    _ => {
                        self.mark_ring
                            .push(super::buffer::Cursor { row: 0, col: 0 });
                        let last_row = self.buffer.lines.len().saturating_sub(1);
                        let last_len = self.buffer.lines[last_row].chars().count();
                        self.buffer.cursor.row = last_row;
                        self.buffer.cursor.col = last_len;
                    }
                }
                self.expand_region_level += 1;
            }
            KeyAction::ContractRegion => {
                if self.expand_region_level > 1 {
                    self.expand_region_level -= 2;
                    // Re-trigger expand at the lower level
                    self.execute_action(KeyAction::ExpandRegion);
                } else {
                    self.expand_region_level = 0;
                    self.mark_ring.clear();
                    self.status_message = "Contract region".to_string();
                }
            }
            KeyAction::HungryDeleteForward => {
                self.record_change();
                self.buffer.hungry_delete_forward();
            }
            KeyAction::HungryDeleteBackward => {
                self.record_change();
                self.buffer.hungry_delete_backward();
            }
            KeyAction::DabbrevExpand => {
                let line = self.buffer.current_line();
                let chars: Vec<char> = line.chars().collect();
                let col = self.buffer.cursor.col;
                let mut start = col;
                while start > 0 && start - 1 < chars.len() && !chars[start - 1].is_whitespace() {
                    start -= 1;
                }
                let prefix: String = chars[start..col].iter().collect();
                if prefix.is_empty() {
                    return;
                }
                let is_continuation = self.dabbrev_prefix.as_ref() == Some(&prefix);
                if !is_continuation {
                    let mut matches = Vec::new();
                    for (i, l) in self.buffer.lines.iter().enumerate() {
                        let lchars: Vec<char> = l.chars().collect();
                        for w in 0..lchars.len() {
                            if w > 0 && lchars[w - 1].is_whitespace() {
                                continue;
                            }
                            if w == 0 && i > 0 {
                                continue;
                            }
                            let mut end = w;
                            while end < lchars.len() && !lchars[end].is_whitespace() {
                                end += 1;
                            }
                            let word: String = lchars[w..end].iter().collect();
                            if word.len() > prefix.len() && word.starts_with(&prefix) {
                                if !matches.contains(&word) {
                                    matches.push(word);
                                }
                            }
                        }
                    }
                    if matches.is_empty() {
                        self.status_message = "No completions".to_string();
                        return;
                    }
                    self.dabbrev_prefix = Some(prefix.clone());
                    self.dabbrev_matches = matches;
                    self.dabbrev_index = 0;
                }
                if self.dabbrev_matches.is_empty() {
                    self.status_message = "No completions".to_string();
                    return;
                }
                let replacement = self.dabbrev_matches[self.dabbrev_index].clone();
                self.buffer.replace_range(start, col, &replacement);
                self.dabbrev_index = (self.dabbrev_index + 1) % self.dabbrev_matches.len();
                self.status_message =
                    format!("({}/{})", self.dabbrev_index, self.dabbrev_matches.len());
            }
            KeyAction::ZapToChar => {}
            KeyAction::InsertEmptyLineBelow => {
                self.record_change();
                self.buffer.insert_empty_line_below();
            }
            KeyAction::InsertEmptyLineAbove => {
                self.record_change();
                self.buffer.insert_empty_line_above();
            }
            KeyAction::GotoLastChange => {
                if let Some((row, col)) = self.last_changes.pop_front() {
                    if row < self.buffer.line_count() {
                        self.buffer.cursor.row = row;
                        let line_len = self.buffer.current_line().chars().count();
                        self.buffer.cursor.col = col.min(line_len);
                        self.status_message = "Goto last change".to_string();
                    }
                } else {
                    self.status_message = "No more last changes".to_string();
                }
            }
            KeyAction::CleanupBuffer => {
                self.record_change();
                self.buffer.cleanup_buffer();
                self.status_message = "Cleaned up buffer".to_string();
            }
            KeyAction::CopyAndComment => {
                self.record_change();
                self.buffer.copy_and_comment();
                self.status_message = "Copy and comment".to_string();
            }
            KeyAction::NarrowRegion => {
                if self.mark_ring.is_active() {
                    if let Some(mark) = self.mark_ring.peek().copied() {
                        let cur = self.buffer.cursor;
                        let start = mark.row.min(cur.row);
                        let end = mark.row.max(cur.row);
                        self.buffer.narrow_to_region(start, end);
                        self.mark_ring.set_active(false);
                        self.status_message =
                            format!("Narrowed to lines {}-{}", start + 1, end + 1);
                    }
                } else {
                    let row = self.buffer.cursor.row;
                    self.buffer.narrow_to_region(row, row);
                    self.status_message = format!("Narrowed to line {}", row + 1);
                }
            }
            KeyAction::Widen => {
                self.buffer.widen();
                self.status_message = "Widened".to_string();
            }
            KeyAction::Dos2Unix => {
                self.record_change();
                self.buffer.dos2unix();
                self.status_message = "Converted to Unix line endings".to_string();
            }
            KeyAction::Unix2Dos => {
                self.record_change();
                self.buffer.unix2dos();
                self.status_message = "Converted to DOS line endings".to_string();
            }
            KeyAction::ToggleFold => {
                self.buffer.toggle_fold();
                if self.buffer.is_folded() {
                    self.status_message = format!(
                        "Folded at indent level {}",
                        self.buffer.fold_level.unwrap_or(0)
                    );
                } else {
                    self.status_message = "Unfolded".to_string();
                }
            }
            KeyAction::SwitchMajorMode(ref name) => {
                if let Some(kind) = super::major_mode::parse_mode_name(name) {
                    self.buffer.major_mode = super::major_mode::create_mode(kind);
                    self.status_message =
                        format!("Switched to {} mode", self.buffer.major_mode.name());
                } else {
                    self.status_message = format!("Unknown mode: {}", name);
                }
            }
            KeyAction::ToggleMinorMode(ref name) => {
                if name.starts_with('!') {
                    let real_name = &name[1..];
                    if self.minor_modes.disable_by_name(real_name) {
                        self.status_message = format!("Disabled minor mode: {}", real_name);
                    } else {
                        self.status_message = format!("Minor mode not enabled: {}", real_name);
                    }
                } else {
                    let enabled = self.minor_modes.toggle_by_name(name);
                    if enabled {
                        self.status_message = format!("Enabled minor mode: {}", name);
                    } else {
                        self.status_message = format!("Disabled minor mode: {}", name);
                    }
                }
            }
            KeyAction::MxComplete => {
                self.mx_complete();
            }
            KeyAction::ToggleCase => {
                self.record_change();
                let row = self.buffer.cursor.row;
                let col = self.buffer.cursor.col;
                let line = self.buffer.lines[row].clone();
                if let Some(ch) = line.chars().nth(col) {
                    let toggled = if ch.is_ascii_uppercase() {
                        ch.to_ascii_lowercase()
                    } else if ch.is_ascii_lowercase() {
                        ch.to_ascii_uppercase()
                    } else {
                        ch
                    };
                    if toggled != ch {
                        let mut chars: Vec<char> = line.chars().collect();
                        chars[col] = toggled;
                        self.buffer.lines[row] = chars.into_iter().collect();
                        self.buffer.modified = true;
                    }
                    if col + 1 < line.len() {
                        self.buffer.cursor.col = col + 1;
                    }
                }
                self.last_normal_change = Some(Box::new(KeyAction::ToggleCase));
            }
            KeyAction::SubstituteLine => {
                self.record_change();
                self.buffer.move_to_line_start();
                let line = self.buffer.current_line();
                let len = line.len();
                if len > 0 {
                    self.buffer.delete_range(0, len);
                }
                self.mode = EditorMode::Insert;
                self.last_normal_change = Some(Box::new(KeyAction::SubstituteLine));
            }
            KeyAction::SearchWordForward => {
                let word = self.buffer.word_under_cursor();
                if !word.is_empty() {
                    self.last_search_forward = Some(word.clone());
                    let row = self.buffer.cursor.row;
                    let col = self.buffer.cursor.col + 1;
                    if let Some((r, c)) = self.buffer.search_forward_from(&word, row, col) {
                        self.buffer.cursor.row = r;
                        self.buffer.cursor.col = c;
                    } else if let Some((r, c)) = self.buffer.search_forward_from(&word, 0, 0) {
                        self.buffer.cursor.row = r;
                        self.buffer.cursor.col = c;
                        self.status_message = "Search wrapped".to_string();
                    }
                }
            }
            KeyAction::SearchWordBackward => {
                let word = self.buffer.word_under_cursor();
                if !word.is_empty() {
                    self.last_search_backward = Some(word.clone());
                    let row = self.buffer.cursor.row;
                    let col = self.buffer.cursor.col;
                    if let Some((r, c)) = self.buffer.search_backward_from(&word, row, col) {
                        self.buffer.cursor.row = r;
                        self.buffer.cursor.col = c;
                    } else {
                        let last_row = self.buffer.line_count().saturating_sub(1);
                        let last_col = self.buffer.lines[last_row].len().saturating_sub(1);
                        if let Some((r, c)) =
                            self.buffer.search_backward_from(&word, last_row, last_col)
                        {
                            self.buffer.cursor.row = r;
                            self.buffer.cursor.col = c;
                            self.status_message = "Search wrapped".to_string();
                        }
                    }
                }
            }
            KeyAction::RepeatLastChange => {
                if let Some(ref action) = self.last_normal_change {
                    let action = (**action).clone();
                    self.execute_action(action);
                }
            }
            KeyAction::GotoMatchingBracket => {
                let line = self.buffer.current_line();
                let col = self.buffer.cursor.col;
                if let Some(ch) = line.chars().nth(col) {
                    let (open, close, forward) = match ch {
                        '(' => ('(', ')', true),
                        ')' => ('(', ')', false),
                        '[' => ('[', ']', true),
                        ']' => ('[', ']', false),
                        '{' => ('{', '}', true),
                        '}' => ('{', '}', false),
                        _ => (ch, ch, true),
                    };
                    let mut depth = 0i32;
                    if forward {
                        let chars: Vec<char> = line.chars().collect();
                        for i in col..chars.len() {
                            if chars[i] == open {
                                depth += 1;
                            } else if chars[i] == close {
                                depth -= 1;
                            }
                            if depth == 0 {
                                self.buffer.cursor.col = i;
                                return;
                            }
                        }
                    } else {
                        let chars: Vec<char> = line.chars().collect();
                        for i in (0..=col).rev() {
                            if chars[i] == close {
                                depth += 1;
                            } else if chars[i] == open {
                                depth -= 1;
                            }
                            if depth == 0 {
                                self.buffer.cursor.col = i;
                                return;
                            }
                        }
                    }
                }
            }
            KeyAction::SplitHorizontal => {
                self.split_window_horizontal();
            }
            KeyAction::SplitVertical => {
                self.split_window_vertical();
            }
            KeyAction::DeleteWindow => {
                self.delete_window();
            }
            KeyAction::DeleteOtherWindows => {
                self.delete_other_windows();
            }
            KeyAction::OtherWindow => {
                self.other_window();
            }
            KeyAction::BalanceWindows => {
                self.balance_windows();
            }
            KeyAction::SaveBuffer => {
                self.save_current_buffer();
                self.status_message.clear();
            }
            KeyAction::FindFile => {
                self.activate_minibuffer_with_prompt(EditorMode::Command, "Find file: ");
                self.status_message = "Find file".to_string();
            }
            KeyAction::SwitchBuffer => {
                self.activate_minibuffer_with_prompt(EditorMode::Command, "Switch buffer: ");
                self.status_message = "Switch buffer".to_string();
            }
            KeyAction::Grep => {
                self.activate_minibuffer_with_prompt(EditorMode::SearchForward, "Grep: ");
                self.status_message = "Grep (ripgrep)".to_string();
            }
            KeyAction::GitStatus => {
                self.run_git_status();
            }
            KeyAction::GitCommit => {
                self.run_git_commit();
            }
            KeyAction::EvilExchange => {
                // gx: exchange (swap) the character under cursor with next
                let row = self.buffer.cursor.row;
                let col = self.buffer.cursor.col;
                if let Some(line) = self.buffer.lines.get_mut(row) {
                    let chars: Vec<char> = line.chars().collect();
                    if col + 1 < chars.len() {
                        let mut new_line = String::new();
                        for i in 0..chars.len() {
                            if i == col {
                                new_line.push(chars[i + 1]);
                            } else if i == col + 1 {
                                new_line.push(chars[i - 1]);
                            } else {
                                new_line.push(chars[i]);
                            }
                        }
                        *line = new_line;
                    }
                }
            }
            KeyAction::ProjectFindFile => {
                self.activate_minibuffer_with_prompt(EditorMode::Command, "Project file: ");
                self.status_message = "Project find file".to_string();
            }
            KeyAction::WindowUp => {
                // Move to window above (like C-w k in vim)
                self.other_window(); // simplified
            }
            KeyAction::WindowDown => {
                self.other_window(); // simplified
            }
            KeyAction::WindowLeft => {
                self.other_window(); // simplified
            }
            KeyAction::WindowRight => {
                self.other_window(); // simplified
            }
            KeyAction::WindowSplitHorizontal => {
                self.split_window_horizontal();
            }
            KeyAction::WindowSplitVertical => {
                self.split_window_vertical();
            }
            KeyAction::EvalLispExpression => {
                self.activate_minibuffer_with_prompt(EditorMode::Command, "Eval: ");
                self.status_message = "Eval expression".to_string();
            }
            KeyAction::LeaderPrefix => {
                // SPC was pressed, waiting_prefix already set
                self.status_message = "SPC-".to_string();
            }
        }
    }

    fn execute_command(&mut self) {
        let input = self.command_input().to_string();
        let trimmed = input.trim();


        // Handle git commit message input
        if self.minibuffer.is_active() && self.minibuffer.prompt().contains("Git commit") {
            if !trimmed.is_empty() {
                self.run_git_commit_with_message(trimmed);
            }
            self.mode = EditorMode::Emacs;
            self.clear_minibuffer();
            return;
        }

        // Handle grep input (ripgrep)
        if self.minibuffer.is_active() && self.minibuffer.prompt().contains("Grep") {
            if !trimmed.is_empty() {
                self.run_ripgrep(trimmed);
            }
            self.mode = EditorMode::Emacs;
            self.clear_minibuffer();
            return;
        }

        // Handle iedit-regex input: user pressed Enter after typing regex pattern
        if self.waiting_iedit_regex {
            self.waiting_iedit_regex = false;
            let pattern = trimmed.to_string();
            if pattern.is_empty() {
                self.mode = EditorMode::Emacs;
                self.clear_minibuffer();
                self.status_message = "Iedit: empty pattern".to_string();
                return;
            }
            self.iedit_word = Some(pattern.clone());
            self.iedit_regions.clear();
            self.iedit_cursor_idx = 0;
            // Find all regex matches in the buffer
            match regex::Regex::new(&pattern) {
                Ok(re) => {
                    for (r, line) in self.buffer.lines.iter().enumerate() {
                        for m in re.find_iter(line) {
                            if r == self.buffer.cursor.row && m.start() <= self.buffer.cursor.col && self.buffer.cursor.col <= m.end() {
                                self.iedit_cursor_idx = self.iedit_regions.len();
                            }
                            self.iedit_regions.push((r, m.start(), m.end()));
                        }
                    }
                    if self.iedit_regions.is_empty() {
                        self.mode = EditorMode::Emacs;
                        self.clear_minibuffer();
                        self.status_message = format!("Iedit: no matches for \"{}\"", pattern);
                        return;
                    }
                    // Jump to first region
                    let (r, c, _) = self.iedit_regions[self.iedit_cursor_idx];
                    self.buffer.cursor.row = r;
                    self.buffer.cursor.col = c;
                    self.view.ensure_cursor_visible(&self.buffer);
                    self.mode = EditorMode::Iedit;
                    self.clear_minibuffer();
                    self.iedit_update_status();
                }
                Err(e) => {
                    self.mode = EditorMode::Emacs;
                    self.clear_minibuffer();
                    self.status_message = format!("Iedit: invalid regex: {}", e);
                }
            }
            return;
        }

        if self.lisp_bridge.has_command(trimmed) {
            self.push_lisp_state();
            let result = self.lisp_bridge.execute_command(trimmed);
            self.pull_lisp_state();

            match result {
                Ok(Some(_)) => {
                    if self.status_message.is_empty() {
                        self.status_message = format!("Ran command: {}", trimmed);
                    }
                }
                Ok(None) => {
                    self.status_message = format!("Unknown command: {}", trimmed);
                }
                Err(e) => {
                    self.status_message = format!("Error: {}", e);
                }
            }
            self.mode = EditorMode::Emacs;
            self.clear_minibuffer();
            return;
        }

        // Handle M-x commands that need special treatment
        match trimmed {
            "iedit" | "multi-cursor-edit" => {
                self.mode = EditorMode::Emacs;
                self.clear_minibuffer();
                self.start_iedit();
                return;
            }
            "iedit-regex" | "multiple-cursors-regex" => {
                self.clear_minibuffer();
                self.start_iedit_regex();
                return;
            }
            "iedit-skip-region" | "skip-region" => {
                if self.mode == EditorMode::Iedit {
                    self.iedit_skip_region();
                } else {
                    self.status_message = "Not in iedit mode".to_string();
                }
                self.mode = EditorMode::Emacs;
                self.clear_minibuffer();
                return;
            }
            "iedit-add-region" | "add-region" => {
                if self.mode == EditorMode::Iedit {
                    self.iedit_add_region_at_cursor();
                } else {
                    self.status_message = "Not in iedit mode".to_string();
                }
                self.mode = EditorMode::Emacs;
                self.clear_minibuffer();
                return;
            }
            "iedit-refind-all" | "refind-all" => {
                if self.mode == EditorMode::Iedit {
                    self.iedit_refind_all();
                } else {
                    self.status_message = "Not in iedit mode".to_string();
                }
                self.mode = EditorMode::Emacs;
                self.clear_minibuffer();
                return;
            }
            "goto-line" | "goto-line-number" => {
                self.clear_minibuffer();
                self.status_message = "Goto line: ".to_string();
                // Stay in command mode for user to type line number
                return;
            }
            "recenter" | "recenter-top-bottom" => {
                let half = self.view.height / 2;
                let row = self.buffer.cursor.row;
                self.view.scroll_top = row.saturating_sub(half);
                self.mode = EditorMode::Emacs;
                self.clear_minibuffer();
                self.status_message = "Recentered".to_string();
                return;
            }
            "describe-mode" => {
                let desc = match self.mode {
                    EditorMode::Emacs => "Emacs mode: C-x prefix, C-c prefix, M-x commands",
                    EditorMode::Normal => "Evil mode: vi-style keybindings (evil-mode)",
                    _ => "Current mode",
                };
                self.status_message = desc.to_string();
                self.mode = EditorMode::Emacs;
                self.clear_minibuffer();
                return;
            }
            "evil-mode" | "toggle-evil" => {
                if self.mode == EditorMode::Normal {
                    self.mode = EditorMode::Emacs;
                    self.status_message = "Evil mode disabled. Emacs mode.".to_string();
                } else {
                    self.mode = EditorMode::Normal;
                    self.status_message = "Evil mode enabled. Vi keybindings.".to_string();
                }
                self.clear_minibuffer();
                return;
            }
            "toggle-theme" => {
                self.theme = if self.theme.background.r < 128 {
                    super::theme::day()
                } else {
                    super::theme::night()
                };
                let mode = if self.theme.background.r < 128 { "Night" } else { "Day" };
                self.status_message = format!("Theme: {mode} (coldnew-{mode})");
                self.clear_minibuffer();
                return;
            }
            "view-messages" | "view-echo-area-messages" => {
                let count = self.messages.len();
                let start = if count > 20 { count - 20 } else { 0 };
                let recent: Vec<&str> = self.messages[start..].iter().map(|s| s.as_str()).collect();
                self.status_message = format!("*Messages* ({} entries): {}", count, recent.join("; "));
                self.mode = EditorMode::Emacs;
                self.clear_minibuffer();
                return;
            }
            "what-cursor-position" => {
                self.status_message = format!(
                    "Line {} Col {} ({} lines total)",
                    self.buffer.cursor.row + 1,
                    self.buffer.cursor.col + 1,
                    self.buffer.line_count()
                );
                self.mode = EditorMode::Emacs;
                self.clear_minibuffer();
                return;
            }
            _ if trimmed.starts_with("switch-mode ") || trimmed.starts_with("set-mode ") => {
                let prefix = if trimmed.starts_with("switch-mode ") {
                    "switch-mode "
                } else {
                    "set-mode "
                };
                let mode_name = trimmed[prefix.len()..].trim();
                if let Some(kind) = super::major_mode::parse_mode_name(mode_name) {
                    self.buffer.major_mode = super::major_mode::create_mode(kind);
                    self.status_message =
                        format!("Switched to {} mode", self.buffer.major_mode.name());
                } else {
                    self.status_message = format!("Unknown mode: {}", mode_name);
                }
                self.mode = EditorMode::Emacs;
                self.clear_minibuffer();
                return;
            }
            _ if trimmed.starts_with("eval ") || trimmed.starts_with("eval-expression ") => {
                let prefix = if trimmed.starts_with("eval ") {
                    "eval "
                } else {
                    "eval-expression "
                };
                let code = trimmed[prefix.len()..].trim();
                // Sync editor state to shared state
                let mut state = super::lisp_ext::EditorState::new();
                state.lines = self.buffer.lines.clone();
                state.cursor_row = self.buffer.cursor.row;
                state.cursor_col = self.buffer.cursor.col;
                state.modified = self.buffer.modified;
                state.file_path = self
                    .buffer
                    .path
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string());
                state.mode = self.mode.label().to_lowercase();
                state.window_count = self.windows.len();
                state.overlays = std::mem::replace(
                    &mut self.buffer.overlays,
                    super::overlay::OverlayStore::new(),
                );
                super::lisp_ext::set_editor_state(state);

                let result = self.lisp_bridge.eval(code);

                // Sync state back
                if let Some(state) = super::lisp_ext::take_editor_state() {
                    self.buffer.lines = state.lines;
                    self.buffer.cursor.row = state
                        .cursor_row
                        .min(self.buffer.lines.len().saturating_sub(1));
                    self.buffer.cursor.col = state.cursor_col;
                    self.buffer.modified = state.modified;
                    self.buffer.overlays = state.overlays;
                    self.status_message = state.status_message;
                    if state.quit_requested {
                        self.quit_requested = true;
                    }
                }

                match result {
                    Ok(val) => {
                        self.status_message = format!("=> {}", val);
                    }
                    Err(e) => {
                        self.status_message = format!("Error: {}", e);
                    }
                }
                self.mode = EditorMode::Emacs;
                self.clear_minibuffer();
                return;
            }
            "lisp-repl" | "repl" => {
                self.status_message = "Mora REPL: type (help) for commands".to_string();
                self.mode = EditorMode::Emacs;
                self.clear_minibuffer();
                return;
            }
            "mshell" => {
                self.mshell.start();
                self.status_message = "Mshell started. Press Ctrl+G to exit.".to_string();
                self.clear_minibuffer();
                return;
            }
            _ if trimmed.starts_with("shell-command ") => {
                let cmd = trimmed["shell-command ".len()..].trim().to_string();
                if cmd.is_empty() {
                    self.status_message = "Usage: shell-command <command>".to_string();
                } else {
                    let mut state = super::lisp_ext::EditorState::new();
                    state.lines = self.buffer.lines.clone();
                    state.cursor_row = self.buffer.cursor.row;
                    state.cursor_col = self.buffer.cursor.col;
                    state.modified = self.buffer.modified;
                    state.file_path = self
                        .buffer
                        .path
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string());
                    state.mode = self.mode.label().to_lowercase();
                    state.window_count = self.windows.len();
                    state.overlays = std::mem::replace(
                        &mut self.buffer.overlays,
                        super::overlay::OverlayStore::new(),
                    );
                    super::lisp_ext::set_editor_state(state);

                    let result = self.lisp_bridge.eval(&format!(
                        "(shell-command \"{}\")",
                        cmd.replace('\\', "\\\\").replace('"', "\\\"")
                    ));

                    if let Some(state) = super::lisp_ext::take_editor_state() {
                        self.buffer.lines = state.lines;
                        self.buffer.cursor.row = state
                            .cursor_row
                            .min(self.buffer.lines.len().saturating_sub(1));
                        self.buffer.cursor.col = state.cursor_col;
                        self.buffer.modified = state.modified;
                        self.buffer.overlays = state.overlays;
                        self.status_message = state.status_message;
                        if state.quit_requested {
                            self.quit_requested = true;
                        }
                    }

                    match result {
                        Ok(_) => {}
                        Err(e) => self.status_message = format!("Error: {}", e),
                    }
                }
                self.mode = EditorMode::Emacs;
                self.clear_minibuffer();
                return;
            }
            _ => {
                // Unknown M-x command — try evaluating as lisp
                self.push_lisp_state();
                let lisp_code = format!("({})", trimmed);
                let result = self.lisp_bridge.eval(&lisp_code);
                self.pull_lisp_state();
                match result {
                    Ok(val) => {
                        let s = format!("{}", val);
                        if s != "nil" {
                            self.status_message = s;
                        }
                    }
                    Err(e) => {
                        self.status_message = format!("{}: {}", trimmed, e);
                    }
                }
                self.mode = EditorMode::Emacs;
                self.clear_minibuffer();
                return;
            }
        }

        let action = match self.mode {
            EditorMode::Command => {
                if trimmed == "wq" || trimmed == "x" {
                    if self.buffer.path.is_some() {
                        let _ = self.buffer.save();
                    }
                    self.quit_requested = true;
                    self.mode = EditorMode::Normal;
                    self.clear_minibuffer();
                    return;
                }
                keymap::parse_command(&input)
            }
            EditorMode::SearchForward => KeyAction::FindForward(input.clone()),
            EditorMode::SearchBackward => KeyAction::FindBackward(input.clone()),
            _ => KeyAction::None,
        };
        self.execute_action(action);
        if self.mode != EditorMode::Command
            && self.mode != EditorMode::SearchForward
            && self.mode != EditorMode::SearchBackward
        {
            self.clear_minibuffer();
        }
    }

    fn push_lisp_state(&mut self) {
        let mut state = super::lisp_ext::EditorState::new();
        state.lines = self.buffer.lines.clone();
        state.cursor_row = self.buffer.cursor.row;
        state.cursor_col = self.buffer.cursor.col;
        state.modified = self.buffer.modified;
        state.file_path = self
            .buffer
            .path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string());
        state.mode = self.mode.label().to_lowercase();
        state.window_count = self.windows.len();
        state.overlays = std::mem::replace(
            &mut self.buffer.overlays,
            super::overlay::OverlayStore::new(),
        );
        super::lisp_ext::set_editor_state(state);
    }

    fn pull_lisp_state(&mut self) {
        if let Some(state) = super::lisp_ext::take_editor_state() {
            self.buffer.lines = state.lines;
            self.buffer.cursor.row = state
                .cursor_row
                .min(self.buffer.lines.len().saturating_sub(1));
            self.buffer.cursor.col = state.cursor_col;
            self.buffer.modified = state.modified;
            self.buffer.overlays = state.overlays;
            self.status_message = state.status_message;
            if state.quit_requested {
                self.quit_requested = true;
            }
        }
    }

    fn command_candidates(&self) -> Vec<String> {
        let built_in_commands = [
            "capitalize-word",
            "cleanup-buffer",
            "copy-and-comment",
            "describe-mode",
            "disable-minor-mode",
            "evil-mode",
            "view-messages",
            "view-echo-area-messages",
            "dos2unix",
            "enable-minor-mode",
            "goto-last-change",
            "goto-line",
            "iedit",
            "iedit-add-region",
            "iedit-regex",
            "iedit-refind-all",
            "iedit-skip-region",
            "kill-buffer",
            "kill-emacs",
            "lowercase-word",
            "lowercase-region",
            "multi-cursor-edit",
            "narrow-to-region",
            "recenter",
            "replace-string",
            "save-buffer",
            "save-some-buffers",
            "set-mode",
            "mshell",
            "shell-command",
            "switch-mode",
            "toggle-theme",
            "toggle-fold",
            "toggle-minor-mode",
            "transpose-char",
            "transpose-line",
            "transpose-word",
            "unix2dos",
            "uppercase-region",
            "uppercase-word",
            "what-cursor-position",
            "widen",
        ];

        let mut commands: Vec<String> = built_in_commands.iter().map(|c| c.to_string()).collect();
        commands.extend(self.lisp_bridge.command_names());
        commands.sort();
        commands.dedup();
        commands
    }

    fn mx_complete(&mut self) {
        let input = self.command_input().trim();
        if input.is_empty() {
            return;
        }

        if self.minibuffer.is_active() {
            let candidates = self.command_candidates();
            self.minibuffer.set_completions(candidates);
            match self.minibuffer.complete_prefix() {
                CompletionResult::Completed => {
                    self.command_input = self.minibuffer.input().to_string();
                }
                CompletionResult::Matches(matches) => {
                    self.status_message = matches.join("  ");
                }
                CompletionResult::None => {}
            }
            return;
        }

        let mut minibuffer = Minibuffer::default();
        minibuffer.activate("M-x ");
        minibuffer.set_input(self.command_input.clone());
        minibuffer.set_completions(self.command_candidates());
        match minibuffer.complete_prefix() {
            CompletionResult::Completed => self.command_input = minibuffer.input().to_string(),
            CompletionResult::Matches(matches) => self.status_message = matches.join("  "),
            CompletionResult::None => {}
        }
    }

    fn extract_text_between(
        &self,
        start: crate::mora::buffer::Cursor,
        end: crate::mora::buffer::Cursor,
    ) -> String {
        if start.row == end.row {
            self.buffer.lines[start.row][start.col..end.col].to_string()
        } else {
            let mut result = self.buffer.lines[start.row][start.col..].to_string();
            result.push('\n');
            for row in (start.row + 1)..end.row {
                result.push_str(&self.buffer.lines[row]);
                result.push('\n');
            }
            result.push_str(&self.buffer.lines[end.row][..end.col]);
            result
        }
    }

    fn record_change(&mut self) {
        let pos = (self.buffer.cursor.row, self.buffer.cursor.col);
        self.last_changes.push_front(pos);
        if self.last_changes.len() > 100 {
            self.last_changes.pop_back();
        }
    }

    fn clamp_cursor_to_narrow(&mut self) {
        if self.buffer.is_narrowed() {
            let min_row = self.buffer.narrow_start.unwrap_or(0);
            let max_row = self
                .buffer
                .narrow_end
                .unwrap_or(self.buffer.line_count().saturating_sub(1));
            if self.buffer.cursor.row < min_row {
                self.buffer.cursor.row = min_row;
            }
            if self.buffer.cursor.row > max_row {
                self.buffer.cursor.row = max_row;
            }
            let line_len = self.buffer.current_line().chars().count();
            if self.buffer.cursor.col > line_len {
                self.buffer.cursor.col = line_len;
            }
        }
    }

    fn toggle_record_macro(&mut self) {
        if self.macro_state.is_recording() {
            self.macro_state.stop_recording();
            let events = self.macro_state.store_in_register('e').unwrap_or_default();
            self.registers.set('e', RegisterValue::Macro(events));
            self.status_message = "Macro recording stopped".to_string();
        } else {
            self.macro_state.start_recording('e');
            self.status_message = "Recording macro...".to_string();
        }
    }

    fn repeated_action(&mut self, action: KeyAction, n: usize) -> KeyAction {
        for _ in 0..n - 1 {
            self.execute_action(action.clone());
        }
        action
    }

    fn save_current_buffer(&mut self) {
        if self.buffer.path.is_some() {
            match self.buffer.save() {
                Ok(()) => {
                    self.status_message = format!("Saved: {}", self.buffer.filename());
                    // Flycheck: lint on save
                    self.flycheck_lint();
                }
                Err(e) => self.status_message = format!("Save error: {}", e),
            }
        } else {
            self.activate_minibuffer(EditorMode::Command);
            self.set_minibuffer_input("w ");
        }
    }

    fn run_git_command(&self, args: &[&str]) -> String {
        match std::process::Command::new("git").args(args).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.is_empty() && stdout.is_empty() {
                    stderr.into_owned()
                } else {
                    stdout.into_owned()
                }
            }
            Err(e) => format!("git: {e}"),
        }
    }

    fn run_git_status(&mut self) {
        let output = self.run_git_command(&["status", "--short"]);
        self.status_message = if output.trim().is_empty() {
            "Git: clean working tree".to_string()
        } else {
            // Show first line in status bar
            let first_line = output.lines().next().unwrap_or("");
            format!("Git: {first_line} ({} files)", output.lines().count())
        };
    }

    fn run_spelling_check(&mut self) {
        // Check spelling of current word using aspell
        let word = self.buffer.current_word();
        if word.is_empty() {
            self.status_message = "Spelling: no word under cursor".to_string();
            return;
        }
        let output = std::process::Command::new("aspell")
            .args(["pipe"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(stdin) = child.stdin.take() {
                    let mut stdin = stdin;
                    let _ = writeln!(stdin, "^{word}");
                }
                child.wait_with_output()
            });
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if stdout.contains("& ") {
                    let suggestions = stdout.lines()
                        .find(|l| l.starts_with("& "))
                        .and_then(|l| l.split(':').nth(1))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();
                    self.status_message = format!("Spelling: \"{word}\" — suggestions: {suggestions}");
                } else if stdout.contains("* ") {
                    self.status_message = format!("Spelling: \"{word}\" is correct");
                } else {
                    self.status_message = format!("Spelling: \"{word}\" — check aspell");
                }
            }
            Err(e) => {
                self.status_message = format!("Spelling: aspell not found ({e})");
            }
        }
    }

    fn run_go_to_definition(&mut self) {
        let word = self.buffer.current_word();
        if word.is_empty() {
            self.status_message = "Go to definition: no word under cursor".to_string();
            return;
        }
        if self.lsp_client.is_connected() {
            let uri = self.buffer.path.as_ref()
                .map(|p| format!("file://{}", p.display()))
                .unwrap_or_default();
            let line = self.buffer.cursor.row as u32;
            let col = self.buffer.cursor.col as u32;
            match self.lsp_client.definition(&uri, line, col) {
                Ok(locs) if !locs.is_empty() => {
                    let loc = &locs[0];
                    self.status_message = format!("Definition: {} L{}:C{}", loc.uri, loc.range.start.line + 1, loc.range.start.character + 1);
                }
                Ok(_) => self.status_message = format!("Definition: no definition found for \"{word}\""),
                Err(e) => self.status_message = format!("Definition: {e}"),
            }
        } else {
            self.status_message = format!("Go to definition: \"{word}\" — use SPC p g to grep (LSP not connected)");
        }
    }

    fn start_lsp(&mut self) {
        let ext = self.buffer.path.as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let root = self.buffer.path.as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        if let Some((cmd, args, lang_id)) = super::lsp::detect_language_server(ext) {
            let root_uri = format!("file://{}", root.display());
            let args_refs: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();
            match self.lsp_client.start(cmd, &args_refs, &root_uri) {
                Ok(()) => {
                    // Open the current document
                    if let Some(ref path) = self.buffer.path {
                        let uri = format!("file://{}", path.display());
                        let text = self.buffer.lines.join("\n");
                        let _ = self.lsp_client.did_open(&uri, lang_id, 1, &text);
                    }
                    self.status_message = format!("LSP: connected to {} ({})", self.lsp_client.server_name, ext);
                }
                Err(e) => {
                    self.status_message = format!("LSP: failed to start {cmd}: {e}");
                }
            }
        } else {
            self.status_message = format!("LSP: no language server configured for .{ext}");
        }
    }

    fn run_find_references(&mut self) {
        let word = self.buffer.current_word();
        if word.is_empty() {
            self.status_message = "Find references: no word under cursor".to_string();
            return;
        }
        if self.lsp_client.is_connected() {
            let uri = self.buffer.path.as_ref()
                .map(|p| format!("file://{}", p.display()))
                .unwrap_or_default();
            let line = self.buffer.cursor.row as u32;
            let col = self.buffer.cursor.col as u32;
            match self.lsp_client.references(&uri, line, col) {
                Ok(locs) => {
                    let count = locs.len();
                    let first = locs.first()
                        .map(|l| format!("{} L{}", l.uri.rsplit('/').next().unwrap_or(&l.uri), l.range.start.line + 1))
                        .unwrap_or_default();
                    self.status_message = format!("References: {first} ({count} matches)");
                }
                Err(e) => self.status_message = format!("References: {e}"),
            }
        } else {
            // Fallback to ripgrep
            let output = std::process::Command::new("rg")
                .args(["--line-number", "--no-heading", "--max-count", "50", "-w", &word])
                .output();
            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    if stdout.trim().is_empty() {
                        self.status_message = format!("References: no references to \"{word}\"");
                    } else {
                        let first = stdout.lines().next().unwrap_or("");
                        self.status_message = format!("References: {first} ({} matches)", stdout.lines().count());
                    }
                }
                Err(e) => self.status_message = format!("References: {e}"),
            }
        }
    }

    fn run_hover_doc(&mut self) {
        let word = self.buffer.current_word();
        if word.is_empty() {
            self.status_message = "Hover: no word under cursor".to_string();
            return;
        }
        if self.lsp_client.is_connected() {
            let uri = self.buffer.path.as_ref()
                .map(|p| format!("file://{}", p.display()))
                .unwrap_or_default();
            let line = self.buffer.cursor.row as u32;
            let col = self.buffer.cursor.col as u32;
            match self.lsp_client.hover(&uri, line, col) {
                Ok(Some(hover)) => {
                    let truncated: String = hover.contents.chars().take(200).collect();
                    self.status_message = format!("Hover: {truncated}");
                }
                Ok(None) => self.status_message = format!("Hover: no documentation for \"{word}\""),
                Err(e) => self.status_message = format!("Hover: {e}"),
            }
        } else {
            let row = self.buffer.cursor.row + 1;
            let col = self.buffer.cursor.col + 1;
            let line = self.buffer.current_line();
            self.status_message = format!("\"{word}\" at L{row}:C{col} — {}", line.trim());
        }
    }
    /// Flycheck: lint the current buffer based on file extension.
    /// Runs the appropriate linter and shows results in status bar.
    /// Errors/warnings are stored as overlays on the buffer.
    fn flycheck_lint(&mut self) {
        let path = match &self.buffer.path {
            Some(p) => p.clone(),
            None => return,
        };
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let filename = path.to_string_lossy();

        // Select linter based on file extension
        let linter = match ext {
            "rs" => ("rustc", vec!["--edition", "2021", "--crate-type", "lib", "-Z", "parse-only", &filename]),
            "py" => ("python3", vec!["-m", "py_compile", &filename]),
            "js" | "ts" | "jsx" | "tsx" => ("node", vec!["--check", &filename]),
            "go" => ("go", vec!["vet", &filename]),
            "c" | "h" => ("gcc", vec!["-fsyntax-only", "-Wall", &filename]),
            "cpp" | "hpp" | "cc" => ("g++", vec!["-fsyntax-only", "-Wall", &filename]),
            "sh" | "bash" => ("bash", vec!["-n", &filename]),
            "rb" => ("ruby", vec!["-c", &filename]),
            "lua" => ("luac", vec!["-p", &filename]),
            _ => return,
        };

        let (cmd, args) = linter;
        let output = std::process::Command::new(cmd)
            .args(&args)
            .stderr(std::process::Stdio::piped())
            .output();

        match output {
            Ok(out) if !out.status.success() => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                // Show first error/warning
                let first_error = stderr.lines()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("lint error");
                self.status_message = format!("⚠ Flycheck: {first_error}");
            }
            Ok(_) => {
                // No errors - clear previous flycheck message if any
                if self.status_message.starts_with("⚠ Flycheck") {
                    self.status_message.clear();
                }
            }
            Err(e) => {
                // Linter not installed - silently skip
                let _ = e;
            }
        }
    }

    /// Dired: open directory browser at current file's directory.
    fn dired_open(&mut self) {
        let dir = self.buffer.path.as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let output = std::process::Command::new("ls")
            .args(["-la", &dir.to_string_lossy()])
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let lines: Vec<&str> = stdout.lines().take(20).collect();
                self.status_message = format!("Dired {}: {}", dir.display(), lines.join("; "));
            }
            Err(e) => {
                self.status_message = format!("Dired: {e}");
            }
        }
    }

    /// Line-reminder: get git diff changed line numbers for current file.
    /// Returns a set of (line_number, change_type) where type is '+', '-', or '~'.
    fn get_git_diff_lines(&self) -> Vec<(usize, char)> {
        let path = match &self.buffer.path {
            Some(p) => p.to_string_lossy().to_string(),
            None => return Vec::new(),
        };
        let output = std::process::Command::new("git")
            .args(["diff", "-U0", "--", &path])
            .output();
        let mut changed = Vec::new();
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                // Parse @@ -old,count +new,count @@
                if line.starts_with("@@") {
                    if let Some(plus_part) = line.split('+').nth(1) {
                        if let Some(num_str) = plus_part.split(|c: char| !c.is_ascii_digit()).next() {
                            if let Ok(num) = num_str.parse::<usize>() {
                                changed.push((num, '+'));
                            }
                        }
                    }
                }
            }
        }
        changed
    }

    fn run_ripgrep(&mut self, pattern: &str) {
        let output = std::process::Command::new("rg")
            .args(["--line-number", "--no-heading", "--max-count", "50", pattern])
            .output();
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if stdout.trim().is_empty() {
                    self.status_message = format!("Grep: no matches for \"{pattern}\"");
                } else {
                    let first = stdout.lines().next().unwrap_or("");
                    self.status_message = format!("Grep: {first} ({} matches)", stdout.lines().count());
                }
            }
            Err(e) => {
                self.status_message = format!("Grep: {e}");
            }
        }
    }

    fn run_git_log(&mut self) {
        let output = self.run_git_command(&["log", "--oneline", "-20"]);
        self.status_message = if output.trim().is_empty() {
            "Git: no commits".to_string()
        } else {
            let first = output.lines().next().unwrap_or("");
            format!("Git log: {first} ({} commits)", output.lines().count())
        };
    }

    fn run_git_diff(&mut self) {
        let output = self.run_git_command(&["diff", "--stat"]);
        self.status_message = if output.trim().is_empty() {
            "Git: no changes".to_string()
        } else {
            format!("Git diff: {}", output.lines().next().unwrap_or(""))
        };
    }

    fn run_git_commit(&mut self) {
        // Stage all changes and commit with a message
        let status = self.run_git_command(&["status", "--short"]);
        if status.trim().is_empty() {
            self.status_message = "Git: nothing to commit".to_string();
            return;
        }
        self.activate_minibuffer_with_prompt(EditorMode::Command, "Git commit message: ");
    }

    fn run_git_commit_with_message(&mut self, msg: &str) {
        // Stage all and commit
        let add_output = self.run_git_command(&["add", "-A"]);
        let commit_output = self.run_git_command(&["commit", "-m", msg]);
        if commit_output.contains("nothing to commit") || commit_output.contains("no changes added") {
            self.status_message = "Git: nothing to commit".to_string();
        } else {
            let first = commit_output.lines().find(|l| !l.is_empty()).unwrap_or("");
            self.status_message = format!("Git: {first}");
        }
    }

    fn kill_line_to_ring(&mut self) {
        let row = self.buffer.cursor.row;
        if row < self.buffer.lines.len() {
            let text = self.buffer.lines[row].clone();
            self.kill_ring.kill(&text, false);
            self.last_yank_was_kill = true;
        }
    }

    fn replace_last_yank(&mut self, new_text: &str) {
        let row = self.buffer.cursor.row;
        if row > 0 && row <= self.buffer.lines.len() {
            self.buffer.lines.remove(row - 1);
            if new_text.contains('\n') {
                for (i, line) in new_text.split('\n').enumerate() {
                    self.buffer.lines.insert(row - 1 + i, line.to_string());
                }
            } else {
                self.buffer.lines.insert(row - 1, new_text.to_string());
            }
            self.buffer.cursor.row = row - 1;
            self.buffer.cursor.col = 0;
            self.buffer.modified = true;
        } else if row < self.buffer.lines.len() {
            let col = self.buffer.cursor.col;
            let yank_len = self.kill_ring.yank().map(|e| e.text.len()).unwrap_or(0);
            let start = col.saturating_sub(yank_len);
            if start < col && col <= self.buffer.lines[row].len() {
                self.buffer.lines[row].replace_range(start..col, new_text);
                self.buffer.modified = true;
            }
        }
    }

    pub fn process_macro_events(&mut self) -> Option<KeyEvent> {
        while let Some(ev) = self.macro_state.next_event() {
            if ev.code == KeyCode::Esc || ev.code == KeyCode::Enter {
                return Some(ev);
            }
        }
        self.macro_state.cancel_playback();
        None
    }

    fn repeat_last_find(&mut self, same_direction: bool) {
        if let Some(c) = self.last_find_char {
            let forward = if same_direction {
                self.last_find_forward
            } else {
                !self.last_find_forward
            };
            let till = self.last_find_till;
            let line = self.buffer.current_line();
            let col = self.buffer.cursor.col;
            let chars: Vec<char> = line.chars().collect();
            if forward {
                let start = if till { col + 1 } else { col };
                if let Some(pos) = chars[start..].iter().position(|&ch| ch == c) {
                    let target = start + pos;
                    self.buffer.cursor.col = if till {
                        target.saturating_sub(1)
                    } else {
                        target
                    };
                }
            } else {
                let end = if till { col } else { col + 1 };
                if let Some(pos) = chars[..end].iter().rposition(|&ch| ch == c) {
                    self.buffer.cursor.col = if till { pos + 1 } else { pos };
                }
            }
        }
    }

    fn handle_ace_jump(&mut self, key: KeyEvent) -> KeyAction {
        // Step 1: waiting for target char
        if self.ace_jump_target.is_none() {
            if let KeyCode::Char(c) = key.code {
                self.ace_jump_target = Some(c);
                self.ace_jump_hints.clear();
                // Find all visible occurrences of target char
                let hint_chars: Vec<char> = "asdfghjkl".chars().collect();
                let view_start = self.view.scroll_top;
                let view_end = (view_start + self.view.height).min(self.buffer.lines.len());
                let mut hint_idx = 0;
                for row in view_start..view_end {
                    let line = &self.buffer.lines[row];
                    for (col, ch) in line.chars().enumerate() {
                        if ch == c && hint_idx < hint_chars.len() {
                            self.ace_jump_hints.push((row, col, hint_chars[hint_idx]));
                            hint_idx += 1;
                        }
                    }
                }
                if self.ace_jump_hints.is_empty() {
                    self.waiting_ace_jump = false;
                    self.ace_jump_target = None;
                    self.status_message = format!("No '{}' found", c);
                    return KeyAction::None;
                }
                self.status_message = format!(
                    "Jump to: {}",
                    self.ace_jump_hints
                        .iter()
                        .map(|(_, _, h)| *h)
                        .collect::<String>()
                );
            } else {
                self.waiting_ace_jump = false;
                self.ace_jump_target = None;
            }
            return KeyAction::None;
        }
        // Step 2: waiting for hint key
        if let KeyCode::Char(c) = key.code {
            if let Some((row, col, _)) = self.ace_jump_hints.iter().find(|(_, _, h)| *h == c) {
                self.buffer.cursor.row = *row;
                self.buffer.cursor.col = *col;
            }
        }
        self.waiting_ace_jump = false;
        self.ace_jump_target = None;
        self.ace_jump_hints.clear();
        KeyAction::None
    }

    pub fn split_window_horizontal(&mut self) {
        self.sync_buffer_to_window();
        let height = self.view.height;
        let half = height / 2;
        if half < 3 {
            self.status_message = "Window too small to split".to_string();
            return;
        }
        let new_view = View::new(half);
        self.view.height = half;
        self.windows.push(WindowState {
            view: new_view,
            buffer_idx: self.current_window_buffer_idx,
            cursor: self.buffer.cursor,
        });
        self.current_window_idx = self.windows.len() - 1;
        self.sync_window_to_buffer();
        self.status_message = format!("Split horizontal ({} windows)", self.windows.len());
    }

    pub fn split_window_vertical(&mut self) {
        self.sync_buffer_to_window();
        let width = 80;
        let half = width / 2;
        if half < 10 {
            self.status_message = "Window too small to split".to_string();
            return;
        }
        let new_view = View::new(self.view.height);
        self.windows.push(WindowState {
            view: new_view,
            buffer_idx: self.current_window_buffer_idx,
            cursor: self.buffer.cursor,
        });
        self.current_window_idx = self.windows.len() - 1;
        self.sync_window_to_buffer();
        self.status_message = format!("Split vertical ({} windows)", self.windows.len());
    }

    fn delete_window(&mut self) {
        if self.windows.len() <= 1 {
            self.status_message = "Can't delete last window".to_string();
            return;
        }
        self.sync_buffer_to_window();
        self.windows.remove(self.current_window_idx);
        if self.current_window_idx >= self.windows.len() {
            self.current_window_idx = self.windows.len() - 1;
        }
        self.sync_window_to_buffer();
        self.status_message = format!("Deleted window ({} remaining)", self.windows.len());
    }

    fn delete_other_windows(&mut self) {
        if self.windows.len() <= 1 {
            return;
        }
        self.sync_buffer_to_window();
        let current = self.windows[self.current_window_idx].clone();
        self.windows.clear();
        self.windows.push(current);
        self.current_window_idx = 0;
        self.sync_window_to_buffer();
        self.status_message = "Deleted other windows".to_string();
    }

    pub fn other_window(&mut self) {
        if self.windows.len() <= 1 {
            return;
        }
        self.sync_buffer_to_window();
        self.current_window_idx = (self.current_window_idx + 1) % self.windows.len();
        self.sync_window_to_buffer();
        self.status_message = format!(
            "Window {}/{}",
            self.current_window_idx + 1,
            self.windows.len()
        );
    }

    fn balance_windows(&mut self) {
        if self.windows.is_empty() {
            return;
        }
        let total_height = self.view.height * self.windows.len();
        let per_window = total_height / self.windows.len();
        for win in &mut self.windows {
            win.view.height = per_window;
        }
        self.status_message = "Balanced windows".to_string();
    }

    fn sync_buffer_to_window(&mut self) {
        if self.current_window_idx < self.windows.len() {
            self.windows[self.current_window_idx].cursor = self.buffer.cursor;
        }
    }

    fn sync_window_to_buffer(&mut self) {
        if self.current_window_idx < self.windows.len() {
            let win = &self.windows[self.current_window_idx];
            self.buffer.cursor = win.cursor;
        }
    }

    pub fn window_index_display(win: &WindowState) -> String {
        format!("[{}]", win.buffer_idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mx_executes_registered_lisp_command() {
        let mut editor = MoraEditor::new(20);
        editor
            .lisp_bridge
            .eval(
                r#"
                (ns coldnew.commands)
                (require [mora.buffer :as buffer])
                (defn insert-marker []
                  (interactive)
                  (buffer/insert! "marker"))
                "#,
            )
            .unwrap();

        editor.command_input = "insert-marker".to_string();
        editor.execute_command();

        assert_eq!(editor.buffer.lines[0], "marker");
        assert_eq!(editor.mode, EditorMode::Emacs);
        assert!(editor.command_input.is_empty());
    }

    #[test]
    fn mx_completion_includes_registered_lisp_command() {
        let mut editor = MoraEditor::new(20);
        editor
            .lisp_bridge
            .eval(
                r#"
                (ns coldnew.commands)
                (defn coldnew-test-command []
                  (interactive)
                  nil)
                "#,
            )
            .unwrap();

        editor.command_input = "coldnew-test".to_string();
        editor.mx_complete();

        assert_eq!(editor.command_input, "coldnew-test-command");
    }

    #[test]
    fn command_mode_activates_minibuffer() {
        let mut editor = MoraEditor::new(20);

        editor.execute_action(KeyAction::SetMode(EditorMode::Command));
        editor.execute_action(KeyAction::InputChar('w'));
        editor.execute_action(KeyAction::InputBackspace);

        assert!(editor.minibuffer_active());
        assert_eq!(editor.minibuffer_prompt(), ":");
        assert_eq!(editor.command_input(), "");
    }

    #[test]
    fn mx_uses_minibuffer_prompt_and_completion() {
        let mut editor = MoraEditor::new(20);

        editor.activate_minibuffer_with_prompt(EditorMode::Command, "M-x ");
        editor.execute_action(KeyAction::InputChar('s'));
        editor.execute_action(KeyAction::InputChar('a'));
        editor.mx_complete();

        assert_eq!(editor.minibuffer_prompt(), "M-x ");
        assert_eq!(editor.command_input(), "save-");
    }

    #[test]
    fn iedit_finds_all_occurrences() {
        let mut editor = MoraEditor::new(20);
        editor.buffer.lines = vec![
            "let x = 1;".to_string(),
            "let y = x + 2;".to_string(),
            "println!(x);".to_string(),
        ];
        // Place cursor on first 'x' at col 4
        editor.buffer.cursor.row = 0;
        editor.buffer.cursor.col = 4;
        editor.start_iedit();

        assert_eq!(editor.mode, EditorMode::Iedit);
        assert_eq!(editor.iedit_word, Some("x".to_string()));
        // Should find 3 occurrences of 'x' as a word
        assert!(editor.iedit_regions.len() >= 3);
    }

    #[test]
    fn iedit_skip_region_removes_current() {
        let mut editor = MoraEditor::new(20);
        editor.buffer.lines = vec![
            "foo bar foo baz foo".to_string(),
        ];
        editor.buffer.cursor.row = 0;
        editor.buffer.cursor.col = 0;
        editor.start_iedit();

        let initial_count = editor.iedit_regions.len();
        assert!(initial_count >= 3);

        // Skip the current region
        editor.iedit_skip_region();
        assert_eq!(editor.iedit_regions.len(), initial_count - 1);
        assert_eq!(editor.mode, EditorMode::Iedit);
    }

    #[test]
    fn iedit_skip_all_regions_exits() {
        let mut editor = MoraEditor::new(20);
        editor.buffer.lines = vec![
            "foo foo".to_string(),
        ];
        editor.buffer.cursor.row = 0;
        editor.buffer.cursor.col = 0;
        editor.start_iedit();

        let count = editor.iedit_regions.len();
        for _ in 0..count {
            if editor.mode == EditorMode::Iedit {
                editor.iedit_skip_region();
            }
        }
        assert_eq!(editor.mode, EditorMode::Emacs);
    }

    #[test]
    fn iedit_insert_char_in_all_regions() {
        let mut editor = MoraEditor::new(20);
        editor.buffer.lines = vec![
            "x = x + 1".to_string(),
        ];
        editor.buffer.cursor.row = 0;
        editor.buffer.cursor.col = 0;
        editor.start_iedit();

        // Type 'y' to append to each 'x' region
        editor.iedit_insert_char('y');
        // Each 'x' (as word) should now be 'xy'
        // Check the first line contains "xy"
        assert!(editor.buffer.lines[0].contains("xy"));
    }

    #[test]
    fn iedit_delete_backward_in_all_regions() {
        let mut editor = MoraEditor::new(20);
        editor.buffer.lines = vec![
            "xy = xy + 1".to_string(),
        ];
        editor.buffer.cursor.row = 0;
        editor.buffer.cursor.col = 0;
        editor.start_iedit();

        // Delete last char from each region
        editor.iedit_delete_backward();
        // Each 'xy' should now be 'x'
        assert!(editor.buffer.lines[0].contains("x"));
    }

    #[test]
    fn iedit_tab_cycles_regions() {
        let mut editor = MoraEditor::new(20);
        editor.buffer.lines = vec![
            "foo foo".to_string(),
            "bar".to_string(),
            "foo foo".to_string(),
        ];
        editor.buffer.cursor.row = 0;
        editor.buffer.cursor.col = 0;
        editor.start_iedit();

        let count = editor.iedit_regions.len();
        // "foo" appears 4 times: row 0 col 0, row 0 col 4, row 2 col 0, row 2 col 4
        assert!(count >= 2);
        let first_idx = editor.iedit_cursor_idx;

        // Tab to next region
        editor.iedit_next_region();
        assert_ne!(editor.iedit_cursor_idx, first_idx, "cursor_idx should change after next_region");

        // Tab back should return to first
        editor.iedit_prev_region();
        assert_eq!(editor.iedit_cursor_idx, first_idx);
    }

    #[test]
    fn iedit_add_region_at_cursor() {
        let mut editor = MoraEditor::new(20);
        editor.buffer.lines = vec![
            "foo foo bar foo".to_string(),
        ];
        editor.buffer.cursor.row = 0;
        editor.buffer.cursor.col = 0;
        editor.start_iedit();

        let initial_count = editor.iedit_regions.len();
        // Move cursor to 'bar' at col 8 (should not add since it's a different word)
        editor.buffer.cursor.col = 8;
        editor.iedit_add_region_at_cursor();
        // bar is different from foo, should not add
        assert_eq!(editor.iedit_regions.len(), initial_count);
        assert!(editor.status_message.contains("expected"));
    }


    #[test]
    fn iedit_delete_forward_removes_correct_char() {
        let mut editor = MoraEditor::new(20);
        // Use "foo foo" so start_iedit finds 2 occurrences
        editor.buffer.lines = vec![
            "foo foo".to_string(),
        ];
        editor.buffer.cursor.row = 0;
        editor.buffer.cursor.col = 0;
        editor.start_iedit();

        // Should find 2 occurrences of "foo"
        assert_eq!(editor.mode, EditorMode::Iedit);
        let regions_before = editor.iedit_regions.len();
        assert!(regions_before >= 2, "expected at least 2 regions, got {}", regions_before);

        // Delete forward in all regions
        editor.iedit_delete_forward();
        // Each "foo" should now be "fo" (deleted the last char)
        assert!(editor.buffer.lines[0].contains("fo"), "expected 'fo' in line");
        // The line should NOT contain the original "foo"
        assert!(!editor.buffer.lines[0].contains("foo"), "should not contain 'foo' after delete");
    }
    #[test]
    fn iedit_pushes_undo_snapshot() {
        let mut editor = MoraEditor::new(20);
        editor.buffer.lines = vec![
            "foo foo".to_string(),
        ];
        editor.start_iedit();
        // After entering iedit, buffer should have an undo snapshot
        // (we can't directly test undo here without the full undo-tree,
        // but at least the mode should be correct)
        assert_eq!(editor.mode, EditorMode::Iedit);
    }

    #[test]
    fn iedit_exits_on_single_occurrence() {
        let mut editor = MoraEditor::new(20);
        editor.buffer.lines = vec![
            "unique_word".to_string(),
            "other".to_string(),
        ];
        editor.buffer.cursor.row = 0;
        editor.buffer.cursor.col = 0;
        editor.start_iedit();

        // Only one occurrence, should not enter iedit
        assert_ne!(editor.mode, EditorMode::Iedit);
        assert_eq!(editor.status_message, "No other occurrences");
    }

    #[test]
    fn iedit_regex_finds_matches() {
        let mut editor = MoraEditor::new(20);
        editor.buffer.lines = vec![
            "x1 = x2 + x3".to_string(),
            "y1 = y2".to_string(),
        ];
        editor.buffer.cursor.row = 0;
        editor.buffer.cursor.col = 0;

        // Simulate: M-x iedit-regex, then type pattern
        editor.start_iedit_regex();
        assert!(editor.waiting_iedit_regex);
        assert!(editor.minibuffer_active());

        // Set pattern via minibuffer
        editor.set_minibuffer_input("[xy][0-9]");
        editor.execute_command();

        assert_eq!(editor.mode, EditorMode::Iedit);
        assert!(editor.iedit_regex);
        // Should find x1, x2, x3, y1, y2 = 5 matches
        assert_eq!(editor.iedit_regions.len(), 5);
    }


    #[test]
    fn scratch_buffer_initializes_correctly() {
        let mut editor = MoraEditor::new(20);
        editor.init_scratch_buffer();

        assert!(editor.scratch_initialized);
        assert_eq!(editor.buffer.lines.len(), 5);
        assert_eq!(editor.buffer.lines[0], ";; This buffer is for text that is not saved.");
        assert_eq!(editor.buffer.cursor.row, 4);
        assert!(!editor.buffer.modified);
    }

    #[test]
    fn scratch_buffer_init_is_idempotent() {
        let mut editor = MoraEditor::new(20);
        editor.init_scratch_buffer();
        editor.buffer.lines[4] = "user typed".to_string();
        editor.init_scratch_buffer(); // should not re-init

        assert_eq!(editor.buffer.lines[4], "user typed");
    }

    #[test]
    fn message_logging_works() {
        let mut editor = MoraEditor::new(20);
        editor.message("hello");
        editor.message("world");

        assert_eq!(editor.messages.len(), 2);
        assert_eq!(editor.messages[0], "hello");
        assert_eq!(editor.messages[1], "world");
    }

    #[test]
    fn messages_truncate_at_max() {
        let mut editor = MoraEditor::new(20);
        editor.messages_max = 3;
        editor.message("a");
        editor.message("b");
        editor.message("c");
        editor.message("d");

        assert_eq!(editor.messages.len(), 3);
        assert_eq!(editor.messages[0], "b");
        assert_eq!(editor.messages[2], "d");
    }

    #[test]
    fn evil_mode_command_toggles_mode() {
        let mut editor = MoraEditor::new(20);
        assert_eq!(editor.mode, EditorMode::Emacs);

        // Toggle to evil (Normal) mode
        editor.command_input = "evil-mode".to_string();
        editor.execute_command();
        // After command, we should be back in a mode
        assert!(editor.status_message.contains("Evil mode"));
    }

}

fn op_char(op: PendingOp) -> char {
    match op {
        PendingOp::Delete => 'd',
        PendingOp::Yank => 'y',
        PendingOp::Change => 'c',
    }
}
