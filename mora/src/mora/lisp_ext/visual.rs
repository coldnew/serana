use super::editor_state::*;
use super::helpers::extract_string;
use crate::lisp::types::Value;

// ── which-key ──────────────────────────────────────────────────

/// (which-key-for-prefix PREFIX) → vector of [key description] pairs for bindings under PREFIX.
fn prim_which_key_for_prefix(args: &[Value]) -> Result<Value, String> {
    let prefix = extract_string(args, 0)?;
    with_editor_state(|state| {
        let mut results: Vec<Value> = Vec::new();
        for (key, action) in &state.keybindings {
            if key.starts_with(&prefix) {
                let desc = match action {
                    Value::String(s) => s.to_string(),
                    Value::Symbol(s) => s.name.to_string(),
                    Value::Fn(f) => f.name.clone().unwrap_or_else(|| "<anonymous>".to_string()),
                    Value::Native(_) => "<native>".to_string(),
                    other => format!("{}", other),
                };
                results.push(Value::vector(vec![
                    Value::string(key.as_str()),
                    Value::string(desc.as_str()),
                ]));
            }
        }
        Ok(Value::vector(results))
    })
}

/// (which-key-bindings) → vector of [key action] pairs for all keybindings.
fn prim_which_key_bindings(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        let mut results: Vec<Value> = Vec::new();
        for (key, action) in &state.keybindings {
            let desc = match action {
                Value::String(s) => s.to_string(),
                Value::Symbol(s) => s.name.to_string(),
                Value::Fn(f) => f.name.clone().unwrap_or_else(|| "<anonymous>".to_string()),
                Value::Native(_) => "<native>".to_string(),
                other => format!("{}", other),
            };
            results.push(Value::vector(vec![
                Value::string(key.as_str()),
                Value::string(desc.as_str()),
            ]));
        }
        Ok(Value::vector(results))
    })
}

// ── line-reminder ──────────────────────────────────────────────

/// (line-reminder-get-modified) → vector of modified line numbers (0-indexed).
/// Simplified: returns all lines since last save.
fn prim_line_reminder_get_modified(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        if !state.modified {
            return Ok(Value::vector(vec![]));
        }
        let all_lines: Vec<Value> = (0..state.lines.len())
            .map(|i| Value::Int(i as i64))
            .collect();
        Ok(Value::vector(all_lines))
    })
}

/// (line-reminder-clear) → clear modified line tracking.
fn prim_line_reminder_clear(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        state.modified = false;
        Ok(Value::Bool(true))
    })
}

// ── focus-mode ─────────────────────────────────────────────────

/// (focus-mode-toggle) → toggle focus mode on/off.
fn prim_focus_mode_toggle(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        state.focus_mode = !state.focus_mode;
        if state.focus_mode {
            state.status_message = "Focus mode ON".to_string();
        } else {
            state.status_message = "Focus mode OFF".to_string();
        }
        Ok(Value::Bool(state.focus_mode))
    })
}

/// (focus-mode?) → return t if focus mode is active.
fn prim_focus_mode_p(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Bool(state.focus_mode)))
}

// ── visual-regexp ──────────────────────────────────────────────

/// (query-replace-pattern OLD NEW) → replace all occurrences of OLD with NEW in buffer.
fn prim_query_replace_pattern(args: &[Value]) -> Result<Value, String> {
    let old = extract_string(args, 0)?;
    let new = extract_string(args, 1)?;
    with_editor_state_mut(|state| {
        let mut count: i64 = 0;
        for line in &mut state.lines {
            let occurrences = line.matches(&old).count();
            if occurrences > 0 {
                *line = line.replace(&old, &new);
                count += occurrences as i64;
            }
        }
        state.modified = true;
        Ok(Value::Int(count))
    })
}

// ── registration ───────────────────────────────────────────────

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    // which-key
    ns.intern_with_doc(
        "which-key-for-prefix",
        Value::Native(prim_which_key_for_prefix),
        "Return vector of [key description] pairs for bindings under PREFIX.",
    );
    ns.intern_private_with_doc(
        "for-prefix",
        Value::Native(prim_which_key_for_prefix),
        "Return vector of [key description] pairs for bindings under PREFIX.",
    );
    ns.intern_with_doc(
        "which-key-bindings",
        Value::Native(prim_which_key_bindings),
        "Return all keybindings as a vector of [key action] pairs.",
    );
    ns.intern_private_with_doc(
        "bindings",
        Value::Native(prim_which_key_bindings),
        "Return all keybindings as a vector of [key action] pairs.",
    );

    // line-reminder
    ns.intern_with_doc(
        "line-reminder-get-modified",
        Value::Native(prim_line_reminder_get_modified),
        "Return vector of modified line numbers (0-indexed).",
    );
    ns.intern_private_with_doc(
        "get-modified",
        Value::Native(prim_line_reminder_get_modified),
        "Return vector of modified line numbers (0-indexed).",
    );
    ns.intern_with_doc(
        "line-reminder-clear",
        Value::Native(prim_line_reminder_clear),
        "Clear modified line tracking.",
    );
    ns.intern_private_with_doc(
        "clear",
        Value::Native(prim_line_reminder_clear),
        "Clear modified line tracking.",
    );

    // focus-mode
    ns.intern_with_doc(
        "focus-mode-toggle",
        Value::Native(prim_focus_mode_toggle),
        "Toggle focus mode on/off.",
    );
    ns.intern_private_with_doc(
        "toggle",
        Value::Native(prim_focus_mode_toggle),
        "Toggle focus mode on/off.",
    );
    ns.intern_with_doc(
        "focus-mode?",
        Value::Native(prim_focus_mode_p),
        "Return t if focus mode is active.",
    );
    ns.intern_private_with_doc(
        "active?",
        Value::Native(prim_focus_mode_p),
        "Return t if focus mode is active.",
    );

    // visual-regexp
    ns.intern_with_doc(
        "query-replace-pattern",
        Value::Native(prim_query_replace_pattern),
        "Replace all occurrences of OLD with NEW in buffer.",
    );
    ns.intern_private_with_doc(
        "replace",
        Value::Native(prim_query_replace_pattern),
        "Replace all occurrences of OLD with NEW in buffer.",
    );
}
