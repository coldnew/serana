use crate::lisp::ns::Namespace;
use crate::lisp::types::Value;

use super::editor_state::with_editor_state;
use super::helpers::{extract_int, extract_string};

fn prim_window_count(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Int(state.window_count as i64)))
}

pub fn register(ns: &mut Namespace) {
    ns.intern("window-count", Value::Native(prim_window_count));
    ns.intern_private("count", Value::Native(prim_window_count));
}
