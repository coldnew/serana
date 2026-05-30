use std::sync::Arc;
use crate::lisp::types::Value;
use super::editor_state::{with_editor_state, with_editor_state_mut};
use super::helpers::extract_string;

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern_with_doc("leader-set-key", Value::Native(prim_leader_set_key),
        "(leader-set-key KEY ACTION) — Bind KEY under leader prefix to ACTION.");
    ns.intern_with_doc("leader-set-prefix", Value::Native(prim_leader_set_prefix),
        "(leader-set-prefix PREFIX) — Set leader key prefix (default \"SPC\").");
    ns.intern_with_doc("leader-bindings", Value::Native(prim_leader_bindings),
        "(leader-bindings) — Return all leader bindings as vector of [key action] pairs.");
}

/// (leader-set-key KEY ACTION) — Bind KEY under leader prefix to ACTION.
/// Stores as "<PREFIX>KEY" → action in EditorState.keybindings.
fn prim_leader_set_key(args: &[Value]) -> Result<Value, String> {
    let key = extract_string(args, 0)?;
    let action = args.get(1).cloned().ok_or_else(|| "missing action argument".to_string())?;
    with_editor_state_mut(|state| {
        let prefix = state.leader_key_prefix.clone();
        let full_key = format!("<{}>{}", prefix, key);
        state.keybindings.insert(full_key, action);
        Ok(Value::Nil)
    })
}

/// (leader-set-prefix PREFIX) — Set leader key prefix (default "SPC").
fn prim_leader_set_prefix(args: &[Value]) -> Result<Value, String> {
    let prefix = extract_string(args, 0)?;
    with_editor_state_mut(|state| {
        state.leader_key_prefix = prefix;
        Ok(Value::Nil)
    })
}

/// (leader-bindings) — Return all leader bindings as vector of [key action] pairs.
fn prim_leader_bindings(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        let prefix = format!("<{}>", state.leader_key_prefix);
        let pairs: Vec<Value> = state.keybindings.iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(k, v)| {
                let stripped = k[prefix.len()..].to_string();
                Value::Vector(Arc::new(vec![Value::string(stripped), v.clone()]))
            })
            .collect();
        Ok(Value::Vector(Arc::new(pairs)))
    })
}
