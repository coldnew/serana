use crate::lisp::types::Value;
use super::editor_state::{with_editor_state, with_editor_state_mut};
use super::helpers::{extract_string, extract_int};

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern_with_doc("register-set", Value::Native(prim_register_set), "Store VALUE in the register named CHAR.");
    ns.intern_private_with_doc("set-register!", Value::Native(prim_register_set), "Store VALUE in the register named CHAR.");
    ns.intern_with_doc("register-get", Value::Native(prim_register_get), "Return the value stored in register CHAR, or nil.");
    ns.intern_private_with_doc("get-register", Value::Native(prim_register_get), "Return the value stored in register CHAR, or nil.");
    ns.intern_with_doc("register-list", Value::Native(prim_register_list), "Return an alist of all register names to values.");
    ns.intern_private_with_doc("list-registers", Value::Native(prim_register_list), "Return an alist of all register names to values.");
}

/// (register-set ?char "value") → store value in named register
fn prim_register_set(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    let ch = name.chars().next().ok_or("register name must be a char")?;
    let value = extract_string(args, 1)?;
    with_editor_state_mut(|state| {
        state.registers.insert(ch, value);
        Ok(Value::Nil)
    })
}
/// (register-get ?char) → retrieve value from named register
fn prim_register_get(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    let ch = name.chars().next().ok_or("register name must be a char")?;
    with_editor_state(|state| {
        match state.registers.get(&ch) {
            Some(val) => Ok(Value::string(val)),
            None => Ok(Value::Nil),
        }
    })
}
/// (register-list) → map of all register names to values
fn prim_register_list(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        let pairs: Vec<(Value, Value)> = state.registers.iter()
            .map(|(ch, val)| (Value::keyword(ch.to_string()), Value::string(val)))
            .collect();
        Ok(Value::map(pairs))
    })
}
