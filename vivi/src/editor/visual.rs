use display_protocol::{KeyCode, KeyEvent, SelectionMode};

use crate::editor::{Editor, YankRegister};
use crate::mode::Mode;

impl Editor {
    pub(super) fn handle_visual(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('v') => {
                self.mode = Mode::Normal;
                self.selection = None;
            }
            KeyCode::Char(':') => {
                self.mode = Mode::Command;
                self.command_input = String::from("'<,'>");
                self.command_cursor = self.command_input.len();
            }
            KeyCode::Char('o') => {
                if let Some(ref mut sel) = self.selection {
                    std::mem::swap(&mut sel.anchor_line, &mut sel.head_line);
                    std::mem::swap(&mut sel.anchor_col, &mut sel.head_col);
                    self.cursor.row = sel.head_line as usize;
                    self.cursor.col = sel.head_col as usize;
                }
            }
            KeyCode::Char('d') | KeyCode::Char('x') => {
                self.push_undo_state();
                self.delete_selection();
                self.mode = Mode::Normal;
                self.selection = None;
            }
            KeyCode::Char('y') => {
                self.yank_selection();
                self.mode = Mode::Normal;
                self.selection = None;
            }
            KeyCode::Char('c') | KeyCode::Char('s') => {
                self.push_undo_state();
                self.delete_selection();
                self.in_change_group = true;
                self.mode = Mode::Insert;
                self.selection = None;
            }
            KeyCode::Char('>') => {
                self.push_undo_state();
                self.indent_selection(4);
                self.mode = Mode::Normal;
                self.selection = None;
            }
            KeyCode::Char('<') => {
                self.push_undo_state();
                self.dedent_selection(4);
                self.mode = Mode::Normal;
                self.selection = None;
            }
            KeyCode::Char('U') => {
                self.push_undo_state();
                self.case_selection(true);
                self.mode = Mode::Normal;
                self.selection = None;
            }
            KeyCode::Char('u') => {
                self.push_undo_state();
                self.case_selection(false);
                self.mode = Mode::Normal;
                self.selection = None;
            }
            KeyCode::Char('~') => {
                self.push_undo_state();
                self.toggle_case_selection();
                self.mode = Mode::Normal;
                self.selection = None;
            }
            _ => {
                self.handle_normal(key);
                self.update_selection_head();
                self.clamp_cursor();
            }
        }
    }

    pub(super) fn handle_visual_line(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('V') => {
                self.mode = Mode::Normal;
                self.selection = None;
            }
            KeyCode::Char(':') => {
                self.mode = Mode::Command;
                self.command_input = String::from("'<,'>");
                self.command_cursor = self.command_input.len();
            }
            KeyCode::Char('o') => {
                if let Some(ref mut sel) = self.selection {
                    std::mem::swap(&mut sel.anchor_line, &mut sel.head_line);
                    self.cursor.row = sel.head_line as usize;
                    self.cursor.col = 0;
                }
            }
            KeyCode::Char('d') | KeyCode::Char('x') => {
                self.delete_selection();
                self.mode = Mode::Normal;
                self.selection = None;
            }
            KeyCode::Char('y') => {
                self.yank_selection();
                self.mode = Mode::Normal;
                self.selection = None;
            }
            KeyCode::Char('c') | KeyCode::Char('s') => {
                self.delete_selection();
                self.mode = Mode::Insert;
                self.selection = None;
            }
            KeyCode::Char('>') => {
                self.indent_selection(4);
                self.mode = Mode::Normal;
                self.selection = None;
            }
            KeyCode::Char('<') => {
                self.dedent_selection(4);
                self.mode = Mode::Normal;
                self.selection = None;
            }
            KeyCode::Char('U') => {
                self.case_selection(true);
                self.mode = Mode::Normal;
                self.selection = None;
            }
            KeyCode::Char('u') => {
                self.case_selection(false);
                self.mode = Mode::Normal;
                self.selection = None;
            }
            KeyCode::Char('~') => {
                self.toggle_case_selection();
                self.mode = Mode::Normal;
                self.selection = None;
            }
            KeyCode::Char('J') => {
                let n = self.take_count();
                for _ in 0..n {
                    self.buffer.join_lines(self.cursor.row);
                }
                self.update_selection_head();
            }
            _ => {
                self.handle_normal(key);
                if let Some(ref mut sel) = self.selection {
                    sel.head_line = self.cursor.row as u32;
                    sel.head_col = self.buffer.line_len(self.cursor.row) as u32;
                }
            }
        }
    }

    fn update_selection_head(&mut self) {
        if let Some(ref mut sel) = self.selection {
            sel.head_line = self.cursor.row as u32;
            sel.head_col = self.cursor.col as u32;
        }
    }

    fn delete_selection(&mut self) {
        let Some(ref sel) = self.selection else {
            return;
        };
        let (start_line, start_col, end_line, end_col) = sel.normalized_range();

        match sel.mode {
            SelectionMode::Line => {
                let count = (end_line - start_line + 1) as usize;
                let mut lines = Vec::new();
                for _ in 0..count {
                    if let Some(line) = self.buffer.delete_line(start_line as usize) {
                        lines.push(line);
                    }
                }
                self.yank_register = YankRegister::Lines(lines);
                self.cursor.row = start_line as usize;
                self.cursor.col = 0;
            }
            SelectionMode::Char => {
                let end = (end_col as usize + 1).min(self.buffer.line_len(end_line as usize));
                if start_line == end_line {
                    let deleted =
                        self.buffer.line(start_line as usize)[start_col as usize..end].to_string();
                    self.buffer
                        .replace_range(start_line as usize, start_col as usize, end, "");
                    self.yank_register = YankRegister::Chars(deleted);
                } else {
                    let last_end =
                        (end_col as usize + 1).min(self.buffer.line_len(end_line as usize));
                    let first_part =
                        self.buffer.line(start_line as usize)[start_col as usize..].to_string();
                    let last_part = self.buffer.line(end_line as usize)[..last_end].to_string();
                    self.buffer.replace_range(
                        start_line as usize,
                        start_col as usize,
                        self.buffer.line_len(start_line as usize),
                        "",
                    );
                    let middle_count = (end_line - start_line) as usize;
                    for _ in 0..middle_count {
                        self.buffer.delete_line(start_line as usize + 1);
                    }
                    if start_line as usize + 1 < self.buffer.line_count() {
                        self.buffer
                            .replace_range(start_line as usize + 1, 0, last_end, "");
                        self.buffer.join_lines(start_line as usize);
                    }
                    self.yank_register =
                        YankRegister::Chars(format!("{}\n{}", first_part, last_part));
                }
                self.cursor.row = start_line as usize;
                self.cursor.col = start_col as usize;
            }
            SelectionMode::Block => {
                let min_col = start_col.min(end_col) as usize;
                let max_col = (start_col.max(end_col) as usize + 1).min(
                    (start_line..=end_line)
                        .map(|line| self.buffer.line_len(line as usize))
                        .max()
                        .unwrap_or(0),
                );
                for line_idx in start_line..=end_line {
                    let line_idx = line_idx as usize;
                    let len = self.buffer.line_len(line_idx);
                    if min_col < len {
                        self.buffer
                            .replace_range(line_idx, min_col, max_col.min(len), "");
                    }
                }
                self.cursor.row = start_line as usize;
                self.cursor.col = min_col;
            }
        }
    }

    fn yank_selection(&mut self) {
        let Some(ref sel) = self.selection else {
            return;
        };
        let (start_line, start_col, end_line, end_col) = sel.normalized_range();

        match sel.mode {
            SelectionMode::Line => {
                let count = (end_line - start_line + 1) as usize;
                let mut lines = Vec::new();
                for i in 0..count {
                    let idx = start_line as usize + i;
                    if idx < self.buffer.line_count() {
                        lines.push(self.buffer.line(idx).to_string());
                    }
                }
                self.yank_register = YankRegister::Lines(lines);
                self.message = Some(format!("{} line(s) yanked", count));
            }
            SelectionMode::Char => {
                let end = (end_col as usize + 1).min(self.buffer.line_len(end_line as usize));
                if start_line == end_line {
                    let text =
                        self.buffer.line(start_line as usize)[start_col as usize..end].to_string();
                    self.yank_register = YankRegister::Chars(text);
                } else {
                    let mut parts = Vec::new();
                    parts.push(
                        self.buffer.line(start_line as usize)[start_col as usize..].to_string(),
                    );
                    for i in (start_line + 1)..end_line {
                        parts.push(self.buffer.line(i as usize).to_string());
                    }
                    parts.push(self.buffer.line(end_line as usize)[..end].to_string());
                    self.yank_register = YankRegister::Chars(parts.join("\n"));
                }
                self.message = Some("Selection yanked".into());
            }
            SelectionMode::Block => {
                let min_col = start_col.min(end_col) as usize;
                let max_col = (start_col.max(end_col) as usize + 1).min(self.buffer.line_count());
                let mut parts = Vec::new();
                for line_idx in start_line..=end_line {
                    let line = self.buffer.line(line_idx as usize);
                    let end = max_col.min(line.len());
                    if min_col < line.len() {
                        parts.push(line[min_col..end].to_string());
                    } else {
                        parts.push(String::new());
                    }
                }
                self.yank_register = YankRegister::Chars(parts.join("\n"));
                self.message = Some("Block selection yanked".into());
            }
        }
    }
}
