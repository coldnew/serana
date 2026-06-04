use crate::lisp::ns::Namespace;
use crate::lisp::types::Value;

use super::editor_state::with_editor_state;

fn prim_window_count(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Int(state.window_count as i64)))
}

pub fn register(ns: &mut Namespace) {
    ns.intern_with_doc(
        "window-count",
        Value::Native(prim_window_count),
        "Return the number of visible windows.",
    );
    ns.intern_private_with_doc(
        "count",
        Value::Native(prim_window_count),
        "Return the number of visible windows.",
    );
}
