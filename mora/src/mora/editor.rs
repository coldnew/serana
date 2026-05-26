use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::buffer::Buffer;
use super::keymap::{self, EditorMode, KeyAction, PendingOp};
use super::kill_ring::KillRing;
use super::register::{RegisterValue, Registers};
use super::macro_state::MacroState;
use super::mark::MarkRing;
use super::rectangle::{self, RectRegion};
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
    pub kill_ring: KillRing,
    pub registers: Registers,
    pub mark_ring: MarkRing,
    pub macro_state: MacroState,
    pub wasm_host: WasmExtensionHost,
    pub last_search_forward: Option<String>,
    pub last_search_backward: Option<String>,
    pub status_message: String,
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
}

impl MoraEditor {
    pub fn new(height: usize) -> Self {
        let mut editor = Self {
            buffer: Buffer::new(),
            mode: EditorMode::Normal,
            view: View::new(height),
            command_input: String::new(),
            kill_ring: KillRing::new(),
            registers: Registers::new(),
            mark_ring: MarkRing::new(),
            macro_state: MacroState::new(),
            wasm_host: WasmExtensionHost::new(),
            last_search_forward: None,
            last_search_backward: None,
            status_message: String::new(),
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

    pub fn mode(&self) -> EditorMode { self.mode }
    pub fn buffer(&self) -> &Buffer { &self.buffer }
    pub fn view(&self) -> &View { &self.view }
    pub fn command_input(&self) -> &str { &self.command_input }
    pub fn status_message(&self) -> &str { &self.status_message }
    pub fn quit_requested(&self) -> bool { self.quit_requested }

    pub fn set_height(&mut self, height: usize) {
        self.view.height = height.max(1);
        self.view.ensure_cursor_visible(&self.buffer);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
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

        // Minor mode intercept: higher priority modes get first chance
        if let Some(action) = self.minor_modes.intercept_key(key) {
            return action;
        }

        match self.mode {
            EditorMode::Normal => self.handle_normal(key),
            EditorMode::Insert => keymap::insert_key(key),
            EditorMode::Command
            | EditorMode::SearchForward
            | EditorMode::SearchBackward => keymap::command_key(key),
            EditorMode::Emacs => self.handle_emacs(key),
            EditorMode::ReplaceChar => self.handle_replace_char(key),
            EditorMode::Visual => self.handle_visual(key),
            EditorMode::Iedit => self.handle_iedit(key),
        }
    }

    fn handle_prefix_key(&mut self, prefix: char, key: KeyEvent) -> KeyAction {
        match prefix {
            'x' => match (key.modifiers, key.code) {
                (_, KeyCode::Char('s')) | (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                    self.save_current_buffer();
                    KeyAction::SetMode(EditorMode::Normal)
                }
                (_, KeyCode::Char('c')) | (_, KeyCode::Char('k')) => KeyAction::Quit,
                (KeyModifiers::CONTROL, KeyCode::Char('c')) => KeyAction::ForceQuit,
                (_, KeyCode::Char('f')) => KeyAction::SetMode(EditorMode::SearchForward),
                (_, KeyCode::Char('b')) => {
                    let cmd = self.command_input.clone();
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
                (KeyModifiers::CONTROL, KeyCode::Char('r')) => KeyAction::Redo,
                (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
                    self.status_message = "Redraw screen".to_string();
                    KeyAction::None
                }
                // C-x C-t: transpose line
                (KeyModifiers::CONTROL, KeyCode::Char('t')) => KeyAction::TransposeLine,
                // C-x C-u: uppercase region
                (KeyModifiers::CONTROL, KeyCode::Char('u')) => KeyAction::UppercaseRegion,
                // C-x C-l: lowercase region
                (KeyModifiers::CONTROL, KeyCode::Char('l')) => KeyAction::LowercaseRegion,
                // C-x C-=: goto last change
                (KeyModifiers::CONTROL, KeyCode::Char('=')) => KeyAction::GotoLastChange,
                // C-x C-;: cleanup buffer (delete trailing whitespace)
                (KeyModifiers::CONTROL, KeyCode::Char(';')) => KeyAction::CleanupBuffer,
                // C-x C-m: dos2unix
                (KeyModifiers::CONTROL, KeyCode::Char('m')) => KeyAction::Dos2Unix,
                // C-x C-f: toggle fold
                (KeyModifiers::CONTROL, KeyCode::Char('f')) => KeyAction::ToggleFold,
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
                    self.mode = EditorMode::Command;
                    self.command_input.clear();
                    self.status_message = "Goto line: ".to_string();
                    KeyAction::None
                }
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
                self.registers
                    .set(name, RegisterValue::Text(content));
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
                    self.status_message = format!("Yanked {} lines from register {}", l.len(), name);
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
            return if key.code == KeyCode::Char('g') {
                KeyAction::MoveFileStart
            } else {
                KeyAction::None
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
                            if inner { KeyAction::DeleteInnerBrackets(open, close) }
                            else { KeyAction::DeleteAroundBrackets(open, close) }
                        }
                        Some(PendingOp::Change) => {
                            self.pending_action = Some(KeyAction::SetMode(EditorMode::Insert));
                            if inner { KeyAction::DeleteInnerBrackets(open, close) }
                            else { KeyAction::DeleteAroundBrackets(open, close) }
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
                    (PendingOp::Delete, KeyAction::MoveWordBackward) => KeyAction::DeleteWordBackward,
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
                KeyAction::MoveLeft | KeyAction::MoveRight
                | KeyAction::MoveUp | KeyAction::MoveDown
                | KeyAction::DeleteForward => {
                    return self.repeated_action(action, n);
                }
                _ => {}
            }
            return action;
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
                self.command_input.clear();
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
                self.mode = EditorMode::Command;
                self.command_input = String::new();
                KeyAction::None
            }

            (KeyModifiers::ALT, KeyCode::Char('w')) | (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
                KeyAction::Save
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
            (_, KeyCode::Tab) => KeyAction::InsertChar('\t'),
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
        let row = self.buffer.cursor.row;
        let col = self.buffer.cursor.col;
        let line = self.buffer.current_line();
        let chars: Vec<char> = line.chars().collect();

        // Find word under cursor
        let (word_start, word_end) = if col < chars.len() && chars[col].is_alphanumeric() || col < chars.len() && chars[col] == '_' {
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
                    let left_ok = c == 0 || !(line_chars[c - 1].is_alphanumeric() || line_chars[c - 1] == '_');
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
        self.status_message = format!("Iedit: {} ({} regions, Esc to exit)", word, self.iedit_regions.len());
    }

    fn handle_iedit(&mut self, key: KeyEvent) -> KeyAction {
        match (key.modifiers, key.code) {
            // Exit iedit
            (_, KeyCode::Esc) | (KeyModifiers::CONTROL, KeyCode::Char('g')) => {
                let word = self.iedit_word.take().unwrap_or_default();
                self.iedit_regions.clear();
                self.iedit_cursor_idx = 0;
                self.mode = EditorMode::Emacs;
                self.status_message = format!("Iedit exited (edited: {})", word);
                KeyAction::None
            }
            // Tab cycles between iedit regions
            (_, KeyCode::Tab) => {
                if !self.iedit_regions.is_empty() {
                    self.iedit_cursor_idx = (self.iedit_cursor_idx + 1) % self.iedit_regions.len();
                    let (r, c, _) = self.iedit_regions[self.iedit_cursor_idx];
                    self.buffer.cursor.row = r;
                    self.buffer.cursor.col = c;
                    self.view.ensure_cursor_visible(&self.buffer);
                }
                KeyAction::None
            }
            // Back-tab cycles backwards
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                if !self.iedit_regions.is_empty() {
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
                KeyAction::None
            }
            // C-n/C-p navigate between regions
            (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
                if !self.iedit_regions.is_empty() {
                    self.iedit_cursor_idx = (self.iedit_cursor_idx + 1) % self.iedit_regions.len();
                    let (r, c, _) = self.iedit_regions[self.iedit_cursor_idx];
                    self.buffer.cursor.row = r;
                    self.buffer.cursor.col = c;
                    self.view.ensure_cursor_visible(&self.buffer);
                }
                KeyAction::None
            }
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                if !self.iedit_regions.is_empty() {
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
                KeyAction::None
            }
            // Insert char in all regions
            (_, KeyCode::Char(c)) => {
                self.iedit_insert_char(c);
                KeyAction::None
            }
            // Backspace in all regions
            (_, KeyCode::Backspace) => {
                self.iedit_delete_backward();
                KeyAction::None
            }
            // Delete forward in all regions
            (_, KeyCode::Delete) => {
                self.iedit_delete_forward();
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
                let byte_pos: usize = self.buffer.lines[row].chars().take(insert_at).map(|ch| ch.len_utf8()).sum();
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
                let byte_pos: usize = self.buffer.lines[row].chars().take(end - 1).map(|ch| ch.len_utf8()).sum();
                let char_byte_len = self.buffer.lines[row].chars().nth(end - 1).map(|ch| ch.len_utf8()).unwrap_or(0);
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
        for i in (0..self.iedit_regions.len()).rev() {
            let (row, _start, end) = self.iedit_regions[i];
            if row < self.buffer.lines.len() {
                let line_chars: Vec<char> = self.buffer.lines[row].chars().collect();
                if end < line_chars.len() {
                    let byte_pos: usize = line_chars.iter().take(end).map(|ch| ch.len_utf8()).sum();
                    let char_byte_len = line_chars[end].len_utf8();
                    drop(line_chars);
                    self.buffer.lines[row].drain(byte_pos..byte_pos + char_byte_len);
                    self.buffer.modified = true;
                    for j in 0..self.iedit_regions.len() {
                        if self.iedit_regions[j].0 == row {
                            if self.iedit_regions[j].2 > end {
                                self.iedit_regions[j].2 -= 1;
                            }
                            if self.iedit_regions[j].1 > end {
                                self.iedit_regions[j].1 -= 1;
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
                    if inner { self.buffer.inner_bracket_range(open, close) }
                    else { self.buffer.around_bracket_range(open, close) }
                } else if c == 'w' || c == 'W' {
                    if inner { self.buffer.inner_word_range() }
                    else { self.buffer.around_word_range() }
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

            KeyAction::InsertChar(c) => { self.record_change(); self.buffer.insert_char(c); }
            KeyAction::InsertNewline => { self.record_change(); self.buffer.insert_newline(); }
            KeyAction::DeleteBackward => { self.record_change(); self.buffer.delete_backward(); }
            KeyAction::DeleteForward => { self.record_change(); self.buffer.delete_forward(); }
            KeyAction::DeleteLine => {
                self.record_change();
                self.kill_line_to_ring();
                self.buffer.delete_line();
            }
            KeyAction::DeleteToEol => { self.record_change(); self.buffer.delete_to_eol(); }
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
                    while i > 0 && line[..col].chars().nth(i - 1).map_or(false, |c| c.is_whitespace()) {
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
                while end < chars.len() && chars[end].is_whitespace() { end += 1; }
                while end < chars.len() && !chars[end].is_whitespace() { end += 1; }
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
                        while end < chars.len() && is_word(chars[end]) { end += 1; }
                    } else if chars[col].is_whitespace() {
                        while end < chars.len() && chars[end].is_whitespace() { end += 1; }
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
                while end < chars.len() && chars[end].is_whitespace() { end += 1; }
                while end < chars.len() && !chars[end].is_whitespace() { end += 1; }
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
                            self.buffer.lines[start.row] = first[..start.col].to_string() + &last[end.col..];
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
                    let input = self.command_input.clone();
                    if !input.is_empty() {
                        rectangle::insert_rectangle(&mut self.buffer, &rect, &input);
                        self.status_message = "Inserted rectangle".to_string();
                    }
                    self.mark_ring.set_active(false);
                    self.mode = EditorMode::Normal;
                    self.command_input.clear();
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
                    self.status_message = format!(
                        "Yank-pop ({})",
                        self.kill_ring.len()
                    );
                }
            }

            KeyAction::Undo => self.buffer.undo(),
            KeyAction::Redo => self.buffer.redo(),

            KeyAction::SetMode(mode) => {
                if mode == EditorMode::Visual && self.mode != EditorMode::Visual {
                    self.mark_ring.push(self.buffer.cursor);
                    self.mark_ring.set_active(true);
                }
                self.mode = mode;
                if mode == EditorMode::Normal {
                    self.command_input.clear();
                    self.waiting_g = false;
                    self.waiting_op = None;
                    if !self.mark_ring.is_active() {
                        self.mark_ring.clear();
                    }
                }
            }

            KeyAction::Save => self.save_current_buffer(),
            KeyAction::SaveAs => {
                let path_str = self
                    .command_input
                    .strip_prefix("w ")
                    .unwrap_or(&self.command_input);
                let path = PathBuf::from(path_str.trim());
                match self.buffer.save_as(&path) {
                    Ok(()) => self.status_message = format!("Saved: {}", path.display()),
                    Err(e) => self.status_message = format!("Save error: {}", e),
                }
                self.mode = EditorMode::Normal;
                self.command_input.clear();
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
                self.command_input.push(c);
            }
            KeyAction::InputBackspace => {
                self.command_input.pop();
            }
            KeyAction::ExecuteInput => {
                self.execute_command();
            }
            KeyAction::GotoLine(n) => {
                self.buffer.goto_line(n);
                self.mode = EditorMode::Normal;
                self.command_input.clear();
            }

            KeyAction::FindForward(query) => {
                self.last_search_forward = Some(query.clone());
                if !self.buffer.find_forward(&query) {
                    self.status_message = format!("Pattern not found: {}", query);
                }
                self.mode = EditorMode::Normal;
                self.command_input.clear();
            }
            KeyAction::FindBackward(query) => {
                self.last_search_backward = Some(query.clone());
                if !self.buffer.find_backward(&query) {
                    self.status_message = format!("Pattern not found: {}", query);
                }
                self.mode = EditorMode::Normal;
                self.command_input.clear();
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
                self.command_input.clear();
            }
            KeyAction::ReplaceAll(old, new) => {
                let count = self.buffer.replace_all(&old, &new);
                self.status_message = format!("Replaced {} occurrence(s)", count);
                self.mode = EditorMode::Normal;
                self.command_input.clear();
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
                            line.char_indices().nth(char_idx).map(|(i, _)| i).unwrap_or(col)..
                            line.char_indices().nth(char_idx + 1).map(|(i, _)| i).unwrap_or(line.len()),
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
                    let len = self.registers.get('e').map(|v| match v {
                        RegisterValue::Macro(e) => e.len(),
                        _ => 0,
                    }).unwrap_or(0);
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

            KeyAction::TransposeChar => { self.record_change(); self.buffer.transpose_char(); }
            KeyAction::TransposeWord => { self.record_change(); self.buffer.transpose_word(); }
            KeyAction::TransposeLine => { self.record_change(); self.buffer.transpose_line(); }
            KeyAction::CapitalizeWord => { self.record_change(); self.buffer.capitalize_word(); }
            KeyAction::UppercaseWord => { self.record_change(); self.buffer.uppercase_word(); }
            KeyAction::LowercaseWord => { self.record_change(); self.buffer.lowercase_word(); }
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
                let first_non_ws = if first_non_ws >= line.chars().count() { 0 } else { first_non_ws };
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
                    let last_non_ws = chars.iter().enumerate().rev()
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
                        while start > 0 && start - 1 < chars.len() && !chars[start - 1].is_whitespace() {
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
                        self.mark_ring.push(super::buffer::Cursor { row, col: start });
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
                        self.mark_ring.push(super::buffer::Cursor { row: start_row, col: 0 });
                        let end_len = lines[end_row].chars().count();
                        self.buffer.cursor.row = end_row;
                        self.buffer.cursor.col = end_len;
                        self.expand_region_level = 3;
                    }
                    _ => {
                        self.mark_ring.push(super::buffer::Cursor { row: 0, col: 0 });
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
                self.status_message = format!("({}/{})", self.dabbrev_index, self.dabbrev_matches.len());
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
                        self.status_message = format!("Narrowed to lines {}-{}", start + 1, end + 1);
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
                    self.status_message = format!("Folded at indent level {}", self.buffer.fold_level.unwrap_or(0));
                } else {
                    self.status_message = "Unfolded".to_string();
                }
            }
            KeyAction::SwitchMajorMode(ref name) => {
                if let Some(kind) = super::major_mode::parse_mode_name(name) {
                    self.buffer.major_mode = super::major_mode::create_mode(kind);
                    self.status_message = format!("Switched to {} mode", self.buffer.major_mode.name());
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
                        if let Some((r, c)) = self.buffer.search_backward_from(&word, last_row, last_col) {
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
                            if chars[i] == open { depth += 1; }
                            else if chars[i] == close { depth -= 1; }
                            if depth == 0 {
                                self.buffer.cursor.col = i;
                                return;
                            }
                        }
                    } else {
                        let chars: Vec<char> = line.chars().collect();
                        for i in (0..=col).rev() {
                            if chars[i] == close { depth += 1; }
                            else if chars[i] == open { depth -= 1; }
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
        }
    }

    fn execute_command(&mut self) {
        let input = self.command_input.clone();
        let trimmed = input.trim();

        // Handle M-x commands that need special treatment
        match trimmed {
            "iedit" | "multi-cursor-edit" => {
                self.mode = EditorMode::Emacs;
                self.command_input.clear();
                self.start_iedit();
                return;
            }
            "goto-line" | "goto-line-number" => {
                self.command_input.clear();
                self.status_message = "Goto line: ".to_string();
                // Stay in command mode for user to type line number
                return;
            }
            "recenter" | "recenter-top-bottom" => {
                let half = self.view.height / 2;
                let row = self.buffer.cursor.row;
                self.view.scroll_top = row.saturating_sub(half);
                self.mode = EditorMode::Emacs;
                self.command_input.clear();
                self.status_message = "Recentered".to_string();
                return;
            }
            "describe-mode" => {
                let desc = match self.mode {
                    EditorMode::Emacs => "Emacs mode: C-x prefix, C-c prefix, M-x commands",
                    EditorMode::Normal => "Normal mode: vi-style keybindings",
                    _ => "Current mode",
                };
                self.status_message = desc.to_string();
                self.mode = EditorMode::Emacs;
                self.command_input.clear();
                return;
            }
            "what-cursor-position" => {
                self.status_message = format!("Line {} Col {} ({} lines total)",
                    self.buffer.cursor.row + 1,
                    self.buffer.cursor.col + 1,
                    self.buffer.line_count());
                self.mode = EditorMode::Emacs;
                self.command_input.clear();
                return;
            }
            _ if trimmed.starts_with("switch-mode ") || trimmed.starts_with("set-mode ") => {
                let prefix = if trimmed.starts_with("switch-mode ") { "switch-mode " } else { "set-mode " };
                let mode_name = trimmed[prefix.len()..].trim();
                if let Some(kind) = super::major_mode::parse_mode_name(mode_name) {
                    self.buffer.major_mode = super::major_mode::create_mode(kind);
                    self.status_message = format!("Switched to {} mode", self.buffer.major_mode.name());
                } else {
                    self.status_message = format!("Unknown mode: {}", mode_name);
                }
                self.mode = EditorMode::Emacs;
                self.command_input.clear();
                return;
            }
            _ => {}
        }

        let action = match self.mode {
            EditorMode::Command => {
                if trimmed == "wq" || trimmed == "x" {
                    if self.buffer.path.is_some() {
                        let _ = self.buffer.save();
                    }
                    self.quit_requested = true;
                    self.mode = EditorMode::Normal;
                    self.command_input.clear();
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
            self.command_input.clear();
        }
    }

    fn mx_complete(&mut self) {
        let commands = [
            "capitalize-word", "cleanup-buffer", "copy-and-comment",
            "describe-mode", "disable-minor-mode", "dos2unix", "enable-minor-mode",
            "goto-last-change", "goto-line",
            "iedit", "kill-buffer", "kill-emacs", "lowercase-word",
            "lowercase-region", "multi-cursor-edit", "narrow-to-region",
            "recenter", "replace-string", "save-buffer", "save-some-buffers",
            "set-mode", "switch-mode",
            "toggle-fold", "toggle-minor-mode",
            "transpose-char", "transpose-line", "transpose-word",
            "unix2dos", "uppercase-region", "uppercase-word",
            "what-cursor-position", "widen",
        ];
        let input = self.command_input.trim();
        if input.is_empty() {
            return;
        }
        let matches: Vec<&str> = commands.iter()
            .filter(|c| c.starts_with(input))
            .copied()
            .collect();
        if matches.len() == 1 {
            self.command_input = matches[0].to_string();
        } else if matches.len() > 1 {
            // Find common prefix
            let common = matches.iter().fold(matches[0].to_string(), |acc, m| {
                acc.chars().zip(m.chars())
                    .take_while(|(a, b)| a == b)
                    .map(|(a, _)| a)
                    .collect()
            });
            if common.len() > input.len() {
                self.command_input = common;
            } else {
                self.status_message = matches.join("  ");
            }
        }
    }

    fn extract_text_between(&self, start: crate::mora::buffer::Cursor, end: crate::mora::buffer::Cursor) -> String {
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
            let max_row = self.buffer.narrow_end.unwrap_or(self.buffer.line_count().saturating_sub(1));
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
                Ok(()) => self.status_message = format!("Saved: {}", self.buffer.filename()),
                Err(e) => self.status_message = format!("Save error: {}", e),
            }
        } else {
            self.mode = EditorMode::Command;
            self.command_input = "w ".to_string();
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
            let forward = if same_direction { self.last_find_forward } else { !self.last_find_forward };
            let till = self.last_find_till;
            let line = self.buffer.current_line();
            let col = self.buffer.cursor.col;
            let chars: Vec<char> = line.chars().collect();
            if forward {
                let start = if till { col + 1 } else { col };
                if let Some(pos) = chars[start..].iter().position(|&ch| ch == c) {
                    let target = start + pos;
                    self.buffer.cursor.col = if till { target.saturating_sub(1) } else { target };
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
                self.status_message = format!("Jump to: {}", 
                    self.ace_jump_hints.iter().map(|(_, _, h)| *h).collect::<String>());
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

    fn split_window_horizontal(&mut self) {
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

    fn split_window_vertical(&mut self) {
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

    fn other_window(&mut self) {
        if self.windows.len() <= 1 {
            return;
        }
        self.sync_buffer_to_window();
        self.current_window_idx = (self.current_window_idx + 1) % self.windows.len();
        self.sync_window_to_buffer();
        self.status_message = format!("Window {}/{}", self.current_window_idx + 1, self.windows.len());
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

fn op_char(op: PendingOp) -> char {
    match op {
        PendingOp::Delete => 'd',
        PendingOp::Yank => 'y',
        PendingOp::Change => 'c',
    }
}
