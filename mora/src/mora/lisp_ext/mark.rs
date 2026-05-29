use crate::lisp::types::Value;
use super::editor_state::{with_editor_state, with_editor_state_mut};
use super::helpers::{extract_string, extract_int};

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern_with_doc("set-mark", Value::Native(prim_set_mark), "Set the mark at the current cursor position.");
    ns.intern_private_with_doc("mark-set!", Value::Native(prim_set_mark), "Set the mark at the current cursor position.");
    ns.intern_with_doc("goto-mark", Value::Native(prim_goto_mark), "Move the cursor to the mark position.");
    ns.intern_private_with_doc("mark-goto", Value::Native(prim_goto_mark), "Move the cursor to the mark position.");
    ns.intern_with_doc("pop-mark", Value::Native(prim_pop_mark), "Pop the mark ring and move the cursor to the previous mark.");
    ns.intern_private_with_doc("mark-pop!", Value::Native(prim_pop_mark), "Pop the mark ring and move the cursor to the previous mark.");
    ns.intern_with_doc("mark-active?", Value::Native(prim_mark_active), "Return t if the mark is currently active.");
    ns.intern_with_doc("mark-position", Value::Native(prim_mark_position), "Return the current mark position as [row col], or nil.");
    ns.intern_private_with_doc("mark-pos", Value::Native(prim_mark_position), "Return the current mark position as [row col], or nil.");
    ns.intern_with_doc("deactivate-mark", Value::Native(prim_deactivate_mark), "Deactivate the mark.");
    ns.intern_private_with_doc("deactivate-mark!", Value::Native(prim_deactivate_mark), "Deactivate the mark.");
}

/// (set-mark) → set mark at current cursor position
fn prim_set_mark(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let pos = (state.cursor_row, state.cursor_col);
        state.mark_active = true;
        state.mark_pos = Some(pos);
        state.mark_ring.push(pos);
        // Keep ring bounded
        if state.mark_ring.len() > 16 {
            state.mark_ring.remove(0);
        }
        Ok(Value::Nil)
    })
}
/// (goto-mark) → move cursor to mark position
fn prim_goto_mark(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        if let Some((row, col)) = state.mark_pos {
            state.cursor_row = row;
            state.cursor_col = col;
            Ok(Value::Nil)
        } else if let Some(&(row, col)) = state.mark_ring.last() {
            state.cursor_row = row;
            state.cursor_col = col;
            Ok(Value::Nil)
        } else {
            Err("no mark set".to_string())
        }
    })
}
/// (pop-mark) → pop mark ring, move cursor to previous mark
fn prim_pop_mark(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        if let Some((row, col)) = state.mark_ring.pop() {
            state.cursor_row = row;
            state.cursor_col = col;
            state.mark_pos = Some((row, col));
            Ok(Value::Bool(true))
        } else {
            Ok(Value::Bool(false))
        }
    })
}
/// (mark-active?) → is mark currently active?
fn prim_mark_active(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Bool(state.mark_active)))
}
/// (mark-position) → get current mark position [row col] or nil
fn prim_mark_position(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        match state.mark_pos {
            Some((row, col)) => Ok(Value::vector(vec![
                Value::Int(row as i64),
                Value::Int(col as i64),
            ])),
            None => Ok(Value::Nil),
        }
    })
}
/// (deactivate-mark) → deactivate the mark
fn prim_deactivate_mark(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        state.mark_active = false;
        Ok(Value::Nil)
    })
}
