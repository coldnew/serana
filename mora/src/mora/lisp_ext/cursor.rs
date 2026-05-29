use crate::lisp::types::Value;
use super::editor_state::{with_editor_state, with_editor_state_mut};
use super::helpers::extract_int;

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern("cursor-row", Value::Native(prim_cursor_row));
    ns.intern_private("row", Value::Native(prim_cursor_row));
    ns.intern("cursor-col", Value::Native(prim_cursor_col));
    ns.intern_private("col", Value::Native(prim_cursor_col));
    ns.intern("cursor-set!", Value::Native(prim_cursor_set));
    ns.intern_private("set!", Value::Native(prim_cursor_set));
    ns.intern("cursor-goto-line", Value::Native(prim_cursor_goto_line));
    ns.intern_private("goto-line", Value::Native(prim_cursor_goto_line));
    ns.intern("cursor-forward!", Value::Native(prim_cursor_forward));
    ns.intern_private("forward!", Value::Native(prim_cursor_forward));
    ns.intern("cursor-backward!", Value::Native(prim_cursor_backward));
    ns.intern_private("backward!", Value::Native(prim_cursor_backward));
    ns.intern("cursor-beginning-of-line", Value::Native(prim_cursor_bol));
    ns.intern_private("beginning-of-line", Value::Native(prim_cursor_bol));
    ns.intern("cursor-end-of-line", Value::Native(prim_cursor_eol));
    ns.intern_private("end-of-line", Value::Native(prim_cursor_eol));
}

fn prim_cursor_row(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Int(state.cursor_row as i64)))
}

fn prim_cursor_col(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Int(state.cursor_col as i64)))
}

fn prim_cursor_set(args: &[Value]) -> Result<Value, String> {
    let row = extract_int(args, 0)? as usize;
    let col = extract_int(args, 1)? as usize;
    with_editor_state_mut(|state| {
        state.cursor_row = row.min(state.lines.len().saturating_sub(1));
        state.cursor_col = col;
        Ok(Value::Nil)
    })
}

fn prim_cursor_goto_line(args: &[Value]) -> Result<Value, String> {
    let line = extract_int(args, 0)? as usize;
    with_editor_state_mut(|state| {
        let target = line
            .saturating_sub(1)
            .min(state.lines.len().saturating_sub(1));
        state.cursor_row = target;
        state.cursor_col = 0;
        Ok(Value::Nil)
    })
}

fn prim_cursor_forward(args: &[Value]) -> Result<Value, String> {
    let n = args
        .get(0)
        .and_then(|v| match v {
            Value::Int(n) => Some(*n as usize),
            _ => None,
        })
        .unwrap_or(1);
    with_editor_state_mut(|state| {
        state.cursor_col += n;
        Ok(Value::Nil)
    })
}

fn prim_cursor_backward(args: &[Value]) -> Result<Value, String> {
    let n = args
        .get(0)
        .and_then(|v| match v {
            Value::Int(n) => Some(*n as usize),
            _ => None,
        })
        .unwrap_or(1);
    with_editor_state_mut(|state| {
        state.cursor_col = state.cursor_col.saturating_sub(n);
        Ok(Value::Nil)
    })
}

fn prim_cursor_bol(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        state.cursor_col = 0;
        Ok(Value::Nil)
    })
}

fn prim_cursor_eol(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        if let Some(line) = state.lines.get(state.cursor_row) {
            state.cursor_col = line.len();
        }
        Ok(Value::Nil)
    })
}
