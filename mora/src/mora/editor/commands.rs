use std::path::PathBuf;

use super::MoraEditor;
use crate::mora::display::event::{MoraKeyCode as KeyCode, MoraKeyEvent as KeyEvent};
use crate::mora::keymap::{self, EditorMode, KeyAction};
use crate::mora::minibuffer::{CompletionResult, Minibuffer};
use crate::mora::rectangle::{self, RectRegion};
use crate::mora::register::RegisterValue;

impl MoraEditor {
    pub(super) fn execute_action(&mut self, action: KeyAction) {
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
                // Tab: check for snippet expansion
                if c == '\t' {
                    if self.try_expand_snippet() {
                        return;
                    }
                }
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
                    self.mark_ring
                        .push(crate::mora::buffer::Cursor { row, col });
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
                            .push(crate::mora::buffer::Cursor { row, col: start });
                        self.buffer.cursor.col = end;
                    }
                    1 => {
                        self.mark_ring
                            .push(crate::mora::buffer::Cursor { row, col: 0 });
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
                        self.mark_ring.push(crate::mora::buffer::Cursor {
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
                            .push(crate::mora::buffer::Cursor { row: 0, col: 0 });
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
                if let Some(kind) = crate::mora::major_mode::parse_mode_name(name) {
                    self.buffer.major_mode = crate::mora::major_mode::create_mode(kind);
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

    pub(super) fn execute_command(&mut self) {
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
                            if r == self.buffer.cursor.row
                                && m.start() <= self.buffer.cursor.col
                                && self.buffer.cursor.col <= m.end()
                            {
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
                    crate::mora::theme::day()
                } else {
                    crate::mora::theme::night()
                };
                let mode = if self.theme.background.r < 128 {
                    "Night"
                } else {
                    "Day"
                };
                self.status_message = format!("Theme: {mode} (coldnew-{mode})");
                self.clear_minibuffer();
                return;
            }
            "view-messages" | "view-echo-area-messages" => {
                let count = self.messages.len();
                let start = if count > 20 { count - 20 } else { 0 };
                let recent: Vec<&str> = self.messages[start..].iter().map(|s| s.as_str()).collect();
                self.status_message =
                    format!("*Messages* ({} entries): {}", count, recent.join("; "));
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
                if let Some(kind) = crate::mora::major_mode::parse_mode_name(mode_name) {
                    self.buffer.major_mode = crate::mora::major_mode::create_mode(kind);
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
                let mut state = crate::mora::lisp_ext::EditorState::new();
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
                    crate::mora::overlay::OverlayStore::new(),
                );
                crate::mora::lisp_ext::set_editor_state(state);

                let result = self.lisp_bridge.eval(code);

                // Sync state back
                if let Some(state) = crate::mora::lisp_ext::take_editor_state() {
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
                    let mut state = crate::mora::lisp_ext::EditorState::new();
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
                        crate::mora::overlay::OverlayStore::new(),
                    );
                    crate::mora::lisp_ext::set_editor_state(state);

                    let result = self.lisp_bridge.eval(&format!(
                        "(shell-command \"{}\")",
                        cmd.replace('\\', "\\\\").replace('"', "\\\"")
                    ));

                    if let Some(state) = crate::mora::lisp_ext::take_editor_state() {
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
        let mut state = crate::mora::lisp_ext::EditorState::new();
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
            crate::mora::overlay::OverlayStore::new(),
        );
        crate::mora::lisp_ext::set_editor_state(state);
    }
    fn pull_lisp_state(&mut self) {
        if let Some(state) = crate::mora::lisp_ext::take_editor_state() {
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

    pub(super) fn command_candidates(&self) -> Vec<String> {
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

    pub(super) fn mx_complete(&mut self) {
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

    pub(super) fn clamp_cursor_to_narrow(&mut self) {
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

    pub(super) fn toggle_record_macro(&mut self) {
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

    pub(super) fn repeated_action(&mut self, action: KeyAction, n: usize) -> KeyAction {
        for _ in 0..n - 1 {
            self.execute_action(action.clone());
        }
        action
    }

    pub(super) fn save_current_buffer(&mut self) {
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

    pub(super) fn run_git_status(&mut self) {
        let output = self.run_git_command(&["status", "--short"]);
        self.status_message = if output.trim().is_empty() {
            "Git: clean working tree".to_string()
        } else {
            // Show first line in status bar
            let first_line = output.lines().next().unwrap_or("");
            format!("Git: {first_line} ({} files)", output.lines().count())
        };
    }

    pub(super) fn run_spelling_check(&mut self) {
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
                    let suggestions = stdout
                        .lines()
                        .find(|l| l.starts_with("& "))
                        .and_then(|l| l.split(':').nth(1))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();
                    self.status_message =
                        format!("Spelling: \"{word}\" — suggestions: {suggestions}");
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

    pub(super) fn run_go_to_definition(&mut self) {
        let word = self.buffer.current_word();
        if word.is_empty() {
            self.status_message = "Go to definition: no word under cursor".to_string();
            return;
        }
        if self.lsp_client.is_connected() {
            let uri = self
                .buffer
                .path
                .as_ref()
                .map(|p| format!("file://{}", p.display()))
                .unwrap_or_default();
            let line = self.buffer.cursor.row as u32;
            let col = self.buffer.cursor.col as u32;
            match self.lsp_client.definition(&uri, line, col) {
                Ok(locs) if !locs.is_empty() => {
                    let loc = &locs[0];
                    self.status_message = format!(
                        "Definition: {} L{}:C{}",
                        loc.uri,
                        loc.range.start.line + 1,
                        loc.range.start.character + 1
                    );
                }
                Ok(_) => {
                    self.status_message = format!("Definition: no definition found for \"{word}\"")
                }
                Err(e) => self.status_message = format!("Definition: {e}"),
            }
        } else {
            self.status_message =
                format!("Go to definition: \"{word}\" — use SPC p g to grep (LSP not connected)");
        }
    }

    /// Returns true if a snippet was expanded.
    fn try_expand_snippet(&mut self) -> bool {
        // Get the word before cursor
        let word_before = self.word_before_cursor();
        if word_before.is_empty() {
            return false;
        }

        // Detect file type
        let file_type = self
            .buffer
            .path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let lang = match file_type {
            "rs" => "rust",
            "py" => "python",
            "js" | "jsx" => "javascript",
            "ts" | "tsx" => "typescript",
            "go" => "go",
            "c" | "h" => "c",
            "cpp" | "hpp" | "cc" => "c",
            _ => "",
        };

        // Check if word matches a snippet trigger
        if let Some(snippet) = self.snippet_engine.find_exact(&word_before, lang) {
            let body = snippet.body.clone();

            // Delete the trigger word
            let word_len = word_before.len();
            let col = self.buffer.cursor.col;
            let start = col.saturating_sub(word_len);
            self.buffer.cursor.col = start;
            let line = &mut self.buffer.lines[self.buffer.cursor.row];
            line.drain(start..start + word_len);

            // Expand the snippet
            let expansion = crate::mora::snippet::SnippetEngine::expand(&body);

            // Insert the expanded text
            let lines: Vec<&str> = expansion.body.split('\n').collect();
            if lines.len() == 1 {
                let line = &mut self.buffer.lines[self.buffer.cursor.row];
                line.insert_str(start, lines[0]);
                // Position cursor at first placeholder
                if let Some(ph) = expansion.current() {
                    self.buffer.cursor.col = start + ph.offset;
                }
            } else {
                // Multi-line: insert first line, then new lines
                let current_line = self.buffer.lines[self.buffer.cursor.row].clone();
                let (before, _) = current_line.split_at(start);
                self.buffer.lines[self.buffer.cursor.row] = format!("{}{}", before, lines[0]);
                for (i, line) in lines[1..].iter().enumerate() {
                    self.buffer
                        .lines
                        .insert(self.buffer.cursor.row + 1 + i, line.to_string());
                }
                // Position cursor at first placeholder
                if let Some(ph) = expansion.current() {
                    // Calculate line/col from offset
                    let mut remaining = ph.offset;
                    for (i, line) in lines.iter().enumerate() {
                        if remaining <= line.len() {
                            self.buffer.cursor.row += i;
                            self.buffer.cursor.col =
                                if i == 0 { start + remaining } else { remaining };
                            break;
                        }
                        remaining -= line.len() + 1; // +1 for \n
                    }
                }
            }

            self.buffer.modified = true;
            self.status_message = format!("Snippet: {}", snippet.description);
            return true;
        }
        false
    }

    /// Get the word immediately before the cursor on the current line.
    fn word_before_cursor(&self) -> String {
        let line = self.buffer.current_line();
        let col = self.buffer.cursor.col;
        if col == 0 {
            return String::new();
        }
        let chars: Vec<char> = line.chars().collect();
        let col = col.min(chars.len());
        let mut start = col;
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }
        chars[start..col].iter().collect()
    }

    pub(super) fn start_lsp(&mut self) {
        let ext = self
            .buffer
            .path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let root = self
            .buffer
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        if let Some((cmd, args, lang_id)) = crate::mora::lsp::detect_language_server(ext) {
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
                    self.status_message = format!(
                        "LSP: connected to {} ({})",
                        self.lsp_client.server_name, ext
                    );
                }
                Err(e) => {
                    self.status_message = format!("LSP: failed to start {cmd}: {e}");
                }
            }
        } else {
            self.status_message = format!("LSP: no language server configured for .{ext}");
        }
    }

    pub(super) fn run_find_references(&mut self) {
        let word = self.buffer.current_word();
        if word.is_empty() {
            self.status_message = "Find references: no word under cursor".to_string();
            return;
        }
        if self.lsp_client.is_connected() {
            let uri = self
                .buffer
                .path
                .as_ref()
                .map(|p| format!("file://{}", p.display()))
                .unwrap_or_default();
            let line = self.buffer.cursor.row as u32;
            let col = self.buffer.cursor.col as u32;
            match self.lsp_client.references(&uri, line, col) {
                Ok(locs) => {
                    let count = locs.len();
                    let first = locs
                        .first()
                        .map(|l| {
                            format!(
                                "{} L{}",
                                l.uri.rsplit('/').next().unwrap_or(&l.uri),
                                l.range.start.line + 1
                            )
                        })
                        .unwrap_or_default();
                    self.status_message = format!("References: {first} ({count} matches)");
                }
                Err(e) => self.status_message = format!("References: {e}"),
            }
        } else {
            // Fallback to ripgrep
            let output = std::process::Command::new("rg")
                .args([
                    "--line-number",
                    "--no-heading",
                    "--max-count",
                    "50",
                    "-w",
                    &word,
                ])
                .output();
            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    if stdout.trim().is_empty() {
                        self.status_message = format!("References: no references to \"{word}\"");
                    } else {
                        let first = stdout.lines().next().unwrap_or("");
                        self.status_message =
                            format!("References: {first} ({} matches)", stdout.lines().count());
                    }
                }
                Err(e) => self.status_message = format!("References: {e}"),
            }
        }
    }

    pub(super) fn run_hover_doc(&mut self) {
        let word = self.buffer.current_word();
        if word.is_empty() {
            self.status_message = "Hover: no word under cursor".to_string();
            return;
        }
        if self.lsp_client.is_connected() {
            let uri = self
                .buffer
                .path
                .as_ref()
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
            "rs" => (
                "rustc",
                vec![
                    "--edition",
                    "2021",
                    "--crate-type",
                    "lib",
                    "-Z",
                    "parse-only",
                    &filename,
                ],
            ),
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
                let first_error = stderr
                    .lines()
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
    pub(super) fn dired_open(&mut self) {
        let dir = self
            .buffer
            .path
            .as_ref()
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
                        if let Some(num_str) = plus_part.split(|c: char| !c.is_ascii_digit()).next()
                        {
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
            .args([
                "--line-number",
                "--no-heading",
                "--max-count",
                "50",
                pattern,
            ])
            .output();
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if stdout.trim().is_empty() {
                    self.status_message = format!("Grep: no matches for \"{pattern}\"");
                } else {
                    let first = stdout.lines().next().unwrap_or("");
                    self.status_message =
                        format!("Grep: {first} ({} matches)", stdout.lines().count());
                }
            }
            Err(e) => {
                self.status_message = format!("Grep: {e}");
            }
        }
    }

    pub(super) fn run_git_log(&mut self) {
        let output = self.run_git_command(&["log", "--oneline", "-20"]);
        self.status_message = if output.trim().is_empty() {
            "Git: no commits".to_string()
        } else {
            let first = output.lines().next().unwrap_or("");
            format!("Git log: {first} ({} commits)", output.lines().count())
        };
    }

    pub(super) fn run_git_diff(&mut self) {
        let output = self.run_git_command(&["diff", "--stat"]);
        self.status_message = if output.trim().is_empty() {
            "Git: no changes".to_string()
        } else {
            format!("Git diff: {}", output.lines().next().unwrap_or(""))
        };
    }

    pub(super) fn run_git_commit(&mut self) {
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
        if commit_output.contains("nothing to commit") || commit_output.contains("no changes added")
        {
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

    pub(super) fn repeat_last_find(&mut self, same_direction: bool) {
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

    pub(super) fn handle_ace_jump(&mut self, key: KeyEvent) -> KeyAction {
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
}
