use crate::lisp::types::Value;
use super::editor_state::{with_editor_state, with_editor_state_mut};
use super::helpers::{extract_string, extract_int};

pub fn register(ns: &mut crate::lisp::ns::Namespace) {
    ns.intern("buffer-name", Value::Native(prim_buffer_name));
    ns.intern_private("name", Value::Native(prim_buffer_name));
    ns.intern("buffer-content", Value::Native(prim_buffer_content));
    ns.intern_private("content", Value::Native(prim_buffer_content));
    ns.intern("buffer-set-content", Value::Native(prim_buffer_set_content));
    ns.intern_private("set-content!", Value::Native(prim_buffer_set_content));
    ns.intern("buffer-modified?", Value::Native(prim_buffer_modified));
    ns.intern_private("modified?", Value::Native(prim_buffer_modified));
    ns.intern("buffer-file-path", Value::Native(prim_buffer_file_path));
    ns.intern_private("file-path", Value::Native(prim_buffer_file_path));
    ns.intern("buffer-line-count", Value::Native(prim_buffer_line_count));
    ns.intern_private("line-count", Value::Native(prim_buffer_line_count));
    ns.intern("buffer-current-line", Value::Native(prim_buffer_current_line));
    ns.intern_private("current-line", Value::Native(prim_buffer_current_line));
    ns.intern("buffer-line-at", Value::Native(prim_buffer_line_at));
    ns.intern_private("line-at", Value::Native(prim_buffer_line_at));
    ns.intern("buffer-insert!", Value::Native(prim_buffer_insert));
    ns.intern_private("insert!", Value::Native(prim_buffer_insert));
    ns.intern("buffer-replace-line!", Value::Native(prim_buffer_replace_line));
    ns.intern_private("replace-line!", Value::Native(prim_buffer_replace_line));
    ns.intern("buffer-narrowed?", Value::Native(prim_buffer_narrowed));
    ns.intern_private("narrowed?", Value::Native(prim_buffer_narrowed));
    ns.intern("narrow-to-region", Value::Native(prim_narrow_to_region));
    ns.intern("widen", Value::Native(prim_widen));
    ns.intern("buffer-substring", Value::Native(prim_buffer_substring_range));
    ns.intern_private("substring-range", Value::Native(prim_buffer_substring_range));
    ns.intern("buffer-list", Value::Native(prim_buffer_list));
    // Search primitives (belong to mora.buffer namespace)
    super::search::register(ns);
}

fn prim_buffer_name(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        let name = state
            .file_path
            .as_ref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "*scratch*".to_string());
        Ok(Value::string(name))
    })
}

fn prim_buffer_content(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::string(state.lines.join("\n"))))
}

fn prim_buffer_set_content(args: &[Value]) -> Result<Value, String> {
    let content = extract_string(args, 0)?;
    with_editor_state_mut(|state| {
        state.lines = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(String::from).collect()
        };
        state.modified = true;
        Ok(Value::Nil)
    })
}

fn prim_buffer_modified(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Bool(state.modified)))
}

fn prim_buffer_file_path(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| match &state.file_path {
        Some(p) => Ok(Value::string(p.clone())),
        None => Ok(Value::Nil),
    })
}

fn prim_buffer_line_count(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Int(state.lines.len() as i64)))
}

fn prim_buffer_current_line(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        let row = state.cursor_row;
        let line = state.lines.get(row).cloned().unwrap_or_default();
        Ok(Value::string(line))
    })
}

fn prim_buffer_line_at(args: &[Value]) -> Result<Value, String> {
    let row = extract_int(args, 0)? as usize;
    with_editor_state(|state| {
        let line = state.lines.get(row).cloned().unwrap_or_default();
        Ok(Value::string(line))
    })
}

fn prim_buffer_insert(args: &[Value]) -> Result<Value, String> {
    let text = extract_string(args, 0)?;
    with_editor_state_mut(|state| {
        let row = state.cursor_row;
        let col = state.cursor_col;
        if let Some(line) = state.lines.get_mut(row) {
            let insert_pos = col.min(line.len());
            line.insert_str(insert_pos, &text);
            state.cursor_col += text.len();
            state.modified = true;
        }
        Ok(Value::Nil)
    })
}

fn prim_buffer_replace_line(args: &[Value]) -> Result<Value, String> {
    let row = extract_int(args, 0)? as usize;
    let new_content = extract_string(args, 1)?;
    with_editor_state_mut(|state| {
        if let Some(line) = state.lines.get_mut(row) {
            *line = new_content;
            state.modified = true;
        }
        Ok(Value::Nil)
    })
}

/// (buffer-narrowed?) → is the buffer narrowed?
fn prim_buffer_narrowed(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Bool(state.narrow_start.is_some())))
}

/// (narrow-to-region start end) → narrow buffer to line range
fn prim_narrow_to_region(args: &[Value]) -> Result<Value, String> {
    let start = extract_int(args, 0)? as usize;
    let end = extract_int(args, 1)? as usize;
    with_editor_state_mut(|state| {
        let total = state.lines.len();
        state.narrow_start = Some(start.min(total.saturating_sub(1)));
        state.narrow_end = Some(end.min(total));
        Ok(Value::Nil)
    })
}

/// (widen) → remove narrowing
fn prim_widen(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        state.narrow_start = None;
        state.narrow_end = None;
        Ok(Value::Nil)
    })
}

/// (buffer-substring-range start-row start-col end-row end-col)
fn prim_buffer_substring_range(args: &[Value]) -> Result<Value, String> {
    let start_row = extract_int(args, 0)? as usize;
    let start_col = extract_int(args, 1)? as usize;
    let end_row = extract_int(args, 2)? as usize;
    let end_col = extract_int(args, 3)? as usize;
    with_editor_state(|state| {
        if start_row == end_row {
            if let Some(line) = state.lines.get(start_row) {
                let s = start_col.min(line.len());
                let e = end_col.min(line.len());
                return Ok(Value::string(&line[s..e]));
            }
            return Ok(Value::string(""));
        }
        let mut result = String::new();
        for row in start_row..=end_row.min(state.lines.len().saturating_sub(1)) {
            if let Some(line) = state.lines.get(row) {
                if row == start_row {
                    let s = start_col.min(line.len());
                    result.push_str(&line[s..]);
                } else if row == end_row {
                    let e = end_col.min(line.len());
                    result.push_str(&line[..e]);
                } else {
                    result.push_str(line);
                }
                if row < end_row {
                    result.push('\n');
                }
            }
        }
        Ok(Value::string(&result))
    })
}

/// (buffer-list) → list of buffer names (currently just current buffer)
fn prim_buffer_list(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        let name = state.file_path.as_deref().unwrap_or("*scratch*");
        Ok(Value::vector(vec![Value::string(name)]))
    })
}
