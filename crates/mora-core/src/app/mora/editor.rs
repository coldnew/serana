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
            self.view.ensure_cursor_visible(&self.buffer);
            return true;
        }

        if let Some(name) = self.waiting_register.take() {
            self.handle_register_key(name, key);
            self.view.ensure_cursor_visible(&self.buffer);
            return true;
        }

        let action = self.reduce_no_playback(key);

        let redraw = action != KeyAction::None;
        self.execute_action(action);
        self.view.ensure_cursor_visible(&self.buffer);
        redraw
    }

    pub fn drain_macro_events(&mut self) -> Option<KeyEvent> {
        self.macro_state.next_event()
    }

    fn reduce_no_playback(&mut self, key: KeyEvent) -> KeyAction {
        if let Some(prefix) = self.waiting_prefix.take() {
            return self.handle_prefix_key(prefix, key);
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
                _ => KeyAction::None,
            },
            'c' => match key.code {
                KeyCode::Char('c') => KeyAction::ForceQuit,
                KeyCode::Char('s') => {
                    self.save_current_buffer();
                    KeyAction::SetMode(EditorMode::Normal)
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
        if self.waiting_g {
            self.waiting_g = false;
            return if key.code == KeyCode::Char('g') {
                KeyAction::MoveFileStart
            } else {
                KeyAction::None
            };
        }

        if let Some(op) = self.waiting_op {
            self.waiting_op = None;
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
                self.handle_normal(key)
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

        if key.code == KeyCode::Char('g') && key.modifiers.is_empty() {
            self.waiting_g = true;
            return KeyAction::None;
        }

        match &action {
            KeyAction::None if key.code == KeyCode::Char('d') && key.modifiers.is_empty() => {
                self.waiting_op = Some(PendingOp::Delete);
                return KeyAction::None;
            }
            KeyAction::None if key.code == KeyCode::Char('y') && key.modifiers.is_empty() => {
                self.waiting_op = Some(PendingOp::Yank);
                return KeyAction::None;
            }
            KeyAction::None if key.code == KeyCode::Char('c') && key.modifiers.is_empty() => {
                self.waiting_op = Some(PendingOp::Change);
                return KeyAction::None;
            }
            _ => {}
        }

        if let Some(post) = keymap::normal_key_post(key) {
            self.pending_action = Some(post);
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

            (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
                self.status_message = "Recentered".to_string();
                let half = self.view.height / 2;
                let row = self.buffer.cursor.row;
                self.view.scroll_top = row.saturating_sub(half);
                KeyAction::None
            }

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
            // Emacs: C-o insert empty line below
            (KeyModifiers::CONTROL, KeyCode::Char('o')) => KeyAction::InsertEmptyLineBelow,
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
                self.buffer.delete_forward();
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
                for row in rect.start_row..=rect.end_row.min(self.buffer.lines.len() - 1) {
                    self.buffer.lines[row].insert_str(0, "    ");
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
                let line = &mut self.buffer.lines[self.buffer.cursor.row];
                line.insert_str(0, "    ");
                self.buffer.modified = true;
            }
            KeyAction::UnindentLine => {
                self.record_change();
                let line = &mut self.buffer.lines[self.buffer.cursor.row];
                if line.starts_with("    ") {
                    *line = line[4..].to_string();
                    self.buffer.modified = true;
                } else if line.starts_with('\t') {
                    *line = line[1..].to_string();
                    self.buffer.modified = true;
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
        }
    }

    fn execute_command(&mut self) {
        let input = self.command_input.clone();
        let action = match self.mode {
            EditorMode::Command => {
                let trimmed = input.trim();
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

    fn extract_text_between(&self, start: crate::app::mora::buffer::Cursor, end: crate::app::mora::buffer::Cursor) -> String {
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
}

fn op_char(op: PendingOp) -> char {
    match op {
        PendingOp::Delete => 'd',
        PendingOp::Yank => 'y',
        PendingOp::Change => 'c',
    }
}
