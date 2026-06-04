use super::editor_state::*;
use super::helpers::extract_int;
use crate::lisp::types::Value;

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern_with_doc(
        "region-beginning",
        Value::Native(prim_region_beginning),
        "Return the position of the beginning of the region.",
    );
    ns.intern_private_with_doc(
        "beginning",
        Value::Native(prim_region_beginning),
        "Return the position of the beginning of the region.",
    );
    ns.intern_with_doc(
        "region-end",
        Value::Native(prim_region_end),
        "Return the position of the end of the region.",
    );
    ns.intern_private_with_doc(
        "end",
        Value::Native(prim_region_end),
        "Return the position of the end of the region.",
    );
    ns.intern_with_doc(
        "region-active?",
        Value::Native(prim_region_active),
        "Return t if the region is currently active.",
    );
    ns.intern_private_with_doc(
        "active?",
        Value::Native(prim_region_active),
        "Return t if the region is currently active.",
    );
    ns.intern_with_doc(
        "delete-region",
        Value::Native(prim_delete_region),
        "Delete the text between the mark and the cursor.",
    );
    ns.intern_private_with_doc(
        "delete",
        Value::Native(prim_delete_region),
        "Delete the text between the mark and the cursor.",
    );
    ns.intern_with_doc(
        "buffer-substring",
        Value::Native(prim_buffer_substring),
        "Return the contents of the region as a string.",
    );
    ns.intern_private_with_doc(
        "substring",
        Value::Native(prim_buffer_substring),
        "Return the contents of the region as a string.",
    );
}

/// (region-beginning) → position of region start
fn prim_region_beginning(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        if let Some((row, col)) = state.mark_pos {
            let mark_flat = row * 10000 + col;
            let cursor_flat = state.cursor_row * 10000 + state.cursor_col;
            let (r, c) = if mark_flat <= cursor_flat {
                (row, col)
            } else {
                (state.cursor_row, state.cursor_col)
            };
            Ok(Value::Int((r * 10000 + c) as i64))
        } else {
            Ok(Value::Int(
                (state.cursor_row * 10000 + state.cursor_col) as i64,
            ))
        }
    })
}

/// (region-end) → position of region end
fn prim_region_end(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        if let Some((row, col)) = state.mark_pos {
            let mark_flat = row * 10000 + col;
            let cursor_flat = state.cursor_row * 10000 + state.cursor_col;
            let (r, c) = if mark_flat >= cursor_flat {
                (row, col)
            } else {
                (state.cursor_row, state.cursor_col)
            };
            Ok(Value::Int((r * 10000 + c) as i64))
        } else {
            Ok(Value::Int(
                (state.cursor_row * 10000 + state.cursor_col) as i64,
            ))
        }
    })
}

/// (region-active?) → is the region currently active?
fn prim_region_active(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Bool(state.mark_active)))
}

/// (delete-region) → delete text between mark and cursor
fn prim_delete_region(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        if !state.mark_active {
            return Err("region is not active".to_string());
        }
        if let Some((mark_row, mark_col)) = state.mark_pos {
            let start_row = mark_row.min(state.cursor_row);
            let end_row = mark_row.max(state.cursor_row);
            let (start_col, end_col) = if mark_row == state.cursor_row {
                (
                    mark_col.min(state.cursor_col),
                    mark_col.max(state.cursor_col),
                )
            } else if mark_row < state.cursor_row {
                (mark_col, state.cursor_col)
            } else {
                (state.cursor_col, mark_col)
            };
            if start_row == end_row {
                // Single line deletion
                if let Some(line) = state.lines.get_mut(start_row) {
                    let actual_end = end_col.min(line.len());
                    let actual_start = start_col.min(actual_end);
                    line.replace_range(actual_start..actual_end, "");
                }
            } else {
                // Multi-line deletion
                if start_row < state.lines.len() {
                    let start_line = state.lines[start_row].clone();
                    let end_line = state.lines.get(end_row).cloned().unwrap_or_default();
                    let prefix = if start_col <= start_line.len() {
                        &start_line[..start_col]
                    } else {
                        &start_line
                    };
                    let suffix = if end_col <= end_line.len() {
                        &end_line[end_col..]
                    } else {
                        ""
                    };
                    let merged = format!("{}{}", prefix, suffix);
                    state
                        .lines
                        .splice(start_row..=end_row, std::iter::once(merged));
                }
            }
            state.cursor_row = start_row;
            state.cursor_col = start_col;
            state.mark_active = false;
            state.mark_pos = None;
        }
        Ok(Value::Nil)
    })
}

/// (buffer-substring start end) → extract text between positions
fn prim_buffer_substring(args: &[Value]) -> Result<Value, String> {
    let start = extract_int(args, 0)? as usize;
    let end = extract_int(args, 1)? as usize;
    with_editor_state(|state| {
        let start_row = start / 10000;
        let start_col = start % 10000;
        let end_row = end / 10000;
        let end_col = end % 10000;
        if start_row == end_row {
            if let Some(line) = state.lines.get(start_row) {
                let s = start_col.min(line.len());
                let e = end_col.min(line.len());
                return Ok(Value::string(&line[s..e]));
            }
            return Ok(Value::string(""));
        }
        let mut result = String::new();
        for row in start_row..=end_row.min(state.lines.len().saturating_sub(1)) {
            if let Some(line) = state.lines.get(row) {
                if row == start_row {
                    let s = start_col.min(line.len());
                    result.push_str(&line[s..]);
                } else if row == end_row {
                    let e = end_col.min(line.len());
                    result.push_str(&line[..e]);
                } else {
                    result.push_str(line);
                }
                if row < end_row {
                    result.push('\n');
                }
            }
        }
        Ok(Value::string(&result))
    })
}
