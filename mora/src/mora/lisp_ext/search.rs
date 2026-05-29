use crate::lisp::types::Value;
use super::editor_state::*;
use super::helpers::{extract_string, extract_int};

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern("search-forward", Value::Native(prim_search_forward));
    ns.intern("search-backward", Value::Native(prim_search_backward));
    ns.intern("looking-at", Value::Native(prim_looking_at));
}

/// (search-forward "pattern") → search forward for pattern, move cursor, return position or nil
fn prim_search_forward(args: &[Value]) -> Result<Value, String> {
    let pattern = extract_string(args, 0)?;
    with_editor_state_mut(|state| {
        // Search from current cursor position forward
        let start_col_offset = if state.cursor_col < state.lines.get(state.cursor_row).map_or(0, |l| l.len()) {
            state.cursor_col + 1
        } else {
            0
        };
        for row in state.cursor_row..state.lines.len() {
            let line = &state.lines[row];
            let search_from = if row == state.cursor_row { start_col_offset } else { 0 };
            if search_from < line.len() {
                if let Some(pos) = line[search_from..].find(&pattern) {
                    let col = search_from + pos;
                    state.cursor_row = row;
                    state.cursor_col = col;
                    return Ok(Value::Int((row * 10000 + col) as i64));
                }
            }
        }
        Ok(Value::Nil)
    })
}

/// (search-backward "pattern") → search backward for pattern
fn prim_search_backward(args: &[Value]) -> Result<Value, String> {
    let pattern = extract_string(args, 0)?;
    with_editor_state_mut(|state| {
        // Search from current cursor position backward
        for row in (0..=state.cursor_row).rev() {
            let line = state.lines.get(row).map(|s| s.as_str()).unwrap_or("");
            let search_end = if row == state.cursor_row {
                state.cursor_col
            } else {
                line.len()
            };
            if search_end > 0 {
                if let Some(pos) = line[..search_end].rfind(&pattern) {
                    state.cursor_row = row;
                    state.cursor_col = pos;
                    return Ok(Value::Int((row * 10000 + pos) as i64));
                }
            }
        }
        Ok(Value::Nil)
    })
}

/// (looking-at "pattern") → does text at cursor match pattern?
fn prim_looking_at(args: &[Value]) -> Result<Value, String> {
    let pattern = extract_string(args, 0)?;
    with_editor_state(|state| {
        if let Some(line) = state.lines.get(state.cursor_row) {
            if state.cursor_col < line.len() {
                return Ok(Value::Bool(line[state.cursor_col..].starts_with(&pattern)));
            }
        }
        Ok(Value::Bool(false))
    })
}
