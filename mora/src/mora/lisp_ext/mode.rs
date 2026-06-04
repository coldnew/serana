use super::editor_state::{with_editor_state, with_editor_state_mut};
use super::helpers::extract_string;
use crate::lisp::types::Value;

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern_with_doc(
        "current-mode",
        Value::Native(prim_current_mode),
        "Return the current editor mode keyword.",
    );
    ns.intern_private_with_doc(
        "current",
        Value::Native(prim_current_mode),
        "Return the current editor mode keyword.",
    );
    ns.intern_with_doc(
        "set-mode!",
        Value::Native(prim_set_mode),
        "Set the editor mode to MODE.",
    );
    ns.intern_private_with_doc(
        "set!",
        Value::Native(prim_set_mode),
        "Set the editor mode to MODE.",
    );
    ns.intern_with_doc(
        "set-minor-mode!",
        Value::Native(prim_set_minor_mode),
        "Toggle the minor mode MODE.",
    );
    ns.intern_private_with_doc(
        "toggle-minor!",
        Value::Native(prim_set_minor_mode),
        "Toggle the minor mode MODE.",
    );
}

fn prim_current_mode(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::keyword(state.mode.clone())))
}

fn prim_set_mode(args: &[Value]) -> Result<Value, String> {
    let mode = match &args[0] {
        Value::Keyword(k) => k.name.to_string(),
        Value::String(s) => s.to_string(),
        _ => return Err("expected keyword or string".to_string()),
    };
    with_editor_state_mut(|state| {
        state.mode = mode;
        Ok(Value::Nil)
    })
}

fn prim_set_minor_mode(args: &[Value]) -> Result<Value, String> {
    let mode = extract_string(args, 0)?;
    with_editor_state_mut(|state| {
        if let Some(pos) = state.minor_modes.iter().position(|m| m == &mode) {
            state.minor_modes.remove(pos);
        } else {
            state.minor_modes.push(mode);
        }
        Ok(Value::Nil)
    })
}
