use super::editor_state::with_editor_state_mut;
use super::helpers::extract_string;
use crate::lisp::types::Value;

fn prim_iedit(_args: &[Value]) -> Result<Value, String> {
    // Start iedit mode on word under cursor
    // This sets a flag in the shared state; the editor checks it on next cycle
    with_editor_state_mut(|state| {
        state.status_message = "iedit".to_string();
    });
    Ok(Value::Nil)
}

fn prim_iedit_regex(args: &[Value]) -> Result<Value, String> {
    let pattern = extract_string(args, 0)?;
    with_editor_state_mut(|state| {
        state.status_message = format!("iedit-regex:{}", pattern);
    });
    Ok(Value::Nil)
}

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern_with_doc(
        "start",
        Value::Native(prim_iedit),
        "Start iedit mode on word under cursor.",
    );
    ns.intern_with_doc(
        "regex",
        Value::Native(prim_iedit_regex),
        "Start iedit with regex pattern.",
    );
}
