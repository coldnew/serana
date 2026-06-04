use super::super::undo_tree::Snapshot;
use super::editor_state::*;
use super::helpers::extract_int;
use crate::lisp::types::Value;

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern_with_doc("undo", Value::Native(prim_undo), "Undo the last change.");
    ns.intern_with_doc(
        "redo",
        Value::Native(prim_redo),
        "Redo the last undone change.",
    );
    ns.intern_with_doc(
        "undo-boundary",
        Value::Native(prim_undo_boundary),
        "Push an undo boundary (snapshot current state).",
    );
    ns.intern_private_with_doc(
        "boundary",
        Value::Native(prim_undo_boundary),
        "Push an undo boundary (snapshot current state).",
    );
    ns.intern_with_doc(
        "undo-enabled?",
        Value::Native(prim_undo_enabled),
        "Return t if undo is enabled.",
    );
    ns.intern_private_with_doc(
        "enabled?",
        Value::Native(prim_undo_enabled),
        "Return t if undo is enabled.",
    );
    ns.intern_with_doc(
        "undo-tree-branches",
        Value::Native(prim_undo_tree_branches),
        "Return the number of branches at the current undo tree point.",
    );
    ns.intern_with_doc(
        "undo-tree-switch-branch",
        Value::Native(prim_undo_tree_switch_branch),
        "Switch to branch N at the current undo tree point.",
    );
    ns.intern_with_doc(
        "undo-tree-visualize",
        Value::Native(prim_undo_tree_visualize),
        "Return a string representation of the undo tree.",
    );
    ns.intern_with_doc(
        "undo-tree-node-count",
        Value::Native(prim_undo_tree_node_count),
        "Return the total number of nodes in the undo tree.",
    );
    ns.intern_with_doc(
        "undo-tree-can-undo?",
        Value::Native(prim_undo_tree_can_undo),
        "Return t if undo is possible from the current position.",
    );
    ns.intern_with_doc(
        "undo-tree-can-redo?",
        Value::Native(prim_undo_tree_can_redo),
        "Return t if redo is possible from the current position.",
    );
}

/// (undo) → undo last change
fn prim_undo(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        if !state.undo_enabled {
            return Err("undo is disabled".to_string());
        }
        if state.undo_tree.undo() {
            let snap = state.undo_tree.current().clone();
            state.lines = snap.lines;
            state.cursor_row = snap.cursor_row;
            state.cursor_col = snap.cursor_col;
            Ok(Value::Bool(true))
        } else {
            Ok(Value::Bool(false))
        }
    })
}

/// (redo) → redo last undone change
fn prim_redo(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        if state.undo_tree.redo() {
            let snap = state.undo_tree.current().clone();
            state.lines = snap.lines;
            state.cursor_row = snap.cursor_row;
            state.cursor_col = snap.cursor_col;
            Ok(Value::Bool(true))
        } else {
            Ok(Value::Bool(false))
        }
    })
}

/// (undo-boundary) → push an undo boundary (snapshot current state)
fn prim_undo_boundary(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        if state.undo_enabled {
            state.undo_tree.record(Snapshot {
                lines: state.lines.clone(),
                cursor_row: state.cursor_row,
                cursor_col: state.cursor_col,
            });
        }
        Ok(Value::Nil)
    })
}

/// (undo-enabled?) → is undo enabled?
fn prim_undo_enabled(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Bool(state.undo_enabled)))
}

/// (undo-tree-branches) → number of branches at current point
fn prim_undo_tree_branches(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Int(state.undo_tree.branch_count() as i64)))
}

/// (undo-tree-switch-branch N) → switch to branch N at current point
fn prim_undo_tree_switch_branch(args: &[Value]) -> Result<Value, String> {
    let n = extract_int(args, 0)? as usize;
    with_editor_state_mut(|state| {
        if state.undo_tree.switch_branch(n) {
            let snap = state.undo_tree.current().clone();
            state.lines = snap.lines;
            state.cursor_row = snap.cursor_row;
            state.cursor_col = snap.cursor_col;
            Ok(Value::Bool(true))
        } else {
            Ok(Value::Bool(false))
        }
    })
}

/// (undo-tree-visualize) → string representation of the tree
fn prim_undo_tree_visualize(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::string(state.undo_tree.visualize())))
}

/// (undo-tree-node-count) → total nodes in tree
fn prim_undo_tree_node_count(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Int(state.undo_tree.node_count() as i64)))
}

/// (undo-tree-can-undo?) → can undo from current position?
fn prim_undo_tree_can_undo(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Bool(state.undo_tree.can_undo())))
}

/// (undo-tree-can-redo?) → can redo from current position?
fn prim_undo_tree_can_redo(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Bool(state.undo_tree.can_redo())))
}
