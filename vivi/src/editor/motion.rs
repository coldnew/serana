use crate::buffer::Buffer;
use crate::cursor::Cursor;
use display_protocol::KeyCode;

/// A cursor motion — parsed from a key event, then applied to get a target position.
///
/// Every motion knows whether it is **inclusive** (like Vim's `$` and `e`):
/// when used with an operator, the range extends one past the returned column
/// so the exclusive upper-bound `[..ec)` still covers the last character.
///
/// Screen-relative motions (`H`/`L`/`M`) and absolute-line (`G`) are handled
/// inline in `handle_normal` because they depend on scroll state or use count
/// as a direct parameter rather than a repeat count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    /// `h` / Left
    Left(usize),
    /// `l` / Right
    Right(usize),
    /// `j` / Down
    Down(usize),
    /// `k` / Up
    Up(usize),
    /// `w` — start of next word
    WordForward(usize),
    /// `b` — start of previous word
    WordBackward(usize),
    /// `e` — end of word (inclusive)
    WordEnd(usize),
    /// `$` — end of line (inclusive)
    LineEnd,
    /// `^` — first non-blank on line
    FirstNonBlank,
    /// `0` — column zero
    LineStart,
    /// `+` / Enter — next line, first non-blank
    NextLineFirstNonBlank(usize),
    /// `-` — previous line, first non-blank
    PrevLineFirstNonBlank(usize),
    /// `_` — N-1 lines down, first non-blank
    ScreenLineFirstNonBlank(usize),
    /// Ctrl+F / PageDown
    PageDown(usize),
    /// Ctrl+B / PageUp
    PageUp(usize),
}

impl Motion {
    /// Parse a single-key motion from a key event. Returns `None` for
    /// non-motion keys or multi-key sequences (`f`/`F`/`t`/`T`/`g`/`G`/`H`/`L`/`M`).
    pub fn from_key(code: KeyCode, count: usize) -> Option<Motion> {
        Some(match code {
            KeyCode::Char('h') | KeyCode::Left => Motion::Left(count),
            KeyCode::Char('l') | KeyCode::Right => Motion::Right(count),
            KeyCode::Char('j') | KeyCode::Down => Motion::Down(count),
            KeyCode::Char('k') | KeyCode::Up => Motion::Up(count),
            KeyCode::Char('w') => Motion::WordForward(count),
            KeyCode::Char('b') => Motion::WordBackward(count),
            KeyCode::Char('e') => Motion::WordEnd(count),
            KeyCode::Char('$') => Motion::LineEnd,
            KeyCode::Char('^') => Motion::FirstNonBlank,
            KeyCode::Enter | KeyCode::Char('+') => Motion::NextLineFirstNonBlank(count),
            KeyCode::Char('-') => Motion::PrevLineFirstNonBlank(count),
            KeyCode::Char('_') => Motion::ScreenLineFirstNonBlank(count),
            KeyCode::PageDown | KeyCode::Char('\x06') => Motion::PageDown(count),
            KeyCode::PageUp | KeyCode::Char('\x02') => Motion::PageUp(count),
            _ => return None,
        })
    }

    /// Whether this motion is inclusive for operator use (`:help inclusive`).
    pub fn is_inclusive(self) -> bool {
        matches!(self, Motion::LineEnd | Motion::WordEnd(_))
    }

    /// Compute the target position from the current cursor, buffer, and
    /// Whether this motion is vertical (j/k/PageDown/PageUp/+/-
    /// — preserves `preferred_col` across lines).
    pub fn is_vertical(self) -> bool {
        matches!(
            self,
            Motion::Down(_)
                | Motion::Up(_)
                | Motion::PageDown(_)
                | Motion::PageUp(_)
                | Motion::NextLineFirstNonBlank(_)
                | Motion::PrevLineFirstNonBlank(_)
                | Motion::ScreenLineFirstNonBlank(_)
        )
    }
    /// terminal height. Returns `(row, col)`.
    pub fn apply(self, cursor: &Cursor, buffer: &Buffer, term_height: usize) -> (usize, usize) {
        match self {
            Motion::Left(n) => {
                let col = cursor.col.saturating_sub(n);
                (cursor.row, col)
            }
            Motion::Right(n) => {
                let max = buffer.line_len(cursor.row).saturating_sub(1);
                let col = cursor.col.min(max).saturating_add(n).min(max);
                (cursor.row, col)
            }
            Motion::Down(n) => {
                let max_row = buffer.line_count().saturating_sub(1);
                let row = (cursor.row + n).min(max_row);
                let col = cursor.preferred_col.min(buffer.line_len(row));
                (row, col)
            }
            Motion::Up(n) => {
                let row = cursor.row.saturating_sub(n);
                let col = cursor.preferred_col.min(buffer.line_len(row));
                (row, col)
            }
            Motion::WordForward(n) => repeat_motion(cursor, n, buffer, word_forward),
            Motion::WordBackward(n) => repeat_motion(cursor, n, buffer, word_backward),
            Motion::WordEnd(n) => repeat_motion(cursor, n, buffer, word_end),
            Motion::LineEnd => {
                let len = buffer.line_len(cursor.row);
                let col = if len > 0 { len - 1 } else { 0 };
                (cursor.row, col)
            }
            Motion::FirstNonBlank => {
                let col = first_non_blank(buffer.line(cursor.row));
                (cursor.row, col)
            }
            Motion::LineStart => (cursor.row, 0),
            Motion::NextLineFirstNonBlank(n) => {
                let row = (cursor.row + n).min(buffer.line_count().saturating_sub(1));
                (row, first_non_blank(buffer.line(row)))
            }
            Motion::PrevLineFirstNonBlank(n) => {
                let row = cursor.row.saturating_sub(n);
                (row, first_non_blank(buffer.line(row)))
            }
            Motion::ScreenLineFirstNonBlank(n) => {
                let row = (cursor.row + n.saturating_sub(1))
                    .min(buffer.line_count().saturating_sub(1));
                (row, first_non_blank(buffer.line(row)))
            }
            Motion::PageDown(n) => {
                let page = term_height.saturating_sub(2);
                let max_row = buffer.line_count().saturating_sub(1);
                let row = (cursor.row + page * n).min(max_row);
                let col = cursor.preferred_col.min(buffer.line_len(row));
                (row, col)
            }
            Motion::PageUp(n) => {
                let page = term_height.saturating_sub(2);
                let row = cursor.row.saturating_sub(page * n);
                let col = cursor.preferred_col.min(buffer.line_len(row));
                (row, col)
            }
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────

fn first_non_blank(line: &str) -> usize {
    line.find(|c: char| !c.is_ascii_whitespace()).unwrap_or(0)
}

fn repeat_motion(
    cursor: &Cursor,
    n: usize,
    buffer: &Buffer,
    step: fn(usize, usize, &Buffer) -> (usize, usize),
) -> (usize, usize) {
    let mut row = cursor.row;
    let mut col = cursor.col;
    for _ in 0..n {
        let (r, c) = step(row, col, buffer);
        row = r;
        col = c;
    }
    (row, col)
}

// ─── Word motion primitives ───────────────────────────────────────

#[inline]
pub fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// `w` — start of next word (exclusive).
fn word_forward(row: usize, col: usize, buffer: &Buffer) -> (usize, usize) {
    let line = buffer.line(row);
    let bytes = line.as_bytes();
    let len = bytes.len();

    let mut c = col;
    let mut in_word = c < len && is_word_byte(bytes[c]);

    while c < len {
        if in_word && !is_word_byte(bytes[c]) {
            in_word = false;
        } else if !in_word && is_word_byte(bytes[c]) {
            return (row, c);
        }
        c += 1;
    }

    if row + 1 < buffer.line_count() {
        (row + 1, 0)
    } else {
        (row, col)
    }
}

/// `b` — start of previous word.
fn word_backward(row: usize, col: usize, buffer: &Buffer) -> (usize, usize) {
    if col == 0 {
        if row > 0 {
            let prev = row - 1;
            let len = buffer.line_len(prev);
            return (prev, if len > 0 { len - 1 } else { 0 });
        }
        return (row, col);
    }

    let bytes = buffer.line(row).as_bytes();
    let mut c = col.saturating_sub(1);

    while c > 0 && !is_word_byte(bytes[c]) {
        c -= 1;
    }
    while c > 0 && is_word_byte(bytes[c - 1]) {
        c -= 1;
    }

    (row, c)
}

/// `e` — end of current/next word (inclusive).
fn word_end(row: usize, col: usize, buffer: &Buffer) -> (usize, usize) {
    let line = buffer.line(row);
    let bytes = line.as_bytes();
    let len = bytes.len();

    let mut c = col + 1;
    let mut in_word = false;

    while c < len {
        if !in_word && is_word_byte(bytes[c]) {
            in_word = true;
        } else if in_word && !is_word_byte(bytes[c]) {
            return (row, c - 1);
        }
        c += 1;
    }

    if len > 0 {
        (row, len - 1)
    } else if row + 1 < buffer.line_count() {
        word_end(row + 1, 0, buffer)
    } else {
        (row, col)
    }
}

// ─── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Buffer;

    fn buf(lines: &[&str]) -> Buffer {
        let mut b = Buffer::new();
        for (i, line) in lines.iter().enumerate() {
            if i == 0 {
                b.replace_range(0, 0, 0, line);
            } else {
                b.insert_line(i, line.to_string());
            }
        }
        b
    }

    fn cursor(row: usize, col: usize, preferred: usize) -> Cursor {
        Cursor {
            row,
            col,
            preferred_col: preferred,
        }
    }

    #[test]
    fn line_end_is_inclusive() {
        assert!(Motion::LineEnd.is_inclusive());
        assert!(Motion::WordEnd(1).is_inclusive());
        assert!(!Motion::Left(1).is_inclusive());
        assert!(!Motion::WordForward(1).is_inclusive());
    }

    #[test]
    fn dollar_goes_to_last_char() {
        let b = buf(&["hello"]);
        let c = cursor(0, 0, 0);
        let (r, col) = Motion::LineEnd.apply(&c, &b, 24);
        assert_eq!(r, 0);
        assert_eq!(col, 4); // 'o' at index 4
    }

    #[test]
    fn dollar_on_empty_line() {
        let b = buf(&[""]);
        let c = cursor(0, 0, 0);
        let (r, col) = Motion::LineEnd.apply(&c, &b, 24);
        assert_eq!(r, 0);
        assert_eq!(col, 0);
    }

    #[test]
    fn word_forward_skips_word() {
        let b = buf(&["hello world"]);
        let c = cursor(0, 0, 0);
        let (r, col) = Motion::WordForward(1).apply(&c, &b, 24);
        assert_eq!(r, 0);
        assert_eq!(col, 6); // 'w'
    }

    #[test]
    fn word_forward_wraps_line() {
        let b = buf(&["ab", "cd"]);
        let c = cursor(0, 1, 1);
        let (r, col) = Motion::WordForward(1).apply(&c, &b, 24);
        assert_eq!(r, 1);
        assert_eq!(col, 0);
    }

    #[test]
    fn word_end_finds_end() {
        let b = buf(&["hello world"]);
        let c = cursor(0, 0, 0);
        let (r, col) = Motion::WordEnd(1).apply(&c, &b, 24);
        assert_eq!(r, 0);
        assert_eq!(col, 4); // 'o'
    }

    #[test]
    fn first_non_blank() {
        let b = buf(&["  hello"]);
        let c = cursor(0, 0, 0);
        let (r, col) = Motion::FirstNonBlank.apply(&c, &b, 24);
        assert_eq!(r, 0);
        assert_eq!(col, 2);
    }

    #[test]
    fn down_uses_preferred_col() {
        let b = buf(&["hello world", "hi"]);
        let c = cursor(0, 10, 10);
        let (r, col) = Motion::Down(1).apply(&c, &b, 24);
        assert_eq!(r, 1);
        assert_eq!(col, 2); // clamped to shorter line
    }
}
