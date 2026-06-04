use super::editor_state::{with_editor_state, with_editor_state_mut};
use super::helpers::extract_string;
use crate::lisp::types::Value;
use std::path::PathBuf;

// ── helpers ─────────────────────────────────────────────────────────────────

fn session_dir() -> PathBuf {
    dirs::home_dir()
        .expect("no home directory found")
        .join(".mora")
}

/// Escape a string for embedding in a s-expression literal.
fn lisp_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// Write a session file in mora s-expression format.
/// Format:
///   (session
///     (file-path "...")
///     (cursor-row N)
///     (cursor-col N)
///     (mode "...")
///     (content "line1\nline2\n..."))
fn serialize_session() -> String {
    with_editor_state(|state| {
        let file_path = state
            .file_path
            .as_ref()
            .map(|p| lisp_escape(p))
            .unwrap_or_else(|| "nil".to_string());
        let content = lisp_escape(&state.lines.join("\n"));
        format!(
            "(session\n  (file-path {file_path})\n  (cursor-row {row})\n  (cursor-col {col})\n  (mode {mode})\n  (content {content}))\n",
            file_path = file_path,
            row = state.cursor_row,
            col = state.cursor_col,
            mode = lisp_escape(&state.mode),
            content = content,
        )
    })
}

/// Simple s-expression field extractor: find `(key VALUE)` in the session data.
fn extract_field(src: &str, key: &str) -> Option<String> {
    let pat = format!("({} ", key);
    let start = src.find(&pat)? + pat.len();
    let rest = &src[start..];
    let end = rest.find(')')?;
    let val = rest[..end].trim();
    Some(val.to_string())
}

/// Parse a quoted string value, stripping surrounding quotes and unescaping.
fn parse_quoted(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        let inner = &s[1..s.len() - 1];
        let mut out = String::with_capacity(inner.len());
        let mut chars = inner.chars();
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('\\') => out.push('\\'),
                    Some('"') => out.push('"'),
                    Some(c) => {
                        out.push('\\');
                        out.push(c);
                    }
                    None => out.push('\\'),
                }
            } else {
                out.push(ch);
            }
        }
        out
    } else if s == "nil" {
        String::new()
    } else {
        s.to_string()
    }
}

/// Parse an integer value.
fn parse_int(s: &str) -> Result<usize, String> {
    s.trim()
        .parse::<usize>()
        .map_err(|e| format!("session: expected integer, got '{}': {}", s, e))
}

// ── primitives ──────────────────────────────────────────────────────────────

/// (session-save PATH) — Save current editor state to a mora s-expression file.
fn prim_session_save(args: &[Value]) -> Result<Value, String> {
    let path = if args.is_empty() {
        session_dir().join("session.mora")
    } else {
        PathBuf::from(extract_string(args, 0)?)
    };
    let content = serialize_session();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("session-save: failed to create dir: {}", e))?;
    }
    std::fs::write(&path, &content)
        .map_err(|e| format!("session-save: failed to write {}: {}", path.display(), e))?;
    Ok(Value::Nil)
}

/// (session-load PATH) — Load editor state from a mora s-expression file.
fn prim_session_load(args: &[Value]) -> Result<Value, String> {
    let path = if args.is_empty() {
        session_dir().join("session.mora")
    } else {
        PathBuf::from(extract_string(args, 0)?)
    };
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("session-load: failed to read {}: {}", path.display(), e))?;

    let file_path_raw = extract_field(&content, "file-path").unwrap_or_default();
    let cursor_row = extract_field(&content, "cursor-row")
        .and_then(|v| parse_int(&v).ok())
        .unwrap_or(0);
    let cursor_col = extract_field(&content, "cursor-col")
        .and_then(|v| parse_int(&v).ok())
        .unwrap_or(0);
    let mode = extract_field(&content, "mode").unwrap_or_else(|| "normal".to_string());
    let content_raw = extract_field(&content, "content").unwrap_or_default();

    let file_path_val = parse_quoted(&file_path_raw);
    let lines_text = parse_quoted(&content_raw);
    let lines: Vec<String> = if lines_text.is_empty() {
        vec![String::new()]
    } else {
        lines_text.split('\n').map(|s| s.to_string()).collect()
    };
    let mode_val = parse_quoted(&mode);

    with_editor_state_mut(|state| {
        state.file_path = if file_path_val.is_empty() {
            None
        } else {
            Some(file_path_val)
        };
        state.cursor_row = cursor_row;
        state.cursor_col = cursor_col;
        state.mode = mode_val;
        state.lines = lines;
        Ok(Value::Nil)
    })
}

/// (session-save-desktop) — Save desktop (all buffers) to ~/.mora/desktop.mora.
fn prim_session_save_desktop(_args: &[Value]) -> Result<Value, String> {
    let path = session_dir().join("desktop.mora");
    let content = serialize_session();
    std::fs::create_dir_all(session_dir())
        .map_err(|e| format!("session-save-desktop: failed to create dir: {}", e))?;
    std::fs::write(&path, &content)
        .map_err(|e| format!("session-save-desktop: failed to write: {}", e))?;
    Ok(Value::Nil)
}

/// (session-load-desktop) — Load desktop from ~/.mora/desktop.mora.
fn prim_session_load_desktop(_args: &[Value]) -> Result<Value, String> {
    let path = session_dir().join("desktop.mora");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("session-load-desktop: failed to read: {}", e))?;

    let file_path_raw = extract_field(&content, "file-path").unwrap_or_default();
    let cursor_row = extract_field(&content, "cursor-row")
        .and_then(|v| parse_int(&v).ok())
        .unwrap_or(0);
    let cursor_col = extract_field(&content, "cursor-col")
        .and_then(|v| parse_int(&v).ok())
        .unwrap_or(0);
    let mode = extract_field(&content, "mode").unwrap_or_else(|| "normal".to_string());
    let content_raw = extract_field(&content, "content").unwrap_or_default();

    let file_path_val = parse_quoted(&file_path_raw);
    let lines_text = parse_quoted(&content_raw);
    let lines: Vec<String> = if lines_text.is_empty() {
        vec![String::new()]
    } else {
        lines_text.split('\n').map(|s| s.to_string()).collect()
    };
    let mode_val = parse_quoted(&mode);

    with_editor_state_mut(|state| {
        state.file_path = if file_path_val.is_empty() {
            None
        } else {
            Some(file_path_val)
        };
        state.cursor_row = cursor_row;
        state.cursor_col = cursor_col;
        state.mode = mode_val;
        state.lines = lines;
        Ok(Value::Nil)
    })
}

// ── registration ────────────────────────────────────────────────────────────

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern_with_doc(
        "session-save",
        Value::Native(prim_session_save),
        "Save current editor state to a file (default: ~/.mora/session.mora).",
    );
    ns.intern_with_doc(
        "session-load",
        Value::Native(prim_session_load),
        "Load editor state from a file (default: ~/.mora/session.mora).",
    );
    ns.intern_with_doc(
        "session-save-desktop",
        Value::Native(prim_session_save_desktop),
        "Save desktop (all buffers) to ~/.mora/desktop.mora.",
    );
    ns.intern_with_doc(
        "session-load-desktop",
        Value::Native(prim_session_load_desktop),
        "Load desktop from ~/.mora/desktop.mora.",
    );
}
