use super::editor_state::{with_editor_state, with_editor_state_mut};
use super::helpers::extract_int;
use crate::lisp::types::Value;

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern_with_doc(
        "cursor-row",
        Value::Native(prim_cursor_row),
        "Return the current cursor row (0-indexed).",
    );
    ns.intern_private_with_doc(
        "row",
        Value::Native(prim_cursor_row),
        "Return the current cursor row (0-indexed).",
    );
    ns.intern_with_doc(
        "cursor-col",
        Value::Native(prim_cursor_col),
        "Return the current cursor column (0-indexed).",
    );
    ns.intern_private_with_doc(
        "col",
        Value::Native(prim_cursor_col),
        "Return the current cursor column (0-indexed).",
    );
    ns.intern_with_doc(
        "cursor-set!",
        Value::Native(prim_cursor_set),
        "Move the cursor to ROW and COL.",
    );
    ns.intern_private_with_doc(
        "set!",
        Value::Native(prim_cursor_set),
        "Move the cursor to ROW and COL.",
    );
    ns.intern_with_doc(
        "cursor-goto-line",
        Value::Native(prim_cursor_goto_line),
        "Move the cursor to line N (1-indexed).",
    );
    ns.intern_private_with_doc(
        "goto-line",
        Value::Native(prim_cursor_goto_line),
        "Move the cursor to line N (1-indexed).",
    );
    ns.intern_with_doc(
        "cursor-forward!",
        Value::Native(prim_cursor_forward),
        "Move the cursor forward by N characters.",
    );
    ns.intern_private_with_doc(
        "forward!",
        Value::Native(prim_cursor_forward),
        "Move the cursor forward by N characters.",
    );
    ns.intern_with_doc(
        "cursor-backward!",
        Value::Native(prim_cursor_backward),
        "Move the cursor backward by N characters.",
    );
    ns.intern_private_with_doc(
        "backward!",
        Value::Native(prim_cursor_backward),
        "Move the cursor backward by N characters.",
    );
    ns.intern_with_doc(
        "cursor-beginning-of-line",
        Value::Native(prim_cursor_bol),
        "Move the cursor to the beginning of the line.",
    );
    ns.intern_private_with_doc(
        "beginning-of-line",
        Value::Native(prim_cursor_bol),
        "Move the cursor to the beginning of the line.",
    );
    ns.intern_with_doc(
        "cursor-end-of-line",
        Value::Native(prim_cursor_eol),
        "Move the cursor to the end of the line.",
    );
    ns.intern_private_with_doc(
        "end-of-line",
        Value::Native(prim_cursor_eol),
        "Move the cursor to the end of the line.",
    );
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
