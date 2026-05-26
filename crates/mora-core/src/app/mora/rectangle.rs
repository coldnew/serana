use crate::app::mora::buffer::Buffer;
use crate::app::mora::buffer::Cursor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RectRegion {
    pub start_row: usize,
    pub end_row: usize,
    pub start_col: usize,
    pub end_col: usize,
}

impl RectRegion {
    pub fn from_corners(a: Cursor, b: Cursor) -> Self {
        Self {
            start_row: a.row.min(b.row),
            end_row: a.row.max(b.row),
            start_col: a.col.min(b.col),
            end_col: a.col.max(b.col),
        }
    }

    pub fn normalized(self) -> Self {
        Self {
            start_row: self.start_row.min(self.end_row),
            end_row: self.start_row.max(self.end_row),
            start_col: self.start_col.min(self.end_col),
            end_col: self.start_col.max(self.end_col),
        }
    }
}

pub fn kill_rectangle(buf: &mut Buffer, rect: &RectRegion) -> Vec<String> {
    let mut killed = Vec::new();
    let start = rect.start_row;
    let end = (rect.end_row + 1).min(buf.lines.len());
    let c1 = rect.start_col;
    let c2 = rect.end_col;

    for row in start..end {
        let line = &buf.lines[row];
        if c1 < line.len() {
            let until_end = line[c1..].chars().count();
            let count = if c2 < line.len() {
                line[c1..c2].chars().count()
            } else {
                until_end
            };
            let extracted: String = line.chars().skip(line[..c1].chars().count()).take(count).collect();
            killed.push(extracted);
        } else {
            killed.push(String::new());
        }
    }

    let c1 = rect.start_col;
    let c2 = rect.end_col;
    for row in start..end {
        let line = &mut buf.lines[row];
        if c1 < line.len() {
            let char_start = line[..c1].chars().count();
            let char_end = if c2 < line.len() {
                line[..c2].chars().count()
            } else {
                line.chars().count()
            };
            let before: String = line.chars().take(char_start).collect();
            let after: String = line.chars().skip(char_end).collect();
            *line = before + &after;
        }
    }

    buf.modified = true;
    killed
}

pub fn yank_rectangle(buf: &mut Buffer, rect: &RectRegion, strings: &[String]) {
    let start = rect.start_row;
    for (i, s) in strings.iter().enumerate() {
        let row = start + i;
        if row >= buf.lines.len() {
            break;
        }
        let line = &mut buf.lines[row];
        let insert_col = if i == 0 { rect.start_col } else { rect.end_col };
        let char_pos = line[..insert_col.min(line.len())].chars().count();
        let before: String = line.chars().take(char_pos).collect();
        let after: String = line.chars().skip(char_pos).collect();
        *line = before + s + &after;
    }
    buf.modified = true;
}

pub fn insert_rectangle(buf: &mut Buffer, rect: &RectRegion, s: &str) {
    let start = rect.start_row;
    let end = (rect.end_row + 1).min(buf.lines.len());
    for row in start..end {
        let line = &mut buf.lines[row];
        let col = rect.start_col;
        let char_pos = line[..col.min(line.len())].chars().count();
        let before: String = line.chars().take(char_pos).collect();
        let after: String = line.chars().skip(char_pos).collect();
        *line = before + s + &after;
    }
    buf.modified = true;
}

pub fn clear_rectangle(buf: &mut Buffer, rect: &RectRegion) {
    let start = rect.start_row;
    let end = (rect.end_row + 1).min(buf.lines.len());
    let c1 = rect.start_col;
    let c2 = rect.end_col;
    for row in start..end {
        let line = &mut buf.lines[row];
        if c1 < line.len() {
            let char_start = line[..c1].chars().count();
            let char_end = if c2 < line.len() {
                line[..c2].chars().count()
            } else {
                line.chars().count()
            };
            let before: String = line.chars().take(char_start).collect();
            let after: String = line.chars().skip(char_end).collect();
            let spaces = char_end.saturating_sub(char_start);
            let padding: String = std::iter::repeat(' ').take(spaces).collect();
            *line = before + &padding + &after;
        } else {
            let spaces = c2.saturating_sub(c1);
            let padding: String = std::iter::repeat(' ').take(spaces).collect();
            let char_pos = line[..c1.min(line.len())].chars().count();
            let before: String = line.chars().take(char_pos).collect();
            let after: String = line.chars().skip(char_pos).collect();
            *line = before + &padding + &after;
        }
    }
    buf.modified = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::mora::buffer::Buffer;

    fn buf_from(s: &str) -> Buffer {
        let lines: Vec<String> = s.lines().map(String::from).collect();
        let mut b = Buffer::new();
        b.lines = lines;
        b
    }

    #[test]
    fn test_kill_rectangle() {
        let mut b = buf_from("abcdef\nghijkl\nmnopqr");
        let rect = RectRegion {
            start_row: 0,
            end_row: 2,
            start_col: 1,
            end_col: 3,
        };
        let killed = kill_rectangle(&mut b, &rect);
        assert_eq!(killed, vec!["bc", "hi", "no"]);
        assert_eq!(b.lines[0], "adef");
        assert_eq!(b.lines[1], "gjkl");
        assert_eq!(b.lines[2], "mpqr");
    }

    #[test]
    fn test_yank_rectangle() {
        let mut b = buf_from("adef\ngjkl\nmpqr");
        let rect = RectRegion {
            start_row: 0,
            end_row: 2,
            start_col: 1,
            end_col: 1,
        };
        let strings = vec!["BC".to_string(), "HI".to_string(), "NO".to_string()];
        yank_rectangle(&mut b, &rect, &strings);
        assert_eq!(b.lines[0], "aBCdef");
        assert_eq!(b.lines[1], "gHIjkl");
        assert_eq!(b.lines[2], "mNOpqr");
    }

    #[test]
    fn test_clear_rectangle() {
        let mut b = buf_from("abcdef\nghijkl\nmnopqr");
        let rect = RectRegion {
            start_row: 0,
            end_row: 2,
            start_col: 1,
            end_col: 4,
        };
        clear_rectangle(&mut b, &rect);
        assert_eq!(b.lines[0], "a   ef");
        assert_eq!(b.lines[1], "g   kl");
        assert_eq!(b.lines[2], "m   qr");
    }
}
