use crate::lisp::types::Value;
use super::editor_state::{with_editor_state, with_editor_state_mut};
use super::helpers::{extract_string, extract_int};

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern("var-set", Value::Native(prim_var_set));
    ns.intern_private("set-var!", Value::Native(prim_var_set));
    ns.intern("var-get", Value::Native(prim_var_get));
    ns.intern_private("get-var", Value::Native(prim_var_get));
    ns.intern("var-local", Value::Native(prim_var_local));
    ns.intern_private("local-var!", Value::Native(prim_var_local));
    ns.intern("var-bound?", Value::Native(prim_var_bound));
}

/// (var-set "name" value) → set buffer-local variable
fn prim_var_set(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    let value = args.get(1).cloned().ok_or("var-set requires a value")?;
    with_editor_state_mut(|state| {
        state.buffer_local_vars.insert(name, value);
        Ok(Value::Nil)
    })
}
/// (var-get "name") → get buffer-local variable value
fn prim_var_get(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    with_editor_state(|state| {
        match state.buffer_local_vars.get(&name) {
            Some(val) => Ok(val.clone()),
            None => Ok(Value::Nil),
        }
    })
}
/// (var-local "name" default) → set default value for buffer-local variable
fn prim_var_local(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    let default = args.get(1).cloned().unwrap_or(Value::Nil);
    with_editor_state_mut(|state| {
        if !state.buffer_local_vars.contains_key(&name) {
            state.buffer_local_vars.insert(name, default);
        }
        Ok(Value::Nil)
    })
}
/// (var-bound? "name") → check if buffer-local variable is bound
fn prim_var_bound(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    with_editor_state(|state| Ok(Value::Bool(state.buffer_local_vars.contains_key(&name))))
}
