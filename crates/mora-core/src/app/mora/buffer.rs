use std::collections::VecDeque;
use std::path::PathBuf;

const MAX_UNDO: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
struct Snapshot {
    lines: Vec<String>,
    cursor: Cursor,
}

#[derive(Debug)]
pub struct Buffer {
    pub lines: Vec<String>,
    pub cursor: Cursor,
    pub scroll_offset: usize,
    pub modified: bool,
    pub path: Option<PathBuf>,
    undo_stack: VecDeque<Snapshot>,
    redo_stack: Vec<Snapshot>,
    pub narrow_start: Option<usize>,
    pub narrow_end: Option<usize>,
    pub fold_level: Option<usize>,
    pub folded_lines: Vec<bool>,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor: Cursor { row: 0, col: 0 },
            scroll_offset: 0,
            modified: false,
            path: None,
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            narrow_start: None,
            narrow_end: None,
            fold_level: None,
            folded_lines: Vec::new(),
        }
    }

    pub fn from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(String::from).collect()
        };
        Ok(Self {
            lines,
            cursor: Cursor { row: 0, col: 0 },
            scroll_offset: 0,
            modified: false,
            path: Some(path.to_path_buf()),
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            narrow_start: None,
            narrow_end: None,
            fold_level: None,
            folded_lines: Vec::new(),
        })
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(ref path) = self.path {
            std::fs::write(path, self.content())?;
            self.modified = false;
        }
        Ok(())
    }

    pub fn save_as(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(path, self.content())?;
        self.path = Some(path.to_path_buf());
        self.modified = false;
        Ok(())
    }

    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn current_line(&self) -> &str {
        &self.lines[self.cursor.row]
    }

    pub fn line(&self, idx: usize) -> &str {
        &self.lines[idx]
    }

    pub fn filename(&self) -> &str {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[No Name]")
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            lines: self.lines.clone(),
            cursor: self.cursor.clone(),
        }
    }

    fn push_undo(&mut self) {
        self.undo_stack.push_back(self.snapshot());
        if self.undo_stack.len() > MAX_UNDO {
            self.undo_stack.pop_front();
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(snap) = self.undo_stack.pop_back() {
            self.redo_stack.push(self.snapshot());
            self.lines = snap.lines;
            self.cursor = snap.cursor;
            self.clamp_cursor();
        }
    }

    pub fn redo(&mut self) {
        if let Some(snap) = self.redo_stack.pop() {
            self.undo_stack.push_back(self.snapshot());
            self.lines = snap.lines;
            self.cursor = snap.cursor;
            self.clamp_cursor();
        }
    }

    fn clamp_cursor(&mut self) {
        self.cursor.row = self.cursor.row.min(self.lines.len().saturating_sub(1));
        self.cursor.col = self.cursor.col.min(self.lines[self.cursor.row].len());
    }

    fn col_byte_index(line: &str, char_idx: usize) -> usize {
        line.char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(line.len())
    }

    fn char_index_at_col(line: &str, col: usize) -> usize {
        line[..col].chars().count()
    }

    pub fn insert_char(&mut self, ch: char) {
        self.push_undo();
        let line = &mut self.lines[self.cursor.row];
        line.insert(self.cursor.col, ch);
        self.cursor.col += ch.len_utf8();
        self.modified = true;
    }

    pub fn insert_string(&mut self, s: &str) {
        self.push_undo();
        for ch in s.chars() {
            if ch == '\n' {
                self.insert_newline_no_undo();
            } else {
                let line = &mut self.lines[self.cursor.row];
                line.insert(self.cursor.col, ch);
                self.cursor.col += ch.len_utf8();
            }
        }
        self.modified = true;
    }

    pub fn insert_newline(&mut self) {
        self.push_undo();
        self.insert_newline_no_undo();
        self.modified = true;
    }

    fn insert_newline_no_undo(&mut self) {
        let rest = self.lines[self.cursor.row][self.cursor.col..].to_string();
        self.lines[self.cursor.row].truncate(self.cursor.col);
        self.cursor.row += 1;
        self.lines.insert(self.cursor.row, rest);
        self.cursor.col = 0;
    }

    pub fn delete_backward(&mut self) {
        if self.cursor.col > 0 {
            self.push_undo();
            let line = &mut self.lines[self.cursor.row];
            let prev_len = line[..self.cursor.col]
                .chars()
                .next_back()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.cursor.col -= prev_len;
            line.remove(self.cursor.col);
            self.modified = true;
        } else if self.cursor.row > 0 {
            self.push_undo();
            let current = self.lines.remove(self.cursor.row);
            self.cursor.row -= 1;
            self.cursor.col = self.lines[self.cursor.row].len();
            self.lines[self.cursor.row].push_str(&current);
            self.modified = true;
        }
    }

    pub fn delete_forward(&mut self) {
        let line_len = self.lines[self.cursor.row].len();
        if self.cursor.col < line_len {
            self.push_undo();
            self.lines[self.cursor.row].remove(self.cursor.col);
            self.modified = true;
        } else if self.cursor.row + 1 < self.lines.len() {
            self.push_undo();
            let next = self.lines.remove(self.cursor.row + 1);
            self.lines[self.cursor.row].push_str(&next);
            self.modified = true;
        }
    }

    pub fn delete_line(&mut self) {
        self.push_undo();
        if self.lines.len() == 1 {
            self.lines[0].clear();
            self.cursor.col = 0;
        } else {
            self.lines.remove(self.cursor.row);
            if self.cursor.row >= self.lines.len() {
                self.cursor.row = self.lines.len() - 1;
            }
            self.cursor.col = self.cursor.col.min(self.lines[self.cursor.row].len());
        }
        self.modified = true;
    }

    pub fn delete_to_eol(&mut self) {
        self.push_undo();
        let line = &mut self.lines[self.cursor.row];
        line.truncate(self.cursor.col);
        self.modified = true;
    }

    pub fn move_left(&mut self) {
        if self.cursor.col > 0 {
            let line = &self.lines[self.cursor.row];
            let prev_len = line[..self.cursor.col]
                .chars()
                .next_back()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.cursor.col -= prev_len;
        } else if self.cursor.row > 0 {
            self.cursor.row -= 1;
            self.cursor.col = self.lines[self.cursor.row].len();
        }
    }

    pub fn move_right(&mut self) {
        let line_len = self.lines[self.cursor.row].len();
        if self.cursor.col < line_len {
            let line = &self.lines[self.cursor.row];
            let next_len = line[self.cursor.col..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.cursor.col += next_len;
        } else if self.cursor.row + 1 < self.lines.len() {
            self.cursor.row += 1;
            self.cursor.col = 0;
        }
    }

    pub fn move_up(&mut self) {
        let min_row = self.narrow_start.unwrap_or(0);
        let mut prev = self.cursor.row;
        if prev > min_row {
            prev -= 1;
            while prev > min_row && self.is_line_folded(prev) {
                prev -= 1;
            }
            if !self.is_line_folded(prev) {
                self.cursor.row = prev;
                self.cursor.col = self.cursor.col.min(self.lines[self.cursor.row].len());
            }
        }
    }

    pub fn move_down(&mut self) {
        let max_row = self.narrow_end.unwrap_or(self.lines.len().saturating_sub(1));
        let mut next = self.cursor.row + 1;
        while next <= max_row && self.is_line_folded(next) {
            next += 1;
        }
        if next <= max_row {
            self.cursor.row = next;
            self.cursor.col = self.cursor.col.min(self.lines[self.cursor.row].len());
        }
    }

    pub fn move_to_line_start(&mut self) {
        self.cursor.col = 0;
    }

    pub fn move_to_line_end(&mut self) {
        self.cursor.col = self.lines[self.cursor.row].len();
    }

    pub fn move_to_file_start(&mut self) {
        self.cursor.row = self.narrow_start.unwrap_or(0);
        self.cursor.col = 0;
    }

    pub fn move_to_file_end(&mut self) {
        self.cursor.row = self.narrow_end.unwrap_or(self.lines.len().saturating_sub(1));
        self.cursor.col = self.lines[self.cursor.row].len();
    }

    pub fn move_to_line(&mut self, line_num: usize) {
        self.cursor.row = line_num.min(self.lines.len().saturating_sub(1));
        self.cursor.col = 0;
    }

    pub fn move_word_forward(&mut self) {
        let line = &self.lines[self.cursor.row];
        let chars: Vec<char> = line.chars().collect();
        let char_idx = Self::char_index_at_col(line, self.cursor.col);
        let mut i = char_idx;
        let len = chars.len();

        while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
            i += 1;
        }
        while i < len && !chars[i].is_alphanumeric() && chars[i] != '_' {
            i += 1;
        }

        if i >= len && self.cursor.row + 1 < self.lines.len() {
            self.cursor.row += 1;
            self.cursor.col = 0;
        } else {
            self.cursor.col = Self::col_byte_index(line, i);
        }
    }

    pub fn move_word_end(&mut self) {
        let line = self.lines[self.cursor.row].clone();
        let chars: Vec<char> = line.chars().collect();
        let char_idx = Self::char_index_at_col(&line, self.cursor.col);
        let len = chars.len();
        if len == 0 || char_idx >= len {
            if self.cursor.row + 1 < self.lines.len() {
                self.cursor.row += 1;
                self.cursor.col = 0;
                self.move_word_end();
            }
            return;
        }

        let mut i = char_idx;
        while i < len && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= len {
            if self.cursor.row + 1 < self.lines.len() {
                self.cursor.row += 1;
                self.cursor.col = 0;
                self.move_word_end();
            }
            return;
        }

        let is_word = chars[i].is_alphanumeric() || chars[i] == '_';
        while i < len {
            let c = chars[i];
            if c.is_whitespace() || is_word != (c.is_alphanumeric() || c == '_') {
                break;
            }
            i += 1;
        }

        let last = i.saturating_sub(1);
        self.cursor.col = Self::col_byte_index(&line, last);
    }

    pub fn move_word_backward(&mut self) {
        let line = &self.lines[self.cursor.row];
        let chars: Vec<char> = line.chars().collect();
        let char_idx = Self::char_index_at_col(line, self.cursor.col);
        let mut i = char_idx;

        if i == 0 {
            if self.cursor.row > 0 {
                self.cursor.row -= 1;
                self.cursor.col = self.lines[self.cursor.row].len();
            }
            return;
        }

        i -= 1;
        while i > 0 && !chars[i].is_alphanumeric() && chars[i] != '_' {
            i -= 1;
        }
        while i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_') {
            i -= 1;
        }

        self.cursor.col = Self::col_byte_index(line, i);
    }

    pub fn goto_line(&mut self, line_num: usize) {
        self.cursor.row = (line_num - 1).min(self.lines.len().saturating_sub(1));
        self.cursor.col = 0;
    }

    pub fn find_forward(&mut self, query: &str) -> bool {
        let start_row = self.cursor.row;
        let start_col = self.cursor.col + 1;

        for row_off in 0..self.lines.len() {
            let row = (start_row + row_off) % self.lines.len();
            let search_col = if row_off == 0 { start_col } else { 0 };
            if let Some(idx) = self.lines[row][search_col..].find(query) {
                self.cursor.row = row;
                self.cursor.col = search_col + idx;
                return true;
            }
        }
        false
    }

    pub fn find_backward(&mut self, query: &str) -> bool {
        let start_row = self.cursor.row;
        for row_off in 0..self.lines.len() {
            let row = (start_row + self.lines.len() - row_off) % self.lines.len();
            let search_end = if row_off == 0 {
                self.cursor.col
            } else {
                self.lines[row].len()
            };
            if let Some(idx) = self.lines[row][..search_end].rfind(query) {
                self.cursor.row = row;
                self.cursor.col = idx;
                return true;
            }
        }
        false
    }

    pub fn replace_first(&mut self, old: &str, new: &str) -> bool {
        let line = &self.lines[self.cursor.row];
        if let Some(idx) = line[self.cursor.col..].find(old) {
            self.push_undo();
            let abs_idx = self.cursor.col + idx;
            self.lines[self.cursor.row].replace_range(abs_idx..abs_idx + old.len(), new);
            self.modified = true;
            return true;
        }
        for row in (self.cursor.row + 1)..self.lines.len() {
            if let Some(idx) = self.lines[row].find(old) {
                self.push_undo();
                self.lines[row].replace_range(idx..idx + old.len(), new);
                self.cursor.row = row;
                self.cursor.col = idx;
                self.modified = true;
                return true;
            }
        }
        false
    }

    pub fn push_undo_snapshot(&mut self) {
        self.push_undo();
    }

    pub fn replace_all(&mut self, old: &str, new: &str) -> usize {
        self.push_undo();
        let mut count = 0;
        for line in &mut self.lines {
            while let Some(idx) = line.find(old) {
                line.replace_range(idx..idx + old.len(), new);
                count += 1;
            }
        }
        if count > 0 {
            self.modified = true;
            self.clamp_cursor();
        }
        count
    }

    pub fn transpose_char(&mut self) {
        let row = self.cursor.row;
        let col = self.cursor.col;
        let line = &self.lines[row];
        let chars: Vec<char> = line.chars().collect();
        let char_idx = Self::char_index_at_col(line, col);

        if chars.len() < 2 || char_idx == 0 {
            return;
        }

        let swap_idx = if char_idx >= chars.len() {
            chars.len() - 2
        } else {
            char_idx - 1
        };

        if swap_idx + 1 >= chars.len() {
            return;
        }

        self.push_undo();
        let mut new_chars = chars.clone();
        new_chars.swap(swap_idx, swap_idx + 1);
        self.lines[row] = new_chars.into_iter().collect();
        self.cursor.col = Self::col_byte_index(&self.lines[row], swap_idx + 2);
        self.modified = true;
    }

    pub fn transpose_word(&mut self) {
        let row = self.cursor.row;
        let line = self.lines[row].clone();
        let chars: Vec<char> = line.chars().collect();
        let len = chars.len();
        let col = self.cursor.col;
        let char_idx = Self::char_index_at_col(&line, col);

        let is_word = |c: char| c.is_alphanumeric() || c == '_';

        let mut word2_end = char_idx;
        while word2_end < len && !is_word(chars[word2_end]) {
            word2_end += 1;
        }
        while word2_end < len && is_word(chars[word2_end]) {
            word2_end += 1;
        }
        let word2_start = {
            let mut s = word2_end;
            while s > 0 && is_word(chars[s - 1]) {
                s -= 1;
            }
            s
        };

        let mut word1_end = word2_start;
        while word1_end > 0 && !is_word(chars[word1_end - 1]) {
            word1_end -= 1;
        }
        let mut word1_start = word1_end;
        while word1_start > 0 && is_word(chars[word1_start - 1]) {
            word1_start -= 1;
        }

        if word1_start >= word1_end || word2_start >= word2_end || word1_end > word2_start {
            return;
        }

        self.push_undo();
        let word1: String = chars[word1_start..word1_end].iter().collect();
        let sep: String = chars[word1_end..word2_start].iter().collect();
        let word2: String = chars[word2_start..word2_end].iter().collect();
        let rest: String = chars[word2_end..].iter().collect();
        let prefix: String = chars[..word1_start].iter().collect();

        self.lines[row] = format!("{}{}{}{}{}", prefix, word2, sep, word1, rest);
        self.cursor.col = Self::col_byte_index(&self.lines[row], word2_end);
        self.modified = true;
    }

    pub fn transpose_line(&mut self) {
        let row = self.cursor.row;
        if row == 0 {
            return;
        }
        self.push_undo();
        let line = self.lines.remove(row);
        self.lines.insert(row - 1, line);
        self.cursor.row = row - 1;
        self.cursor.col = 0;
        self.modified = true;
    }

    pub fn capitalize_word(&mut self) {
        let row = self.cursor.row;
        let col = self.cursor.col;
        let line = &self.lines[row];
        let chars: Vec<char> = line.chars().collect();
        let len = chars.len();
        let mut i = Self::char_index_at_col(line, col);

        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';

        if i < len && is_word_char(chars[i]) {
            if i > 0 && is_word_char(chars[i - 1]) {
                while i < len && is_word_char(chars[i]) {
                    i += 1;
                }
            }
        }
        while i < len && !is_word_char(chars[i]) {
            i += 1;
        }
        if i >= len {
            return;
        }

        self.push_undo();
        let start = i;
        while i < len && is_word_char(chars[i]) {
            i += 1;
        }

        let mut new_chars = chars.clone();
        if start < new_chars.len() {
            new_chars[start] = new_chars[start].to_uppercase().next().unwrap_or(new_chars[start]);
        }
        for j in (start + 1)..i {
            if j < new_chars.len() {
                new_chars[j] = new_chars[j].to_lowercase().next().unwrap_or(new_chars[j]);
            }
        }
        self.lines[row] = new_chars.into_iter().collect();
        self.cursor.col = Self::col_byte_index(&self.lines[row], i);
        self.modified = true;
    }

    pub fn uppercase_word(&mut self) {
        let row = self.cursor.row;
        let col = self.cursor.col;
        let line = &self.lines[row];
        let chars: Vec<char> = line.chars().collect();
        let len = chars.len();
        let mut i = Self::char_index_at_col(line, col);

        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';

        if i < len && is_word_char(chars[i]) {
            if i > 0 && is_word_char(chars[i - 1]) {
                while i < len && is_word_char(chars[i]) {
                    i += 1;
                }
            }
        }
        while i < len && !is_word_char(chars[i]) {
            i += 1;
        }
        if i >= len {
            return;
        }

        self.push_undo();
        let start = i;
        while i < len && is_word_char(chars[i]) {
            i += 1;
        }

        let mut new_chars = chars.clone();
        for j in start..i {
            if j < new_chars.len() {
                new_chars[j] = new_chars[j].to_uppercase().next().unwrap_or(new_chars[j]);
            }
        }
        self.lines[row] = new_chars.into_iter().collect();
        self.cursor.col = Self::col_byte_index(&self.lines[row], i);
        self.modified = true;
    }

    pub fn lowercase_word(&mut self) {
        let row = self.cursor.row;
        let col = self.cursor.col;
        let line = &self.lines[row];
        let chars: Vec<char> = line.chars().collect();
        let len = chars.len();
        let mut i = Self::char_index_at_col(line, col);

        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';

        if i < len && is_word_char(chars[i]) {
            if i > 0 && is_word_char(chars[i - 1]) {
                while i < len && is_word_char(chars[i]) {
                    i += 1;
                }
            }
        }
        while i < len && !is_word_char(chars[i]) {
            i += 1;
        }
        if i >= len {
            return;
        }

        self.push_undo();
        let start = i;
        while i < len && is_word_char(chars[i]) {
            i += 1;
        }

        let mut new_chars = chars.clone();
        for j in start..i {
            if j < new_chars.len() {
                new_chars[j] = new_chars[j].to_lowercase().next().unwrap_or(new_chars[j]);
            }
        }
        self.lines[row] = new_chars.into_iter().collect();
        self.cursor.col = Self::col_byte_index(&self.lines[row], i);
        self.modified = true;
    }

    pub fn uppercase_region(&mut self, mark: (usize, usize)) {
        let (sr, sc, er, ec) = if (mark.0, mark.1) <= (self.cursor.row, self.cursor.col) {
            (mark.0, mark.1, self.cursor.row, self.cursor.col)
        } else {
            (self.cursor.row, self.cursor.col, mark.0, mark.1)
        };
        self.push_undo();
        for row in sr..=er {
            let line_chars: Vec<char> = self.lines[row].chars().collect();
            let line_len = line_chars.len();
            let col_start = if row == sr { sc } else { 0 };
            let col_end = if row == er { ec.min(line_len) } else { line_len };
            let mut new_chars = line_chars;
            for j in col_start..col_end {
                if j < new_chars.len() {
                    new_chars[j] = new_chars[j].to_uppercase().next().unwrap_or(new_chars[j]);
                }
            }
            self.lines[row] = new_chars.into_iter().collect();
        }
        self.modified = true;
    }

    pub fn lowercase_region(&mut self, mark: (usize, usize)) {
        let (sr, sc, er, ec) = if (mark.0, mark.1) <= (self.cursor.row, self.cursor.col) {
            (mark.0, mark.1, self.cursor.row, self.cursor.col)
        } else {
            (self.cursor.row, self.cursor.col, mark.0, mark.1)
        };
        self.push_undo();
        for row in sr..=er {
            let line_chars: Vec<char> = self.lines[row].chars().collect();
            let line_len = line_chars.len();
            let col_start = if row == sr { sc } else { 0 };
            let col_end = if row == er { ec.min(line_len) } else { line_len };
            let mut new_chars = line_chars;
            for j in col_start..col_end {
                if j < new_chars.len() {
                    new_chars[j] = new_chars[j].to_lowercase().next().unwrap_or(new_chars[j]);
                }
            }
            self.lines[row] = new_chars.into_iter().collect();
        }
        self.modified = true;
    }

    pub fn hungry_delete_forward(&mut self) {
        let line = &self.lines[self.cursor.row];
        let chars: Vec<char> = line.chars().collect();
        let col = self.cursor.col;
        let mut end = col;
        while end < chars.len() && chars[end].is_whitespace() {
            end += 1;
        }
        if end == col && end < chars.len() {
            end += 1;
        }
        if end > col {
            self.push_undo();
            let new_line: String = chars[..col].iter().chain(chars[end..].iter()).collect();
            self.lines[self.cursor.row] = new_line;
            self.modified = true;
        }
    }

    pub fn replace_range(&mut self, start: usize, end: usize, replacement: &str) {
        self.push_undo();
        let chars: Vec<char> = self.lines[self.cursor.row].chars().collect();
        let new_line: String = chars[..start].iter()
            .copied()
            .chain(replacement.chars())
            .chain(chars[end..].iter().copied())
            .collect();
        self.lines[self.cursor.row] = new_line;
        self.cursor.col = start + replacement.chars().count();
        self.modified = true;
    }

    pub fn delete_range(&mut self, start: usize, end: usize) {
        self.push_undo();
        let chars: Vec<char> = self.lines[self.cursor.row].chars().collect();
        let new_line: String = chars[..start].iter()
            .copied()
            .chain(chars[end..].iter().copied())
            .collect();
        self.lines[self.cursor.row] = new_line;
        self.cursor.col = start;
        self.modified = true;
    }

    pub fn insert_empty_line_below(&mut self) {
        self.push_undo();
        let row = self.cursor.row;
        self.lines.insert(row + 1, String::new());
        self.cursor.row = row + 1;
        self.cursor.col = 0;
        self.modified = true;
    }

    pub fn insert_empty_line_above(&mut self) {
        self.push_undo();
        let row = self.cursor.row;
        self.lines.insert(row, String::new());
        self.cursor.row = row;
        self.cursor.col = 0;
        self.modified = true;
    }

    pub fn cleanup_buffer(&mut self) {
        self.push_undo();
        for line in &mut self.lines {
            let trimmed = line.trim_end();
            if trimmed.len() != line.len() {
                *line = trimmed.to_string();
            }
        }
        self.modified = true;
    }

    pub fn copy_and_comment(&mut self) {
        self.push_undo();
        let row = self.cursor.row;
        let line = self.lines[row].clone();
        let commented = format!("// {}", line);
        self.lines[row] = commented;
        self.lines.insert(row + 1, line);
        self.cursor.row = row + 1;
        self.modified = true;
    }

    pub fn narrow_to_region(&mut self, start_row: usize, end_row: usize) {
        let start = start_row.min(end_row);
        let end = start_row.max(end_row).min(self.lines.len().saturating_sub(1));
        if start <= end && end < self.lines.len() {
            self.narrow_start = Some(start);
            self.narrow_end = Some(end);
            self.cursor.row = start;
            self.cursor.col = 0;
            self.scroll_offset = 0;
        }
    }

    pub fn widen(&mut self) {
        self.narrow_start = None;
        self.narrow_end = None;
    }

    pub fn is_narrowed(&self) -> bool {
        self.narrow_start.is_some()
    }

    pub fn visible_line_count(&self) -> usize {
        match (self.narrow_start, self.narrow_end) {
            (Some(start), Some(end)) => end - start + 1,
            _ => self.lines.len(),
        }
    }

    pub fn narrow_offset(&self) -> usize {
        self.narrow_start.unwrap_or(0)
    }

    pub fn dos2unix(&mut self) {
        for line in &mut self.lines {
            if line.ends_with('\r') {
                line.pop();
            }
        }
        self.modified = true;
    }

    pub fn unix2dos(&mut self) {
        for line in &mut self.lines {
            line.push('\r');
        }
        self.modified = true;
    }

    pub fn toggle_fold(&mut self) {
        if self.fold_level.is_some() {
            self.fold_level = None;
            self.folded_lines.clear();
        } else {
            let row = self.cursor.row;
            let indent = self.lines[row].chars().take_while(|c| *c == ' ' || *c == '\t').count();
            self.fold_level = Some(indent);
            self.folded_lines = self.lines.iter().map(|line| {
                let line_indent = line.chars().take_while(|c| *c == ' ' || *c == '\t').count();
                line_indent > indent && !line.trim().is_empty()
            }).collect();
        }
    }

    pub fn is_line_folded(&self, row: usize) -> bool {
        self.fold_level.is_some() && row < self.folded_lines.len() && self.folded_lines[row]
    }

    pub fn is_folded(&self) -> bool {
        self.fold_level.is_some()
    }

    pub fn word_under_cursor(&self) -> String {
        let line = self.current_line();
        let col = self.cursor.col;
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() || col >= chars.len() {
            return String::new();
        }
        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
        if !is_word_char(chars[col]) {
            return String::new();
        }
        let mut start = col;
        while start > 0 && is_word_char(chars[start - 1]) {
            start -= 1;
        }
        let mut end = col;
        while end < chars.len() && is_word_char(chars[end]) {
            end += 1;
        }
        chars[start..end].iter().collect()
    }

    pub fn search_forward_from(&self, pattern: &str, from_row: usize, from_col: usize) -> Option<(usize, usize)> {
        if pattern.is_empty() {
            return None;
        }
        for row in from_row..self.lines.len() {
            let line = &self.lines[row];
            let start_col = if row == from_row { from_col } else { 0 };
            if start_col < line.len() {
                if let Some(pos) = line[start_col..].find(pattern) {
                    return Some((row, start_col + pos));
                }
            }
        }
        None
    }

    pub fn search_backward_from(&self, pattern: &str, from_row: usize, from_col: usize) -> Option<(usize, usize)> {
        if pattern.is_empty() {
            return None;
        }
        for row in (0..=from_row).rev() {
            let line = &self.lines[row];
            let end_col = if row == from_row { from_col.min(line.len()) } else { line.len() };
            if let Some(pos) = line[..end_col].rfind(pattern) {
                return Some((row, pos));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_cursor() {
        let mut buf = Buffer::new();
        buf.insert_char('h');
        buf.insert_char('i');
        assert_eq!(buf.content(), "hi");
        assert_eq!(buf.cursor.col, 2);
    }

    #[test]
    fn test_newline() {
        let mut buf = Buffer::new();
        buf.insert_char('a');
        buf.insert_newline();
        buf.insert_char('b');
        assert_eq!(buf.content(), "a\nb");
        assert_eq!(buf.line_count(), 2);
    }

    #[test]
    fn test_delete_backward_merge() {
        let mut buf = Buffer::new();
        buf.insert_string("ab\ncd");
        buf.cursor.row = 1;
        buf.cursor.col = 0;
        buf.delete_backward();
        assert_eq!(buf.content(), "abcd");
    }

    #[test]
    fn test_word_movement() {
        let mut buf = Buffer::new();
        buf.insert_string("hello world foo");
        buf.cursor.col = 0;
        buf.move_word_forward();
        assert_eq!(buf.cursor.col, 6);
        buf.move_word_forward();
        assert_eq!(buf.cursor.col, 12);
    }

    #[test]
    fn test_find_forward() {
        let mut buf = Buffer::new();
        buf.insert_string("hello\nworld\nhello again");
        buf.cursor = Cursor { row: 0, col: 0 };
        assert!(buf.find_forward("world"));
        assert_eq!(buf.cursor.row, 1);
        assert!(buf.find_forward("hello"));
        assert_eq!(buf.cursor.row, 2);
    }

    #[test]
    fn test_replace_all() {
        let mut buf = Buffer::new();
        buf.insert_string("foo bar foo baz foo");
        let count = buf.replace_all("foo", "qux");
        assert_eq!(count, 3);
        assert_eq!(buf.content(), "qux bar qux baz qux");
    }

    #[test]
    fn test_undo_redo() {
        let mut buf = Buffer::new();
        buf.insert_string("hello");
        buf.undo();
        assert_eq!(buf.content(), "");
        buf.redo();
        assert_eq!(buf.content(), "hello");
    }

    #[test]
    fn test_delete_line() {
        let mut buf = Buffer::new();
        buf.insert_string("line1\nline2\nline3");
        buf.cursor.row = 1;
        buf.delete_line();
        assert_eq!(buf.content(), "line1\nline3");
    }

    #[test]
    fn test_goto_line() {
        let mut buf = Buffer::new();
        buf.insert_string("a\nb\nc\nd\ne");
        buf.goto_line(3);
        assert_eq!(buf.cursor.row, 2);
    }
}
