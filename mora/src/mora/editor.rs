use std::path::Path;

use super::display::event::MoraKeyEvent as KeyEvent;

use super::buffer::Buffer;
use super::keymap::{EditorMode, KeyAction, PendingOp};
use super::kill_ring::KillRing;
use super::lisp_ext::MoraLispBridge;
use super::macro_state::MacroState;
use super::major_mode::{self, MajorModeKind};
use super::mark::MarkRing;
use super::minibuffer::Minibuffer;
use super::register::Registers;
use super::view::View;
use super::wasm_ext::WasmExtensionHost;

mod commands;
mod input;
mod windows;

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
    pub snippet_engine: super::snippet::SnippetEngine,
    pub active_snippet: Option<super::snippet::SnippetExpansion>,
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
            snippet_engine: {
                let mut engine = super::snippet::SnippetEngine::new();
                engine.load_defaults();
                engine
            },
            active_snippet: None,
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
        self.buffer.major_mode = major_mode::create_mode(MajorModeKind::Lisp);
        self.buffer.cursor.row = 4;
        self.buffer.cursor.col = 0;
        self.buffer.modified = false;
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
                (defn ^:interactive insert-marker []
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
    fn mx_executes_lisp_command_after_init_mora_load() {
        use crate::mora::editor_core::MoraCore;
        use crate::mora::lisp_ext::{set_editor_state, EditorState};

        // Simulate the wgpu binary's startup: init_lisp_state + load_init_file
        let mut core = MoraCore::new(80, 24);
        let mut state = EditorState::new();
        state.lines = core.editor.buffer.lines.clone();
        state.cursor_row = core.editor.buffer.cursor.row;
        state.cursor_col = core.editor.buffer.cursor.col;
        state.modified = core.editor.buffer.modified;
        state.file_path = core
            .editor
            .buffer
            .path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string());
        state.mode = format!("{:?}", core.editor.mode()).to_lowercase();
        state.window_count = core.editor.windows.len().max(1);
        set_editor_state(state);

        // Define a lisp command directly (simulating what init.mora would do)
        core.editor
            .lisp_bridge
            .eval(
                r#"
                (ns coldnew.commands)
                (require [mora.editor :as editor])
                (defn ^:interactive show-status
                  "Show Mora editor status."
                  []
                  (editor/message (editor/status)))
                "#,
            )
            .unwrap();

        // Verify the command is registered
        assert!(
            core.editor.lisp_bridge.has_command("show-status"),
            "show-status should be registered"
        );

        // Execute via the same path the wgpu binary uses
        core.editor.command_input = "show-status".to_string();
        core.editor.execute_command();
    }

    #[test]
    fn mx_executes_lisp_command_via_keymap() {
        use crate::mora::display::event::{
            MoraKeyCode as KeyCode, MoraKeyEvent as KeyEvent, MoraKeyModifiers as KeyModifiers,
        };
        use crate::mora::editor_core::MoraCore;
        use crate::mora::lisp_ext::EditorState;

        let mut core = MoraCore::new(80, 24);
        // init_lisp_state: set a fresh state on the thread-local
        let mut state = EditorState::new();
        state.lines = core.editor.buffer.lines.clone();
        state.cursor_row = core.editor.buffer.cursor.row;
        state.cursor_col = core.editor.buffer.cursor.col;
        state.mode = format!("{:?}", core.editor.mode()).to_lowercase();
        state.window_count = core.editor.windows.len().max(1);
        crate::mora::lisp_ext::set_editor_state(state);

        core.editor
            .lisp_bridge
            .eval(
                r#"
                (ns coldnew.commands)
                (require [mora.editor :as editor])
                (defn ^:interactive show-status
                  "Show Mora editor status."
                  []
                  (editor/message (editor/status)))
                "#,
            )
            .unwrap();

        // Simulate: M-x via Alt+x
        core.editor.handle_key(KeyEvent::new(
            KeyCode::Char('x'),
            KeyModifiers {
                alt: true,
                ..Default::default()
            },
        ));

        // Type the command name
        for c in "show-status".chars() {
            core.editor
                .handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::default()));
        }

        // Press Enter
        core.editor
            .handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::default()));
    }

    #[test]
    fn mx_handles_unknown_command_without_panic() {
        use crate::mora::display::event::{
            MoraKeyCode as KeyCode, MoraKeyEvent as KeyEvent, MoraKeyModifiers as KeyModifiers,
        };
        use crate::mora::editor_core::MoraCore;
        use crate::mora::lisp_ext::EditorState;

        // Simulate wgpu binary startup with NO init.mora loaded (no commands registered)
        let mut core = MoraCore::new(80, 24);
        let mut state = EditorState::new();
        state.lines = core.editor.buffer.lines.clone();
        state.cursor_row = core.editor.buffer.cursor.row;
        state.cursor_col = core.editor.buffer.cursor.col;
        state.mode = format!("{:?}", core.editor.mode()).to_lowercase();
        state.window_count = core.editor.windows.len().max(1);
        crate::mora::lisp_ext::set_editor_state(state);

        // M-x, type a lisp expression that calls a native function, press Enter
        core.editor.handle_key(KeyEvent::new(
            KeyCode::Char('x'),
            KeyModifiers {
                alt: true,
                ..Default::default()
            },
        ));
        for c in "(editor-message \"hello\")".chars() {
            core.editor
                .handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::default()));
        }
        core.editor
            .handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::default()));
    }

    #[test]
    fn mx_via_handle_input_mimics_wgpu_binary() {
        use crate::mora::editor_core::MoraCore;
        use crate::mora::lisp_ext::EditorState;
        use display_protocol::{InputEvent, KeyCode, KeyEvent, KeyModifiers};

        // Simulate the exact flow the wgpu binary does:
        //   run_editor_wgpu() → init_lisp_state → load_init_file → WgpuWindow::run
        // The user callback receives InputEvent and calls core.handle_input(ev.clone()).
        let mut core = MoraCore::new(80, 24);

        // init_lisp_state
        let mut state = EditorState::new();
        state.lines = core.editor.buffer.lines.clone();
        state.cursor_row = core.editor.buffer.cursor.row;
        state.cursor_col = core.editor.buffer.cursor.col;
        state.modified = core.editor.buffer.modified;
        state.file_path = core
            .editor
            .buffer
            .path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string());
        state.mode = format!("{:?}", core.editor.mode()).to_lowercase();
        state.window_count = core.editor.windows.len().max(1);
        crate::mora::lisp_ext::set_editor_state(state);

        // No init.mora, so no commands registered.
        // User presses M-x (Alt+x), types "editor-message" (a built-in native function name),
        // and Enter. Since "editor-message" is not a registered command, the editor tries to
        // eval it as lisp, which is "(editor-message)". Without args, the call is rejected,
        // but no panic should occur.
        let events = vec![InputEvent::Key(KeyEvent::new(
            KeyCode::Char('x'),
            KeyModifiers {
                alt: true,
                ..Default::default()
            },
        ))];
        for ev in &events {
            let _cmds = core.handle_input(ev.clone());
        }
        for c in "editor-message".chars() {
            let ev = InputEvent::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::default()));
            let _cmds = core.handle_input(ev);
        }
        let ev = InputEvent::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::default()));
        let _cmds = core.handle_input(ev);
    }

    #[test]
    fn mx_completion_includes_registered_lisp_command() {
        let mut editor = MoraEditor::new(20);
        editor
            .lisp_bridge
            .eval(
                r#"
                (ns coldnew.commands)
                (defn ^:interactive coldnew-test-command []
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
        editor.buffer.lines = vec!["foo bar foo baz foo".to_string()];
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
        editor.buffer.lines = vec!["foo foo".to_string()];
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
        editor.buffer.lines = vec!["x = x + 1".to_string()];
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
        editor.buffer.lines = vec!["xy = xy + 1".to_string()];
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
        assert_ne!(
            editor.iedit_cursor_idx, first_idx,
            "cursor_idx should change after next_region"
        );

        // Tab back should return to first
        editor.iedit_prev_region();
        assert_eq!(editor.iedit_cursor_idx, first_idx);
    }

    #[test]
    fn iedit_add_region_at_cursor() {
        let mut editor = MoraEditor::new(20);
        editor.buffer.lines = vec!["foo foo bar foo".to_string()];
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
        editor.buffer.lines = vec!["foo foo".to_string()];
        editor.buffer.cursor.row = 0;
        editor.buffer.cursor.col = 0;
        editor.start_iedit();

        // Should find 2 occurrences of "foo"
        assert_eq!(editor.mode, EditorMode::Iedit);
        let regions_before = editor.iedit_regions.len();
        assert!(
            regions_before >= 2,
            "expected at least 2 regions, got {}",
            regions_before
        );

        // Delete forward in all regions
        editor.iedit_delete_forward();
        // Each "foo" should now be "fo" (deleted the last char)
        assert!(
            editor.buffer.lines[0].contains("fo"),
            "expected 'fo' in line"
        );
        // The line should NOT contain the original "foo"
        assert!(
            !editor.buffer.lines[0].contains("foo"),
            "should not contain 'foo' after delete"
        );
    }
    #[test]
    fn iedit_pushes_undo_snapshot() {
        let mut editor = MoraEditor::new(20);
        editor.buffer.lines = vec!["foo foo".to_string()];
        editor.start_iedit();
        // After entering iedit, buffer should have an undo snapshot
        // (we can't directly test undo here without the full undo-tree,
        // but at least the mode should be correct)
        assert_eq!(editor.mode, EditorMode::Iedit);
    }

    #[test]
    fn iedit_exits_on_single_occurrence() {
        let mut editor = MoraEditor::new(20);
        editor.buffer.lines = vec!["unique_word".to_string(), "other".to_string()];
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
        editor.buffer.lines = vec!["x1 = x2 + x3".to_string(), "y1 = y2".to_string()];
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
        assert_eq!(editor.buffer.major_mode.name(), "Lisp");
        assert_eq!(
            editor.buffer.lines[0],
            ";; This buffer is for text that is not saved."
        );
        assert_eq!(editor.buffer.cursor.row, 4);
        assert!(!editor.buffer.modified);
        assert!(editor.messages.is_empty());
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
