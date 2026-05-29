use crate::lisp::types::Value;
use super::editor_state::{with_editor_state, with_editor_state_mut};
use super::helpers::{extract_string, extract_int};

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern("kill-ring-yank", Value::Native(prim_kill_ring_yank));
    ns.intern_private("yank", Value::Native(prim_kill_ring_yank));
    ns.intern("kill-ring-push", Value::Native(prim_kill_ring_push));
    ns.intern_private("kill-push!", Value::Native(prim_kill_ring_push));
    ns.intern("kill-ring-pop", Value::Native(prim_kill_ring_pop));
    ns.intern_private("kill-pop!", Value::Native(prim_kill_ring_pop));
    ns.intern("kill-ring-pop-back", Value::Native(prim_kill_ring_pop_back));
    ns.intern_private("kill-pop-back!", Value::Native(prim_kill_ring_pop_back));
    ns.intern("kill-ring-count", Value::Native(prim_kill_ring_count));
    ns.intern_private("kill-count", Value::Native(prim_kill_ring_count));
    ns.intern("kill-ring-contents", Value::Native(prim_kill_ring_contents));
    ns.intern_private("kill-contents", Value::Native(prim_kill_ring_contents));
}

/// (kill-ring-yank) → returns most recent kill entry or nil
fn prim_kill_ring_yank(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        if state.kill_ring.is_empty() {
            Ok(Value::Nil)
        } else {
            let idx = if state.kill_ring_idx < state.kill_ring.len() {
                state.kill_ring_idx
            } else {
                0
            };
            Ok(Value::string(&state.kill_ring[idx]))
        }
    })
}
/// (kill-ring-push "text") → push text onto kill ring
fn prim_kill_ring_push(args: &[Value]) -> Result<Value, String> {
    let text = extract_string(args, 0)?;
    with_editor_state_mut(|state| {
        state.kill_ring.push(text);
        state.kill_ring_idx = state.kill_ring.len() - 1;
        // Keep kill ring bounded (max 60 like emacs)
        if state.kill_ring.len() > 60 {
            state.kill_ring.remove(0);
            if state.kill_ring_idx > 0 {
                state.kill_ring_idx -= 1;
            }
        }
        Ok(Value::Nil)
    })
}
/// (kill-ring-pop) → rotate kill ring forward, return entry
fn prim_kill_ring_pop(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        if state.kill_ring.is_empty() {
            return Ok(Value::Nil);
        }
        state.kill_ring_idx = (state.kill_ring_idx + 1) % state.kill_ring.len();
        Ok(Value::string(&state.kill_ring[state.kill_ring_idx]))
    })
}
/// (kill-ring-pop-back) → rotate kill ring backward, return entry
fn prim_kill_ring_pop_back(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        if state.kill_ring.is_empty() {
            return Ok(Value::Nil);
        }
        if state.kill_ring_idx == 0 {
            state.kill_ring_idx = state.kill_ring.len() - 1;
        } else {
            state.kill_ring_idx -= 1;
        }
        Ok(Value::string(&state.kill_ring[state.kill_ring_idx]))
    })
}
/// (kill-ring-count) → number of entries
fn prim_kill_ring_count(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Int(state.kill_ring.len() as i64)))
}
/// (kill-ring-contents) → vector of all kill ring entries
fn prim_kill_ring_contents(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        Ok(Value::vector(
            state.kill_ring.iter().map(|s| Value::string(s)).collect(),
        ))
    })
}
