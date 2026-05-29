/// Cursor position in the document (0-indexed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
    /// Desired column for vertical movement. When moving up/down through
    /// shorter lines, the cursor snaps to this column when a long enough
    /// line is reached.
    pub preferred_col: usize,
}

impl Cursor {
    pub fn new(row: usize, col: usize) -> Self {
        Self {
            row,
            col,
            preferred_col: col,
        }
    }

    pub fn home() -> Self {
        Self::new(0, 0)
    }

    /// Clamp column to the actual line length. Call after any buffer
    /// modification or vertical movement.
    pub fn clamp_col(&mut self, line_len: usize) {
        if self.col > line_len {
            self.col = line_len;
        }
    }

    /// Clamp row and col to valid document bounds.
    pub fn clamp(&mut self, line_count: usize, line_len: usize) {
        if line_count == 0 {
            self.row = 0;
            self.col = 0;
            return;
        }
        if self.row >= line_count {
            self.row = line_count - 1;
        }
        self.clamp_col(line_len);
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::home()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPos {
    pub row: usize,
    pub col: usize,
}

impl From<Cursor> for CursorPos {
    fn from(c: Cursor) -> Self {
        Self { row: c.row, col: c.col }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_col() {
        let mut c = Cursor::new(0, 10);
        c.clamp_col(5);
        assert_eq!(c.col, 5);
    }

    #[test]
    fn test_clamp() {
        let mut c = Cursor::new(100, 200);
        c.clamp(10, 50);
        assert_eq!(c.row, 9);
        assert_eq!(c.col, 50);
    }

    #[test]
    fn test_preferred_col_preserved() {
        let mut c = Cursor::new(0, 20);
        c.clamp_col(5);
        assert_eq!(c.col, 5);
        assert_eq!(c.preferred_col, 20);
    }
}
