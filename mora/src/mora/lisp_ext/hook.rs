use crate::lisp::ns::Namespace;
use crate::lisp::types::Value;

use super::editor_state::{with_editor_state, with_editor_state_mut};
use super::helpers::extract_string;

fn prim_add_hook(args: &[Value]) -> Result<Value, String> {
    let hook_name = extract_string(args, 0)?;
    let handler = args.get(1).cloned().ok_or("missing hook handler")?;
    with_editor_state_mut(|state| {
        state.hooks.entry(hook_name).or_default().push(handler);
        Ok(Value::Nil)
    })
}

fn prim_define_key(args: &[Value]) -> Result<Value, String> {
    let key_desc = extract_string(args, 0)?;
    let action = args.get(1).cloned().ok_or("missing action")?;
    with_editor_state_mut(|state| {
        state.keybindings.insert(key_desc, action);
        Ok(Value::Nil)
    })
}

fn prim_remove_hook(args: &[Value]) -> Result<Value, String> {
    let hook_name = extract_string(args, 0)?;
    // Remove by index or by matching handler
    match args.get(1) {
        Some(Value::Int(idx)) => {
            let idx = *idx as usize;
            with_editor_state_mut(|state| {
                if let Some(handlers) = state.hooks.get_mut(&hook_name) {
                    if idx < handlers.len() {
                        handlers.remove(idx);
                    }
                }
                Ok(Value::Nil)
            })
        }
        Some(handler) => {
            // Remove by identity (last matching)
            let handler_str = format!("{:?}", handler);
            with_editor_state_mut(|state| {
                if let Some(handlers) = state.hooks.get_mut(&hook_name) {
                    if let Some(pos) = handlers
                        .iter()
                        .rposition(|h| format!("{:?}", h) == handler_str)
                    {
                        handlers.remove(pos);
                    }
                }
                Ok(Value::Nil)
            })
        }
        None => Err("remove-hook requires hook name and handler".to_string()),
    }
}

fn prim_run_hook(args: &[Value]) -> Result<Value, String> {
    let hook_name = extract_string(args, 0)?;
    with_editor_state(|state| {
        match state.hooks.get(&hook_name) {
            Some(handlers) => {
                let count = handlers.len();
                // In headless mode, just count how many would run
                Ok(Value::Int(count as i64))
            }
            None => Ok(Value::Int(0)),
        }
    })
}

fn prim_hook_bound(args: &[Value]) -> Result<Value, String> {
    let hook_name = extract_string(args, 0)?;
    with_editor_state(|state| {
        Ok(Value::Bool(
            state.hooks.get(&hook_name).map_or(false, |h| !h.is_empty()),
        ))
    })
}

fn prim_hooks_for(args: &[Value]) -> Result<Value, String> {
    let hook_name = extract_string(args, 0)?;
    with_editor_state(|state| {
        Ok(Value::Int(
            state.hooks.get(&hook_name).map_or(0, |h| h.len()) as i64,
        ))
    })
}

pub fn register(ns: &mut Namespace) {
    ns.intern_with_doc(
        "add-hook",
        Value::Native(prim_add_hook),
        "Add HANDLER to the named HOOK.",
    );
    ns.intern_private_with_doc(
        "add",
        Value::Native(prim_add_hook),
        "Add HANDLER to the named HOOK.",
    );
    ns.intern_with_doc(
        "define-key",
        Value::Native(prim_define_key),
        "Bind KEY to ACTION in the current keymap.",
    );
    ns.intern_private_with_doc(
        "define",
        Value::Native(prim_define_key),
        "Bind KEY to ACTION in the current keymap.",
    );
    ns.intern_with_doc(
        "remove-hook",
        Value::Native(prim_remove_hook),
        "Remove HANDLER from the named HOOK.",
    );
    ns.intern_private_with_doc(
        "remove",
        Value::Native(prim_remove_hook),
        "Remove HANDLER from the named HOOK.",
    );
    ns.intern_with_doc(
        "run-hook",
        Value::Native(prim_run_hook),
        "Run all handlers for the named HOOK.",
    );
    ns.intern_private_with_doc(
        "run",
        Value::Native(prim_run_hook),
        "Run all handlers for the named HOOK.",
    );
    ns.intern_with_doc(
        "hook-bound?",
        Value::Native(prim_hook_bound),
        "Return t if the named HOOK has handlers.",
    );
    ns.intern_private_with_doc(
        "bound?",
        Value::Native(prim_hook_bound),
        "Return t if the named HOOK has handlers.",
    );
    ns.intern_with_doc(
        "hooks-for",
        Value::Native(prim_hooks_for),
        "Return the number of handlers for the named HOOK.",
    );
    ns.intern_private_with_doc(
        "for",
        Value::Native(prim_hooks_for),
        "Return the number of handlers for the named HOOK.",
    );
}
