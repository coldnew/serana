use super::{op_char, MoraEditor};
use crate::mora::display::event::{
    MoraKeyCode as KeyCode, MoraKeyEvent as KeyEvent, MoraKeyModifiers as KeyModifiers,
};
use crate::mora::keymap::{self, EditorMode, KeyAction, PendingOp};
use crate::mora::rectangle::RectRegion;
use crate::mora::register::RegisterValue;

impl MoraEditor {
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
                KeyCode::Char('n') => KeyAction::MoveDown,   // next buffer
                KeyCode::Char('p') => KeyAction::MoveUp,     // previous buffer
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
                        crate::mora::theme::day()
                    } else {
                        crate::mora::theme::night()
                    };
                    let mode = if self.theme.background.r < 128 {
                        "Night"
                    } else {
                        "Day"
                    };
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
                    self.status_message = "AI chat (serana)".to_string();
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

    pub(super) fn start_iedit(&mut self) {
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

    pub(super) fn iedit_insert_char(&mut self, c: char) {
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

    pub(super) fn iedit_delete_backward(&mut self) {
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

    pub(super) fn iedit_delete_forward(&mut self) {
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

    pub(super) fn iedit_next_region(&mut self) {
        if self.iedit_regions.is_empty() {
            return;
        }
        self.iedit_cursor_idx = (self.iedit_cursor_idx + 1) % self.iedit_regions.len();
        let (r, c, _) = self.iedit_regions[self.iedit_cursor_idx];
        self.buffer.cursor.row = r;
        self.buffer.cursor.col = c;
        self.view.ensure_cursor_visible(&self.buffer);
    }

    pub(super) fn iedit_prev_region(&mut self) {
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

    pub(super) fn iedit_skip_region(&mut self) {
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

    pub(super) fn iedit_add_region_at_cursor(&mut self) {
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
            self.status_message = format!(
                "Iedit: word at cursor is \"{}\", expected \"{}\"",
                candidate, word
            );
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

    pub(super) fn iedit_refind_all(&mut self) {
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
                    let candidate: String =
                        line_chars[c..c + word.chars().count()].iter().collect();
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

    pub(super) fn iedit_kill_region(&mut self) {
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

    pub(super) fn iedit_update_status(&mut self) {
        let word = self.iedit_word.as_deref().unwrap_or("");
        let mode = if self.iedit_regex { "regex" } else { "word" };
        self.status_message = format!(
            "Iedit: {} \"{}\" [{} region{}]  Tab next  C-d skip  C-; add  C-a refind  Esc exit",
            mode,
            word,
            self.iedit_regions.len(),
            if self.iedit_regions.len() == 1 {
                ""
            } else {
                "s"
            }
        );
    }

    pub(super) fn start_iedit_regex(&mut self) {
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
}
