use crate::lisp::types::Value;
use super::editor_state::{with_editor_state, with_editor_state_mut};
use super::helpers::{extract_string, extract_int};

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern_with_doc("smartparens-wrap", Value::Native(prim_smartparens_wrap),
        "(smartparens-wrap OPEN CLOSE) — Wrap region between mark and cursor with OPEN/CLOSE chars.");
    ns.intern_with_doc("smartparens-insert-pair", Value::Native(prim_smartparens_insert_pair),
        "(smartparens-insert-pair OPEN CLOSE) — Insert paired chars, place cursor between them.");
    ns.intern_with_doc("smartparens-forward-slurp", Value::Native(prim_smartparens_forward_slurp),
        "(smartparens-forward-slurp) — Absorb next s-expression into current pair.");
    ns.intern_with_doc("smartparens-backward-slurp", Value::Native(prim_smartparens_backward_slurp),
        "(smartparens-backward-slurp) — Absorb previous s-expression into current pair.");
    ns.intern_with_doc("smartparens-unwrap", Value::Native(prim_smartparens_unwrap),
        "(smartparens-unwrap) — Remove surrounding pair, keep inner content.");
}

/// (smartparens-wrap OPEN CLOSE) — Wrap region (between mark and cursor) with OPEN/CLOSE chars.
fn prim_smartparens_wrap(args: &[Value]) -> Result<Value, String> {
    let open = extract_string(args, 0)?;
    let close = extract_string(args, 1)?;
    with_editor_state_mut(|state| {
        let mark = state.mark_pos.ok_or_else(|| "no active mark".to_string())?;
        let cursor = (state.cursor_row, state.cursor_col);

        // Normalize: ensure start <= end
        let (start, end) = if mark < cursor { (mark, cursor) } else { (cursor, mark) };

        if start.0 == end.0 {
            // Single-line wrap
            let line = state.lines.get(start.0)
                .ok_or_else(|| "cursor out of range".to_string())?;
            let before = &line[..start.1.min(line.len())];
            let inner = &line[start.1.min(line.len())..end.1.min(line.len())];
            let after = &line[end.1.min(line.len())..];
            state.lines[start.0] = format!("{}{}{}{}{}", before, open, inner, close, after);
        } else {
            // Multi-line wrap: insert open at start, close at end
            let first_line = state.lines.get(start.0)
                .ok_or_else(|| "cursor out of range".to_string())?;
            let before = &first_line[..start.1.min(first_line.len())];
            let after_start = &first_line[start.1.min(first_line.len())..];
            state.lines[start.0] = format!("{}{}{}", before, open, after_start);

            let last_line = state.lines.get(end.0)
                .ok_or_else(|| "cursor out of range".to_string())?;
            let before_end = &last_line[..end.1.min(last_line.len())];
            let after = &last_line[end.1.min(last_line.len())..];
            state.lines[end.0] = format!("{}{}{}", before_end, close, after);
        }

        state.modified = true;
        Ok(Value::Nil)
    })
}

/// (smartparens-insert-pair OPEN CLOSE) — Insert paired chars, place cursor between.
fn prim_smartparens_insert_pair(args: &[Value]) -> Result<Value, String> {
    let open = extract_string(args, 0)?;
    let close = extract_string(args, 1)?;
    with_editor_state_mut(|state| {
        let row = state.cursor_row;
        let col = state.cursor_col;
        let line = state.lines.get_mut(row)
            .ok_or_else(|| "cursor out of range".to_string())?;
        let insert_pos = col.min(line.len());
        let combined = format!("{}{}", open, close);
        line.insert_str(insert_pos, &combined);
        // Place cursor between open and close
        state.cursor_col = insert_pos + open.len();
        state.modified = true;
        Ok(Value::Nil)
    })
}

/// (smartparens-forward-slurp) — Absorb next s-expression into current pair.
fn prim_smartparens_forward_slurp(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let row = state.cursor_row;
        let line = state.lines.get_mut(row)
            .ok_or_else(|| "cursor out of range".to_string())?;

        // Find the closing pair char (one of ) ] } or a paired quote) after cursor
        let closing_chars = [')', ']', '}'];
        let mut close_pos = None;
        for (i, ch) in line.chars().enumerate() {
            if i >= state.cursor_col && closing_chars.contains(&ch) {
                close_pos = Some(i);
                break;
            }
        }
        let close_pos = close_pos.ok_or_else(|| "no closing pair found on line".to_string())?;

        // Find next non-whitespace after closing char
        let after_close = &line[close_pos + 1..];
        let mut word_start = None;
        for (i, ch) in after_close.char_indices() {
            if !ch.is_whitespace() {
                word_start = Some(close_pos + 1 + i);
                break;
            }
        }
        let word_start = word_start.ok_or_else(|| "no next s-expression to slurp".to_string())?;

        // Find end of next word (next whitespace or closing delimiter)
        let rest = &line[word_start..];
        let mut word_end = word_start;
        for (i, ch) in rest.char_indices() {
            if ch.is_whitespace() || closing_chars.contains(&ch) {
                break;
            }
            word_end = word_start + i + ch.len_utf8();
        }

        // Extract the word and move it before the closing char
        let word: String = line[word_start..word_end].to_string();
        line.replace_range(word_start..word_end, "");
        // Re-find close_pos (shifted after removal)
        let new_close_pos = if word_start < close_pos {
            close_pos - (word_end - word_start)
        } else {
            close_pos
        };
        line.insert_str(new_close_pos, &word);

        state.modified = true;
        Ok(Value::Nil)
    })
}

/// (smartparens-backward-slurp) — Absorb previous s-expression into current pair.
fn prim_smartparens_backward_slurp(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let row = state.cursor_row;
        let line = state.lines.get_mut(row)
            .ok_or_else(|| "cursor out of range".to_string())?;

        // Find the opening pair char (one of ( [ { ) before cursor
        let opening_chars = ['(', '[', '{'];
        let mut open_pos = None;
        for (i, ch) in line.char_indices() {
            if i < state.cursor_col && opening_chars.contains(&ch) {
                open_pos = Some(i);
            }
        }
        let open_pos = open_pos.ok_or_else(|| "no opening pair found on line".to_string())?;

        // Find previous non-whitespace before opening char
        let before_open = &line[..open_pos];
        let mut word_end = None;
        for (i, ch) in before_open.char_indices().rev() {
            if !ch.is_whitespace() {
                word_end = Some(i + ch.len_utf8());
                break;
            }
        }
        let word_end = word_end.ok_or_else(|| "no previous s-expression to slurp".to_string())?;

        // Find start of that word
        let before_word = &line[..word_end];
        let mut word_start = 0;
        for (i, ch) in before_word.char_indices().rev() {
            if ch.is_whitespace() || opening_chars.contains(&ch) {
                word_start = i + ch.len_utf8();
                break;
            }
        }

        // Extract the word and move it after the opening char
        let word: String = line[word_start..word_end].to_string();
        line.replace_range(word_start..word_end, "");
        // Re-find open_pos (shifted after removal)
        let new_open_pos = if word_end <= open_pos {
            open_pos - (word_end - word_start)
        } else {
            open_pos
        };
        line.insert_str(new_open_pos + 1, &word);

        state.modified = true;
        Ok(Value::Nil)
    })
}

/// (smartparens-unwrap) — Remove surrounding pair, keep inner content.
fn prim_smartparens_unwrap(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let row = state.cursor_row;
        let col = state.cursor_col;
        let line = state.lines.get(row)
            .ok_or_else(|| "cursor out of range".to_string())?;

        let pairs: [(char, char); 3] = [('(', ')'), ('[', ']'), ('{', '}')];

        // Search backwards from cursor for an opening char
        let mut found_pair = None;
        let mut depth = 0;
        for (i, ch) in line.char_indices().rev() {
            if i >= col { continue; }
            for &(open, close) in &pairs {
                if ch == close {
                    depth += 1;
                }
                if ch == open {
                    if depth == 0 {
                        found_pair = Some((i, open, close));
                        break;
                    }
                    depth -= 1;
                }
            }
            if found_pair.is_some() { break; }
        }

        let (open_pos, _open_ch, close_ch) = found_pair
            .ok_or_else(|| "no surrounding pair found".to_string())?;

        // Find matching closing char forward from open_pos
        let mut close_pos = None;
        let mut fwd_depth = 0;
        for (i, ch) in line.char_indices() {
            if i <= open_pos { continue; }
            for &(o, c) in &pairs {
                if ch == o { fwd_depth += 1; }
                if ch == c {
                    if fwd_depth == 0 {
                        if ch == close_ch {
                            close_pos = Some(i);
                            break;
                        }
                        fwd_depth -= 1;
                    } else {
                        fwd_depth -= 1;
                    }
                }
            }
            if close_pos.is_some() { break; }
        }

        let close_pos = close_pos.ok_or_else(|| "matching close not found".to_string())?;

        // Remove close first (higher index), then open
        let line_mut = state.lines.get_mut(row).unwrap();
        line_mut.remove(close_pos);
        line_mut.remove(open_pos);

        state.modified = true;
        Ok(Value::Nil)
    })
}
