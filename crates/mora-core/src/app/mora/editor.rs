use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent};

use super::buffer::Buffer;
use super::keymap::{self, EditorMode, KeyAction, PendingOp};
use super::view::View;

#[derive(Debug, Default)]
struct Clipboard {
    lines: Vec<String>,
}

pub struct MoraEditor {
    buffer: Buffer,
    mode: EditorMode,
    view: View,
    command_input: String,
    clipboard: Clipboard,
    last_find_forward: Option<String>,
    last_find_backward: Option<String>,
    status_message: String,
    quit_requested: bool,
    pending_action: Option<KeyAction>,
    waiting_g: bool,
    waiting_op: Option<PendingOp>,
}

impl MoraEditor {
    pub fn new(height: usize) -> Self {
        Self {
            buffer: Buffer::new(),
            mode: EditorMode::Normal,
            view: View::new(height),
            command_input: String::new(),
            clipboard: Clipboard::default(),
            last_find_forward: None,
            last_find_backward: None,
            status_message: String::new(),
            quit_requested: false,
            pending_action: None,
            waiting_g: false,
            waiting_op: None,
        }
    }

    pub fn open(path: &Path, height: usize) -> std::io::Result<Self> {
        let buffer = Buffer::from_file(path)?;
        let mut editor = Self::new(height);
        editor.buffer = buffer;
        editor.status_message = format!("Opened: {}", path.display());
        Ok(editor)
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
        &self.command_input
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

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if let Some(action) = self.pending_action.take() {
            self.execute_action(action);
            self.view.ensure_cursor_visible(&self.buffer);
            return true;
        }

        let action = match self.mode {
            EditorMode::Normal => self.handle_normal(key),
            EditorMode::Insert => keymap::insert_key(key),
            EditorMode::Command
            | EditorMode::SearchForward
            | EditorMode::SearchBackward => keymap::command_key(key),
            EditorMode::Emacs => keymap::emacs_key(key),
            EditorMode::ReplaceChar => self.handle_replace_char(key),
            EditorMode::Visual => self.handle_visual(key),
        };

        let redraw = action != KeyAction::None;
        self.execute_action(action);
        self.view.ensure_cursor_visible(&self.buffer);
        redraw
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

        let action = keymap::normal_key(key);

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

    fn handle_replace_char(&mut self, key: KeyEvent) -> KeyAction {
        self.mode = EditorMode::Normal;
        match key.code {
            KeyCode::Esc => KeyAction::None,
            KeyCode::Char(c) => KeyAction::ReplaceChar(c),
            _ => KeyAction::None,
        }
    }

    fn handle_visual(&mut self, key: KeyEvent) -> KeyAction {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => KeyAction::SetMode(EditorMode::Normal),
            (_, KeyCode::Char('h')) | (_, KeyCode::Left) => KeyAction::MoveLeft,
            (_, KeyCode::Char('j')) | (_, KeyCode::Down) => KeyAction::MoveDown,
            (_, KeyCode::Char('k')) | (_, KeyCode::Up) => KeyAction::MoveUp,
            (_, KeyCode::Char('l')) | (_, KeyCode::Right) => KeyAction::MoveRight,
            (_, KeyCode::Char('d')) | (_, KeyCode::Char('x')) => KeyAction::DeleteLine,
            (_, KeyCode::Char('y')) => KeyAction::YankLine,
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

            KeyAction::InsertChar(c) => self.buffer.insert_char(c),
            KeyAction::InsertNewline => self.buffer.insert_newline(),
            KeyAction::DeleteBackward => self.buffer.delete_backward(),
            KeyAction::DeleteForward => self.buffer.delete_forward(),
            KeyAction::DeleteLine => {
                self.yank_current_line();
                self.buffer.delete_line();
            }
            KeyAction::DeleteToEol => self.buffer.delete_to_eol(),
            KeyAction::DeleteWordForward => {
                let start = self.buffer.cursor.col;
                self.buffer.move_word_forward();
                let end = self.buffer.cursor.col;
                if end > start {
                    let row = self.buffer.cursor.row;
                    for _ in 0..(end - start) {
                        if start < self.buffer.lines[row].len() {
                            self.buffer.lines[row].remove(start);
                        }
                    }
                    self.buffer.cursor.col = start;
                    self.buffer.modified = true;
                }
            }

            KeyAction::Undo => self.buffer.undo(),
            KeyAction::Redo => self.buffer.redo(),

            KeyAction::SetMode(mode) => {
                self.mode = mode;
                if mode == EditorMode::Normal {
                    self.command_input.clear();
                    self.waiting_g = false;
                    self.waiting_op = None;
                }
            }

            KeyAction::Save => {
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
            KeyAction::SaveAs => {
                let path_str = self.command_input.strip_prefix("w ").unwrap_or(&self.command_input);
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
                    self.status_message = "Unsaved changes! Use :q! to force quit".to_string();
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
                self.last_find_forward = Some(query.clone());
                if !self.buffer.find_forward(&query) {
                    self.status_message = format!("Pattern not found: {}", query);
                }
                self.mode = EditorMode::Normal;
                self.command_input.clear();
            }
            KeyAction::FindBackward(query) => {
                self.last_find_backward = Some(query.clone());
                if !self.buffer.find_backward(&query) {
                    self.status_message = format!("Pattern not found: {}", query);
                }
                self.mode = EditorMode::Normal;
                self.command_input.clear();
            }
            KeyAction::RepeatFind(forward) => {
                if forward {
                    if let Some(ref q) = self.last_find_forward.clone() {
                        if !self.buffer.find_forward(q) {
                            self.status_message = format!("Pattern not found: {}", q);
                        }
                    }
                } else {
                    if let Some(ref q) = self.last_find_backward.clone() {
                        if !self.buffer.find_backward(q) {
                            self.status_message = format!("Pattern not found: {}", q);
                        }
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
                self.yank_current_line();
                self.status_message = "Yanked line".to_string();
            }
            KeyAction::PasteAfter => {
                if !self.clipboard.lines.is_empty() {
                    let paste_row = self.buffer.cursor.row + 1;
                    for (i, line) in self.clipboard.lines.iter().enumerate() {
                        self.buffer.lines.insert(paste_row + i, line.clone());
                    }
                    self.buffer.modified = true;
                    self.buffer.cursor.row = paste_row;
                    self.buffer.cursor.col = 0;
                }
            }

            KeyAction::IndentLine => {
                let line = &mut self.buffer.lines[self.buffer.cursor.row];
                line.insert_str(0, "    ");
                self.buffer.modified = true;
            }
            KeyAction::UnindentLine => {
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
                let row = self.buffer.cursor.row;
                let col = self.buffer.cursor.col;
                if col < self.buffer.lines[row].len() {
                    self.buffer.push_undo_snapshot();
                    let line = &mut self.buffer.lines[row];
                    let char_idx = line[..col].chars().count();
                    let chars: Vec<char> = line.chars().collect();
                    if char_idx < chars.len() {
                        let mut new_line = String::new();
                        for (i, ch) in chars.iter().enumerate() {
                            if i == char_idx {
                                new_line.push(c);
                            } else {
                                new_line.push(*ch);
                            }
                        }
                        *line = new_line;
                        self.buffer.modified = true;
                    }
                }
            }

            KeyAction::ToggleVisual => {
                self.mode = if self.mode == EditorMode::Visual {
                    EditorMode::Normal
                } else {
                    EditorMode::Visual
                };
            }

            KeyAction::SwitchToEmacs => {
                self.mode = EditorMode::Emacs;
            }
        }
    }

    fn execute_command(&mut self) {
        let input = self.command_input.clone();
        let action = match self.mode {
            EditorMode::Command => {
                let trimmed = input.trim();
                if trimmed == "wq" || trimmed == "x" || trimmed == "wq!" {
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

    fn yank_current_line(&mut self) {
        self.clipboard.lines = vec![self.buffer.lines[self.buffer.cursor.row].clone()];
    }
}

fn op_char(op: PendingOp) -> char {
    match op {
        PendingOp::Delete => 'd',
        PendingOp::Yank => 'y',
        PendingOp::Change => 'c',
    }
}
