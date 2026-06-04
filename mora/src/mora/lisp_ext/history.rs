use super::editor_state::*;
use super::helpers::extract_string;
use crate::lisp::types::Value;
use std::cell::RefCell;
use std::collections::HashMap;

// ── recentf: thread-local storage ──────────────────────────────────────────

thread_local! {
    static RECENT_FILES: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

const MAX_RECENT_FILES: usize = 50;

fn history_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .expect("no home directory found")
        .join(".mora")
}

// ── savehist primitives ────────────────────────────────────────────────────

fn prim_savehist_save(_args: &[Value]) -> Result<Value, String> {
    let vars = with_editor_state(|state| {
        Ok::<HashMap<String, Value>, String>(state.buffer_local_vars.clone())
    })?;

    let mut lines = String::new();
    for (key, val) in &vars {
        let val_str: String = match val {
            Value::String(s) => s.to_string(),
            Value::Int(n) => n.to_string(),
            Value::Bool(b) => {
                if *b {
                    "t".to_string()
                } else {
                    "nil".to_string()
                }
            }
            Value::Nil => "nil".to_string(),
            other => format!("{}", other),
        };
        lines.push_str(key);
        lines.push('=');
        lines.push_str(&val_str);
        lines.push('\n');
    }

    let path = history_dir().join("history.dat");
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("savehist-save: failed to create dir: {}", e))?;
        }
    }
    std::fs::write(&path, &lines)
        .map_err(|e| format!("savehist-save: failed to write {}: {}", path.display(), e))?;
    Ok(Value::Nil)
}

fn prim_savehist_load(_args: &[Value]) -> Result<Value, String> {
    let path = history_dir().join("history.dat");
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Ok(Value::Nil), // file doesn't exist yet, not an error
    };

    for line in content.lines() {
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].to_string();
            let val_str = &line[eq_pos + 1..];
            let val = match val_str {
                "nil" => Value::Nil,
                "t" => Value::Bool(true),
                s => {
                    if let Ok(n) = s.parse::<i64>() {
                        Value::Int(n)
                    } else {
                        Value::string(s)
                    }
                }
            };
            with_editor_state_mut(|state| {
                state.buffer_local_vars.insert(key, val);
                Ok::<(), String>(())
            })?;
        }
    }
    Ok(Value::Nil)
}

// ── recentf primitives ─────────────────────────────────────────────────────

fn prim_recentf_add(args: &[Value]) -> Result<Value, String> {
    let path = extract_string(args, 0)?;
    RECENT_FILES.with(|files| {
        let mut files = files.borrow_mut();
        // Remove existing entry if present (for dedup)
        files.retain(|p| p != &path);
        // Insert at front
        files.insert(0, path);
        // Trim to max
        if files.len() > MAX_RECENT_FILES {
            files.truncate(MAX_RECENT_FILES);
        }
    });
    Ok(Value::Nil)
}

fn prim_recentf_list(_args: &[Value]) -> Result<Value, String> {
    let paths = RECENT_FILES.with(|files| {
        files
            .borrow()
            .iter()
            .map(|s| Value::string(s.clone()))
            .collect()
    });
    Ok(Value::vector(paths))
}

fn prim_recentf_clear(_args: &[Value]) -> Result<Value, String> {
    RECENT_FILES.with(|files| {
        files.borrow_mut().clear();
    });
    Ok(Value::Nil)
}

fn prim_recentf_save(_args: &[Value]) -> Result<Value, String> {
    let content = RECENT_FILES.with(|files| files.borrow().join("\n"));
    let path = history_dir().join("recentf.dat");
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("recentf-save: failed to create dir: {}", e))?;
        }
    }
    std::fs::write(&path, &content)
        .map_err(|e| format!("recentf-save: failed to write {}: {}", path.display(), e))?;
    Ok(Value::Nil)
}

fn prim_recentf_load(_args: &[Value]) -> Result<Value, String> {
    let path = history_dir().join("recentf.dat");
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Ok(Value::Nil),
    };
    let entries: Vec<String> = content
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();
    RECENT_FILES.with(|files| {
        *files.borrow_mut() = entries;
    });
    Ok(Value::Nil)
}

// ── registration ───────────────────────────────────────────────────────────

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern_with_doc(
        "savehist-save",
        Value::Native(prim_savehist_save),
        "Save current minibuffer/keybinding vars to ~/.mora/history.dat.",
    );
    ns.intern_with_doc(
        "savehist-load",
        Value::Native(prim_savehist_load),
        "Load saved vars from ~/.mora/history.dat.",
    );

    ns.intern_with_doc(
        "recentf-add",
        Value::Native(prim_recentf_add),
        "Add a file path to the recent files list.",
    );
    ns.intern_with_doc(
        "recentf-list",
        Value::Native(prim_recentf_list),
        "Return vector of recent file paths.",
    );
    ns.intern_with_doc(
        "recentf-clear",
        Value::Native(prim_recentf_clear),
        "Clear the recent files list.",
    );
    ns.intern_with_doc(
        "recentf-save",
        Value::Native(prim_recentf_save),
        "Save recent files to ~/.mora/recentf.dat.",
    );
    ns.intern_with_doc(
        "recentf-load",
        Value::Native(prim_recentf_load),
        "Load recent files from ~/.mora/recentf.dat.",
    );
}
