use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A text buffer holding a document's content as lines.
pub struct Buffer {
    lines: Vec<String>,
    path: Option<PathBuf>,
    modified: bool,
}

impl Buffer {
    /// Create an empty buffer.
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            path: None,
            modified: false,
        }
    }

    /// Load a buffer from a file path.
    pub fn from_file(path: &Path) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(String::from).collect()
        };
        // Preserve trailing newline semantics: if content ended with \n,
        // lines() already handled it correctly (yields empty trailing element
        // only if there's content after the last \n, which lines() omits).
        // We always want at least one line.
        let lines = if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        };
        Ok(Self {
            lines,
            path: Some(path.to_path_buf()),
            modified: false,
        })
    }

    /// Save the buffer to its path. Returns error if no path is set.
    pub fn save(&mut self) -> io::Result<()> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "No file name"))?;
        let content = self.lines.join("\n");
        fs::write(path, content)?;
        self.modified = false;
        Ok(())
    }

    /// Save the buffer to a specific path.
    pub fn save_as(&mut self, path: &Path) -> io::Result<()> {
        self.path = Some(path.to_path_buf());
        self.save()
    }

    /// Get the file path, if any.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Display name for the status bar.
    pub fn display_name(&self) -> String {
        match &self.path {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "[No Name]".into()),
            None => "[No Name]".into(),
        }
    }

    /// Whether the buffer has unsaved changes.
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Number of lines in the buffer.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Get a line by index. Returns empty string for out-of-bounds.
    pub fn line(&self, idx: usize) -> &str {
        self.lines.get(idx).map(|s| s.as_str()).unwrap_or("")
    }

    /// Get the length of a line.
    pub fn line_len(&self, idx: usize) -> usize {
        self.lines.get(idx).map(|s| s.len()).unwrap_or(0)
    }

    /// Return a copy of all lines (for undo snapshots).
    pub fn all_lines(&self) -> Vec<String> {
        self.lines.clone()
    }

    /// Restore all lines from a snapshot.
    pub fn restore_lines(&mut self, lines: Vec<String>) {
        self.lines = lines;
    }

    /// Insert a character at (row, col).
    pub fn insert_char(&mut self, row: usize, col: usize, ch: char) {
        if let Some(line) = self.lines.get_mut(row) {
            let col = col.min(line.len());
            line.insert(col, ch);
            self.modified = true;
        }
    }

    /// Insert a string at (row, col).
    pub fn insert_str(&mut self, row: usize, col: usize, s: &str) {
        if s.is_empty() {
            return;
        }
        if let Some(line) = self.lines.get_mut(row) {
            let col = col.min(line.len());
            line.insert_str(col, s);
            self.modified = true;
        }
    }

    /// Delete a character at (row, col). Returns the deleted char.
    pub fn delete_char(&mut self, row: usize, col: usize) -> Option<char> {
        if let Some(line) = self.lines.get_mut(row) {
            if col < line.len() {
                self.modified = true;
                return Some(line.remove(col));
            }
        }
        None
    }

    /// Join line `row` with line `row + 1`. Returns the column where the
    /// join happened (length of line `row` before joining).
    pub fn join_lines(&mut self, row: usize) -> Option<usize> {
        if row + 1 >= self.lines.len() {
            return None;
        }
        let next = self.lines.remove(row + 1);
        let col = self.lines[row].len();
        self.lines[row].push_str(&next);
        self.modified = true;
        Some(col)
    }

    /// Split line at (row, col). Text at and after col moves to a new line
    /// at row + 1. Returns the new line's index.
    pub fn split_line(&mut self, row: usize, col: usize) -> usize {
        if let Some(line) = self.lines.get_mut(row) {
            let col = col.min(line.len());
            let rest = line.split_off(col);
            self.lines.insert(row + 1, rest);
            self.modified = true;
            row + 1
        } else {
            self.lines.push(String::new());
            self.modified = true;
            self.lines.len() - 1
        }
    }

    /// Delete an entire line by index. Returns the deleted line.
    pub fn delete_line(&mut self, row: usize) -> Option<String> {
        if self.lines.len() <= 1 {
            let old = std::mem::take(&mut self.lines[0]);
            self.modified = true;
            return Some(old);
        }
        if row < self.lines.len() {
            self.modified = true;
            Some(self.lines.remove(row))
        } else {
            None
        }
    }

    /// Insert a new line at the given index (pushes existing lines down).
    pub fn insert_line(&mut self, row: usize, content: String) {
        let row = row.min(self.lines.len());
        self.lines.insert(row, content);
        self.modified = true;
    }

    /// Replace a range [col_start, col_end) on a line with new text.
    pub fn replace_range(&mut self, row: usize, col_start: usize, col_end: usize, text: &str) {
        if let Some(line) = self.lines.get_mut(row) {
            let cs = col_start.min(line.len());
            let ce = col_end.min(line.len());
            line.replace_range(cs..ce, text);
            self.modified = true;
        }
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer() {
        let buf = Buffer::new();
        assert_eq!(buf.line_count(), 1);
        assert_eq!(buf.line(0), "");
        assert!(!buf.is_modified());
    }

    #[test]
    fn test_insert_char() {
        let mut buf = Buffer::new();
        buf.insert_char(0, 0, 'h');
        buf.insert_char(0, 1, 'i');
        assert_eq!(buf.line(0), "hi");
        assert!(buf.is_modified());
    }

    #[test]
    fn test_delete_char() {
        let mut buf = Buffer::new();
        buf.insert_str(0, 0, "hello");
        let ch = buf.delete_char(0, 2);
        assert_eq!(ch, Some('l'));
        assert_eq!(buf.line(0), "helo");
    }

    #[test]
    fn test_split_line() {
        let mut buf = Buffer::new();
        buf.insert_str(0, 0, "hello world");
        buf.split_line(0, 5);
        assert_eq!(buf.line_count(), 2);
        assert_eq!(buf.line(0), "hello");
        assert_eq!(buf.line(1), " world");
    }

    #[test]
    fn test_join_lines() {
        let mut buf = Buffer::new();
        buf.insert_str(0, 0, "hello");
        buf.split_line(0, 5);
        buf.insert_str(1, 5, " world");
        let col = buf.join_lines(0);
        assert_eq!(col, Some(5));
        assert_eq!(buf.line_count(), 1);
        assert_eq!(buf.line(0), "hello world");
    }

    #[test]
    fn test_delete_line() {
        let mut buf = Buffer::new();
        buf.insert_str(0, 0, "line 1");
        buf.insert_line(1, "line 2".into());
        buf.insert_line(2, "line 3".into());
        let deleted = buf.delete_line(1);
        assert_eq!(deleted.as_deref(), Some("line 2"));
        assert_eq!(buf.line_count(), 2);
        assert_eq!(buf.line(0), "line 1");
        assert_eq!(buf.line(1), "line 3");
    }

    #[test]
    fn test_delete_only_line() {
        let mut buf = Buffer::new();
        buf.insert_str(0, 0, "only line");
        let deleted = buf.delete_line(0);
        assert_eq!(deleted.as_deref(), Some("only line"));
        assert_eq!(buf.line_count(), 1);
        assert_eq!(buf.line(0), "");
    }

    #[test]
    fn test_insert_str() {
        let mut buf = Buffer::new();
        buf.insert_str(0, 0, "world");
        buf.insert_str(0, 0, "hello ");
        assert_eq!(buf.line(0), "hello world");
    }

    #[test]
    fn test_replace_range() {
        let mut buf = Buffer::new();
        buf.insert_str(0, 0, "hello world");
        buf.replace_range(0, 6, 11, "there");
        assert_eq!(buf.line(0), "hello there");
    }
}
