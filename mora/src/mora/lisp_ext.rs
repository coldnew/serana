use std::cell::RefCell;

use mora_lisp::types::Value;
use mora_lisp::eval::{EvalError, Evaluator};

use super::overlay::{OverlayFace, OverlayStore};

/// Shared editor state accessible from Lisp primitives
pub struct EditorState {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub modified: bool,
    pub file_path: Option<String>,
    pub mode: String,
    pub status_message: String,
    pub quit_requested: bool,
    pub window_count: usize,
    pub minor_modes: Vec<String>,
    pub hooks: std::collections::HashMap<String, Vec<Value>>,
    pub keybindings: std::collections::HashMap<String, Value>,
    pub overlays: OverlayStore,
    pub ui_builders: Vec<Value>,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            modified: false,
            file_path: None,
            mode: "normal".to_string(),
            status_message: String::new(),
            quit_requested: false,
            window_count: 1,
            minor_modes: Vec::new(),
            hooks: std::collections::HashMap::new(),
            keybindings: std::collections::HashMap::new(),
            overlays: OverlayStore::new(),
            ui_builders: Vec::new(),
        }
    }
}

thread_local! {
    static EDITOR_STATE: RefCell<Option<EditorState>> = RefCell::new(None);
}

pub fn with_editor_state<R>(f: impl FnOnce(&EditorState) -> R) -> R {
    EDITOR_STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref().expect("No editor context available");
        f(state)
    })
}

pub fn with_editor_state_mut<R>(f: impl FnOnce(&mut EditorState) -> R) -> R {
    EDITOR_STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().expect("No editor context available");
        f(state)
    })
}

pub fn set_editor_state(state: EditorState) {
    EDITOR_STATE.with(|s| {
        *s.borrow_mut() = Some(state);
    });
}

pub fn take_editor_state() -> Option<EditorState> {
    EDITOR_STATE.with(|s| {
        s.borrow_mut().take()
    })
}

pub struct MoraLispBridge {
    pub evaluator: Evaluator,
}

impl MoraLispBridge {
    pub fn new() -> Self {
        let mut evaluator = Evaluator::new();
        Self::register_editor_primitives(&mut evaluator);
        Self { evaluator }
    }

    fn register_editor_primitives(eval: &mut Evaluator) {
        let ns = eval.ns.current();
        let mut ns = ns.lock();

        // Buffer operations
        ns.intern("buffer-name", Value::Native(prim_buffer_name));
        ns.intern("buffer-content", Value::Native(prim_buffer_content));
        ns.intern("buffer-set-content", Value::Native(prim_buffer_set_content));
        ns.intern("buffer-modified?", Value::Native(prim_buffer_modified));
        ns.intern("buffer-file-path", Value::Native(prim_buffer_file_path));
        ns.intern("buffer-line-count", Value::Native(prim_buffer_line_count));
        ns.intern("buffer-current-line", Value::Native(prim_buffer_current_line));
        ns.intern("buffer-line-at", Value::Native(prim_buffer_line_at));
        ns.intern("buffer-insert!", Value::Native(prim_buffer_insert));
        ns.intern("buffer-replace-line!", Value::Native(prim_buffer_replace_line));

        // Cursor operations
        ns.intern("cursor-row", Value::Native(prim_cursor_row));
        ns.intern("cursor-col", Value::Native(prim_cursor_col));
        ns.intern("cursor-set!", Value::Native(prim_cursor_set));
        ns.intern("cursor-goto-line", Value::Native(prim_cursor_goto_line));
        ns.intern("cursor-forward!", Value::Native(prim_cursor_forward));
        ns.intern("cursor-backward!", Value::Native(prim_cursor_backward));
        ns.intern("cursor-beginning-of-line", Value::Native(prim_cursor_bol));
        ns.intern("cursor-end-of-line", Value::Native(prim_cursor_eol));

        // Mode operations
        ns.intern("current-mode", Value::Native(prim_current_mode));
        ns.intern("set-mode!", Value::Native(prim_set_mode));

        // Editor operations
        ns.intern("editor-message", Value::Native(prim_editor_message));
        ns.intern("editor-quit", Value::Native(prim_editor_quit));
        ns.intern("editor-status", Value::Native(prim_editor_status));

        // Window operations
        ns.intern("window-count", Value::Native(prim_window_count));

        // Hook system
        ns.intern("add-hook", Value::Native(prim_add_hook));

        // Keybinding
        ns.intern("define-key", Value::Native(prim_define_key));

        // Overlay operations
        ns.intern("make-overlay", Value::Native(prim_make_overlay));
        ns.intern("overlay-put-face", Value::Native(prim_overlay_put_face));
        ns.intern("overlay-put-property", Value::Native(prim_overlay_put_property));
        ns.intern("overlay-delete", Value::Native(prim_overlay_delete));
        ns.intern("overlay-get", Value::Native(prim_overlay_get));
        ns.intern("overlays-at", Value::Native(prim_overlays_at));
        ns.intern("overlay-put-invisible", Value::Native(prim_overlay_put_invisible));
        ns.intern("overlay-put-read-only", Value::Native(prim_overlay_put_read_only));

        // Shell operations
        ns.intern("shell-command", Value::Native(prim_shell_command));
        ns.intern("shell-capture", Value::Native(prim_shell_capture));

        // Constants
        ns.intern("*mora-version*", Value::string(env!("CARGO_PKG_VERSION")));
        ns.intern("*newline*", Value::string("\n"));
        ns.intern("*tab*", Value::string("\t"));

        // UI DSL
        ns.intern("register-ui-builder", Value::Native(prim_register_ui_builder));
        ns.intern("ui-builders", Value::Native(prim_ui_builders));
    }

    pub fn eval(&mut self, code: &str) -> Result<Value, EvalError> {
        let forms = mora_lisp::reader::read_all(code)
            .map_err(|e| EvalError::Custom(format!("read error: {}", e)))?;
        let mut result = Value::Nil;
        for form in forms {
            result = self.evaluator.eval(&form)?;
        }
        Ok(result)
    }

    pub fn load_init_file(&mut self) {
        let home_init = dirs::home_dir()
            .map(|h| h.join(".mora").join("init.mora"));
        let local_init = std::path::PathBuf::from(".mora/init.mora");

        for path in [home_init, Some(local_init)].into_iter().flatten() {
            if path.exists() {
                match std::fs::read_to_string(&path) {
                    Ok(code) => {
                        // Use catch_unwind to prevent crashes from Lisp evaluation
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            self.eval(&code)
                        }));
                        match result {
                            Ok(Ok(_)) => {} // Success
                            Ok(Err(e)) => {
                                eprintln!("Error loading {}: {}", path.display(), e);
                            }
                            Err(_) => {
                                eprintln!("Panic loading {}: init file caused a crash", path.display());
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading {}: {}", path.display(), e);
                    }
                }
                break;
            }
        }
    }
}

impl Default for MoraLispBridge {
    fn default() -> Self {
        Self::new()
    }
}

// --- Helper functions ---

fn extract_string(args: &[Value], idx: usize) -> Result<String, String> {
    match args.get(idx) {
        Some(Value::String(s)) => Ok(s.to_string()),
        Some(v) => Err(format!("expected string, got {:?}", v)),
        None => Err("missing argument".to_string()),
    }
}

fn extract_int(args: &[Value], idx: usize) -> Result<i64, String> {
    match args.get(idx) {
        Some(Value::Int(n)) => Ok(*n),
        Some(v) => Err(format!("expected int, got {:?}", v)),
        None => Err("missing argument".to_string()),
    }
}

// --- Buffer primitives ---

fn prim_buffer_name(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        let name = state.file_path
            .as_ref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "*scratch*".to_string());
        Ok(Value::string(name))
    })
}

fn prim_buffer_content(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        Ok(Value::string(state.lines.join("\n")))
    })
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
    with_editor_state(|state| {
        match &state.file_path {
            Some(p) => Ok(Value::string(p.clone())),
            None => Ok(Value::Nil),
        }
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

// --- Cursor primitives ---

fn prim_cursor_row(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Int(state.cursor_row as i64)))
}

fn prim_cursor_col(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Int(state.cursor_col as i64)))
}

fn prim_cursor_set(args: &[Value]) -> Result<Value, String> {
    let row = extract_int(args, 0)? as usize;
    let col = extract_int(args, 1)? as usize;
    with_editor_state_mut(|state| {
        state.cursor_row = row.min(state.lines.len().saturating_sub(1));
        state.cursor_col = col;
        Ok(Value::Nil)
    })
}

fn prim_cursor_goto_line(args: &[Value]) -> Result<Value, String> {
    let line = extract_int(args, 0)? as usize;
    with_editor_state_mut(|state| {
        let target = line.saturating_sub(1).min(state.lines.len().saturating_sub(1));
        state.cursor_row = target;
        state.cursor_col = 0;
        Ok(Value::Nil)
    })
}

// --- Mode primitives ---

fn prim_current_mode(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        Ok(Value::keyword(state.mode.clone()))
    })
}

fn prim_set_mode(args: &[Value]) -> Result<Value, String> {
    let mode = match &args[0] {
        Value::Keyword(k) => k.name.to_string(),
        Value::String(s) => s.to_string(),
        _ => return Err("expected keyword or string".to_string()),
    };
    with_editor_state_mut(|state| {
        state.mode = mode;
        Ok(Value::Nil)
    })
}

// --- Editor primitives ---

fn prim_editor_message(args: &[Value]) -> Result<Value, String> {
    let msg = extract_string(args, 0)?;
    with_editor_state_mut(|state| {
        state.status_message = msg;
        Ok(Value::Nil)
    })
}

fn prim_editor_quit(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        state.quit_requested = true;
        Ok(Value::Nil)
    })
}

// --- Window primitives ---

fn prim_window_count(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Int(state.window_count as i64)))
}

// --- Hook primitives ---

fn prim_add_hook(args: &[Value]) -> Result<Value, String> {
    let hook_name = extract_string(args, 0)?;
    let handler = args.get(1).cloned().ok_or("missing hook handler")?;
    with_editor_state_mut(|state| {
        state.hooks.entry(hook_name).or_default().push(handler);
        Ok(Value::Nil)
    })
}

// --- Keybinding primitives ---

fn prim_define_key(args: &[Value]) -> Result<Value, String> {
    let key_desc = extract_string(args, 0)?;
    let action = args.get(1).cloned().ok_or("missing action")?;
    with_editor_state_mut(|state| {
        state.keybindings.insert(key_desc, action);
        Ok(Value::Nil)
    })
}

// --- Buffer editing primitives ---

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

// --- Cursor movement primitives ---

fn prim_cursor_forward(args: &[Value]) -> Result<Value, String> {
    let n = args.get(0).and_then(|v| match v {
        Value::Int(n) => Some(*n as usize),
        _ => None,
    }).unwrap_or(1);
    with_editor_state_mut(|state| {
        state.cursor_col += n;
        Ok(Value::Nil)
    })
}

fn prim_cursor_backward(args: &[Value]) -> Result<Value, String> {
    let n = args.get(0).and_then(|v| match v {
        Value::Int(n) => Some(*n as usize),
        _ => None,
    }).unwrap_or(1);
    with_editor_state_mut(|state| {
        state.cursor_col = state.cursor_col.saturating_sub(n);
        Ok(Value::Nil)
    })
}

fn prim_cursor_bol(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        state.cursor_col = 0;
        Ok(Value::Nil)
    })
}

fn prim_cursor_eol(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        if let Some(line) = state.lines.get(state.cursor_row) {
            state.cursor_col = line.len();
        }
        Ok(Value::Nil)
    })
}

// --- Editor status ---

fn prim_editor_status(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        let status = format!(
            "Mode: {} | Line: {} Col: {} | Modified: {} | File: {}",
            state.mode,
            state.cursor_row + 1,
            state.cursor_col + 1,
            state.modified,
            state.file_path.as_deref().unwrap_or("[scratch]")
        );
        Ok(Value::string(status))
    })
}

// --- Overlay primitives ---

fn prim_make_overlay(args: &[Value]) -> Result<Value, String> {
    let start = extract_int(args, 0)? as usize;
    let end = extract_int(args, 1)? as usize;
    with_editor_state_mut(|state| {
        let id = state.overlays.add(start, end);
        Ok(Value::Int(id as i64))
    })
}

fn prim_overlay_put_face(args: &[Value]) -> Result<Value, String> {
    let id = extract_int(args, 0)? as usize;
    let fg = args.get(1).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        _ => None,
    });
    let bg = args.get(2).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        _ => None,
    });
    let bold = args.get(3).and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        _ => None,
    });
    with_editor_state_mut(|state| {
        if let Some(ov) = state.overlays.get_mut(id) {
            let mut face = OverlayFace::new();
            if let Some(fg_str) = fg {
                face.fg = parse_color(&fg_str);
            }
            if let Some(bg_str) = bg {
                face.bg = parse_color(&bg_str);
            }
            face.bold = bold;
            ov.face = Some(face);
        }
        Ok(Value::Nil)
    })
}

fn prim_overlay_put_property(args: &[Value]) -> Result<Value, String> {
    let id = extract_int(args, 0)? as usize;
    let key = extract_string(args, 1)?;
    let val = extract_string(args, 2)?;
    with_editor_state_mut(|state| {
        if let Some(ov) = state.overlays.get_mut(id) {
            ov.properties.insert(key, val);
        }
        Ok(Value::Nil)
    })
}

fn prim_overlay_delete(args: &[Value]) -> Result<Value, String> {
    let id = extract_int(args, 0)? as usize;
    with_editor_state_mut(|state| {
        state.overlays.remove(id);
        Ok(Value::Nil)
    })
}

fn prim_overlay_get(args: &[Value]) -> Result<Value, String> {
    let id = extract_int(args, 0)? as usize;
    let key = extract_string(args, 1)?;
    with_editor_state(|state| {
        if let Some(ov) = state.overlays.get(id) {
            if let Some(val) = ov.properties.get(&key) {
                Ok(Value::string(val.clone()))
            } else {
                Ok(Value::Nil)
            }
        } else {
            Ok(Value::Nil)
        }
    })
}

fn prim_overlays_at(args: &[Value]) -> Result<Value, String> {
    let pos = extract_int(args, 0)? as usize;
    with_editor_state(|state| {
        let overlays = state.overlays.overlays_at(pos);
        let ids: Vec<Value> = overlays.iter().map(|o| Value::Int(o.id as i64)).collect();
        Ok(Value::Vector(std::sync::Arc::new(ids)))
    })
}

fn prim_overlay_put_invisible(args: &[Value]) -> Result<Value, String> {
    let id = extract_int(args, 0)? as usize;
    let invisible = args.get(1).and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        _ => None,
    }).unwrap_or(true);
    with_editor_state_mut(|state| {
        if let Some(ov) = state.overlays.get_mut(id) {
            ov.invisible = invisible;
        }
        Ok(Value::Nil)
    })
}

fn prim_overlay_put_read_only(args: &[Value]) -> Result<Value, String> {
    let id = extract_int(args, 0)? as usize;
    let read_only = args.get(1).and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        _ => None,
    }).unwrap_or(true);
    with_editor_state_mut(|state| {
        if let Some(ov) = state.overlays.get_mut(id) {
            ov.read_only = read_only;
        }
        Ok(Value::Nil)
    })
}

// --- Shell primitives ---

fn prim_shell_command(args: &[Value]) -> Result<Value, String> {
    let cmd = extract_string(args, 0)?;
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .output()
        .map_err(|e| format!("failed to run command: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if output.status.success() {
        let trimmed = stdout.trim_end_matches('\n').to_string();
        with_editor_state_mut(|state| {
            state.status_message = if trimmed.is_empty() {
                format!("Shell command succeeded (exit {})", output.status.code().unwrap_or(0))
            } else {
                trimmed.clone()
            };
            Ok(Value::string(trimmed))
        })
    } else {
        let msg = if stderr.is_empty() {
            format!("Command failed (exit {})", output.status.code().unwrap_or(-1))
        } else {
            stderr.trim_end_matches('\n').to_string()
        };
        with_editor_state_mut(|state| {
            state.status_message = msg.clone();
            Ok(Value::string(msg))
        })
    }
}

fn prim_shell_capture(args: &[Value]) -> Result<Value, String> {
    let cmd = extract_string(args, 0)?;
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .output()
        .map_err(|e| format!("failed to run command: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(Value::string(stdout.trim_end_matches('\n').to_string()))
}

// --- UI DSL primitives ---

fn prim_register_ui_builder(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("register-ui-builder requires 1 argument (a function)".to_string());
    }
    let builder = args[0].clone();
    with_editor_state_mut(|state| {
        state.ui_builders.push(builder);
        Ok(Value::Nil)
    })
}

fn prim_ui_builders(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        Ok(Value::Int(state.ui_builders.len() as i64))
    })
}

// --- Lisp Value → UiNode converter ---

fn map_get_kw<'a>(map: &'a std::collections::HashMap<Value, Value>, key: &str) -> Option<&'a Value> {
    map.get(&Value::keyword(key))
}

fn map_get_str_kw(map: &std::collections::HashMap<Value, Value>, key: &str) -> Option<String> {
    map_get_kw(map, key).and_then(|v| match v {
        Value::String(s) => Some(s.to_string()),
        Value::Keyword(k) => Some(k.name.to_string()),
        _ => None,
    })
}

fn map_get_bool_kw(map: &std::collections::HashMap<Value, Value>, key: &str) -> Option<bool> {
    map_get_kw(map, key).and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        _ => None,
    })
}

fn map_get_u16_kw(map: &std::collections::HashMap<Value, Value>, key: &str) -> Option<u16> {
    map_get_kw(map, key).and_then(|v| match v {
        Value::Int(i) => Some(*i as u16),
        _ => None,
    })
}

fn map_get_f64_kw(map: &std::collections::HashMap<Value, Value>, key: &str) -> Option<f64> {
    map_get_kw(map, key).and_then(|v| match v {
        Value::Float(f) => Some(*f),
        Value::Int(i) => Some(*i as f64),
        _ => None,
    })
}

fn parse_color_str(s: &str) -> display_protocol::Color {
    match parse_color(s) {
        Some(c) => display_protocol::Color::new(c.r, c.g, c.b),
        None => display_protocol::Color::WHITE,
    }
}

fn lisp_value_to_style(val: &Value) -> display_protocol::Style {
    let mut style = display_protocol::Style::default();
    if let Value::Map(map) = val {
        if let Some(fg) = map_get_str_kw(map, "fg") {
            style.fg = Some(parse_color_str(&fg));
        }
        if let Some(color) = map_get_str_kw(map, "color") {
            style.fg = Some(parse_color_str(&color));
        }
        if let Some(bg) = map_get_str_kw(map, "bg") {
            style.bg = Some(parse_color_str(&bg));
        }
        if let Some(b) = map_get_bool_kw(map, "bold") {
            style.bold = b;
        }
        if let Some(b) = map_get_bool_kw(map, "italic") {
            style.italic = b;
        }
        if let Some(b) = map_get_bool_kw(map, "underline") {
            style.underline = b;
        }
        if let Some(b) = map_get_bool_kw(map, "dim") {
            style.dim = b;
        }
        if let Some(b) = map_get_bool_kw(map, "strikethrough") {
            style.strikethrough = b;
        }
        if let Some(b) = map_get_bool_kw(map, "reverse") {
            style.reverse = b;
        }
        if let Some(b) = map_get_bool_kw(map, "blink") {
            style.blink = b;
        }
    }
    style
}

fn apply_style_to_text(mut node: display_protocol::UiNode, style: &display_protocol::Style) -> display_protocol::UiNode {
    if *style != display_protocol::Style::default() {
        node = node.bold().dim(); // reset
        if style.bold { node = node.bold(); }
        if style.dim { node = node.dim(); }
        if style.italic { node = node.italic(); }
        if style.underline { node = node.underline(); }
        if let Some(fg) = style.fg { node = node.color(fg); }
        if let Some(bg) = style.bg { node = node.bg(bg); }
    }
    node
}

pub fn lisp_value_to_uinode(val: &Value) -> display_protocol::UiNode {
    match val {
        Value::Nil => display_protocol::UiNode::None,
        Value::Bool(b) => display_protocol::UiNode::text(b.to_string()),
        Value::Int(i) => display_protocol::UiNode::text(i.to_string()),
        Value::Float(f) => display_protocol::UiNode::text(f.to_string()),
        Value::String(s) => display_protocol::UiNode::text(s.to_string()),
        Value::Vector(v) => {
            let children: Vec<display_protocol::UiNode> = v.iter().map(lisp_value_to_uinode).collect();
            display_protocol::UiNode::column(children)
        }
        Value::Map(map) => {
            let node_type = map_get_str_kw(map, "type").unwrap_or_default();
            let style = map_get_kw(map, "style").map(lisp_value_to_style).unwrap_or_default();

            match node_type.as_str() {
                "text" => {
                    let content = map_get_str_kw(map, "content").unwrap_or_default();
                    let mut node = display_protocol::UiNode::text(content);
                    node = apply_style_to_text(node, &style);
                    node
                }
                "span" => {
                    let content = map_get_str_kw(map, "content").unwrap_or_default();
                    display_protocol::UiNode::span(content, style)
                }
                "column" => {
                    let children = extract_children(map);
                    let mut node = display_protocol::UiNode::column(children);
                    node = apply_flex_props(node, map);
                    node
                }
                "row" => {
                    let children = extract_children(map);
                    let mut node = display_protocol::UiNode::row(children);
                    node = apply_flex_props(node, map);
                    node
                }
                "box" => {
                    let children = extract_children(map);
                    let mut node = display_protocol::UiNode::boxed(children);
                    node = apply_box_props(node, map);
                    node
                }
                "divider" => {
                    let node = display_protocol::UiNode::divider();
                    if let Some(ch) = map_get_str_kw(map, "char").and_then(|s| s.chars().next()) {
                        match node {
                            display_protocol::UiNode::Divider(d) => {
                                display_protocol::UiNode::Divider(d.char(ch))
                            }
                            other => other,
                        }
                    } else {
                        node
                    }
                }
                "progress" => {
                    let value = map_get_f64_kw(map, "value").unwrap_or(0.0) as f32;
                    let max = map_get_f64_kw(map, "max").unwrap_or(100.0) as f32;
                    display_protocol::UiNode::progress(value, max)
                }
                "list" => {
                    let children = extract_children(map);
                    display_protocol::UiNode::list(children)
                }
                "show" => {
                    let when = map_get_bool_kw(map, "when").unwrap_or(false);
                    let child = map_get_kw(map, "child")
                        .map(lisp_value_to_uinode)
                        .unwrap_or(display_protocol::UiNode::None);
                    display_protocol::UiNode::show(when, child)
                }
                "for" => {
                    let children = if let Some(coll) = map_get_kw(map, "coll") {
                        let func = map_get_kw(map, "func");
                        match (func, coll) {
                            (Some(_f), Value::Vector(v)) | (Some(_f), Value::List(v)) => {
                                // Simplified: convert items directly (calling fn requires evaluator)
                                v.iter().map(lisp_value_to_uinode).collect()
                            }
                            _ => extract_children(map),
                        }
                    } else {
                        extract_children(map)
                    };
                    display_protocol::UiNode::For { children }
                }
                "scroll-view" => {
                    let child = map_get_kw(map, "child")
                        .map(lisp_value_to_uinode)
                        .unwrap_or(display_protocol::UiNode::None);
                    let mut scroll_top = 0u16;
                    let mut height = 10u16;
                    if let Some(props) = map_get_kw(map, "props").and_then(|v| match v {
                        Value::Map(m) => Some(m.as_ref()),
                        _ => None,
                    }) {
                        scroll_top = map_get_u16_kw(props, "scroll-top").unwrap_or(0);
                        height = map_get_u16_kw(props, "height").unwrap_or(10);
                    }
                    display_protocol::UiNode::ScrollView(display_protocol::ScrollNode {
                        child: Box::new(child),
                        scroll_top,
                        height,
                    })
                }
                "button" => {
                    let content = map_get_str_kw(map, "content").unwrap_or_default();
                    let mut node = display_protocol::UiNode::text(content);
                    node = apply_style_to_text(node, &style);
                    node
                }
                _ => display_protocol::UiNode::None,
            }
        }
        _ => display_protocol::UiNode::text(format!("{:?}", val)),
    }
}

fn extract_children(map: &std::collections::HashMap<Value, Value>) -> Vec<display_protocol::UiNode> {
    match map_get_kw(map, "children") {
        Some(Value::Vector(v)) => v.iter().map(lisp_value_to_uinode).collect(),
        Some(Value::List(v)) => v.iter().map(lisp_value_to_uinode).collect(),
        _ => vec![],
    }
}

fn apply_flex_props(
    node: display_protocol::UiNode,
    map: &std::collections::HashMap<Value, Value>,
) -> display_protocol::UiNode {
    let mut node = node;
    if let Some(props) = map_get_kw(map, "props").and_then(|v| match v {
        Value::Map(m) => Some(m.as_ref()),
        _ => None,
    }) {
        if let Some(gap) = map_get_u16_kw(props, "gap") {
            node = node.gap(gap);
        }
        if let Some(align) = map_get_str_kw(props, "align") {
            node = node.align(match align.as_str() {
                "center" => display_protocol::Align::Center,
                "end" => display_protocol::Align::End,
                "stretch" => display_protocol::Align::Stretch,
                _ => display_protocol::Align::Start,
            });
        }
        if let Some(justify) = map_get_str_kw(props, "justify") {
            node = node.justify(match justify.as_str() {
                "center" => display_protocol::Justify::Center,
                "end" => display_protocol::Justify::End,
                "space-between" => display_protocol::Justify::SpaceBetween,
                "space-around" => display_protocol::Justify::SpaceAround,
                _ => display_protocol::Justify::Start,
            });
        }
        if let Some(w) = map_get_u16_kw(props, "width") {
            node = node.width(w);
        }
        if let Some(h) = map_get_u16_kw(props, "height") {
            node = node.height(h);
        }
        if let Some(fg) = map_get_f64_kw(props, "flex-grow") {
            node = node.flex_grow(fg as f32);
        }
    }
    node
}

fn apply_box_props(
    node: display_protocol::UiNode,
    map: &std::collections::HashMap<Value, Value>,
) -> display_protocol::UiNode {
    let mut node = node;
    if let Some(props) = map_get_kw(map, "props").and_then(|v| match v {
        Value::Map(m) => Some(m.as_ref()),
        _ => None,
    }) {
        if let Some(title) = map_get_str_kw(props, "title") {
            node = node.title(title);
        }
        if let Some(w) = map_get_u16_kw(props, "width") {
            node = node.width(w);
        }
        if let Some(h) = map_get_u16_kw(props, "height") {
            node = node.height(h);
        }
        if let Some(b) = map_get_bool_kw(props, "border") {
            if b {
                node = node.border(display_protocol::Border::all(None));
            }
        }
    }
    node
}

/// Build UI with Lisp builder hooks. Returns Lisp-generated UiNode if any builder is registered.
pub fn build_lisp_ui(width: u16, height: u16) -> Option<display_protocol::UiNode> {
    let builders: Vec<Value> = with_editor_state(|state| state.ui_builders.clone());

    if builders.is_empty() {
        return None;
    }

    let mut nodes = Vec::new();
    for builder_fn in &builders {
        let args = vec![Value::Int(width as i64), Value::Int(height as i64)];
        let result = match builder_fn {
            Value::Native(f) => f(&args),
            _ => Err("ui-builder is not a native function".to_string()),
        };

        match result {
            Ok(val) => {
                nodes.push(lisp_value_to_uinode(&val));
            }
            Err(_) => {}
        }
    }

    if nodes.is_empty() {
        None
    } else if nodes.len() == 1 {
        Some(nodes.into_iter().next().unwrap())
    } else {
        Some(display_protocol::UiNode::column(nodes))
    }
}

fn parse_color(s: &str) -> Option<super::display::style::MoraColor> {
    let s = s.trim();
    if s.starts_with('#') && s.len() == 7 {
        let r = u8::from_str_radix(&s[1..3], 16).ok()?;
        let g = u8::from_str_radix(&s[3..5], 16).ok()?;
        let b = u8::from_str_radix(&s[5..7], 16).ok()?;
        Some(super::display::style::MoraColor::new(r, g, b))
    } else {
        match s.to_lowercase().as_str() {
            "red" => Some(super::display::style::MoraColor::new(255, 0, 0)),
            "green" => Some(super::display::style::MoraColor::new(0, 255, 0)),
            "blue" => Some(super::display::style::MoraColor::new(0, 0, 255)),
            "yellow" => Some(super::display::style::MoraColor::new(255, 255, 0)),
            "cyan" => Some(super::display::style::MoraColor::new(0, 255, 255)),
            "magenta" => Some(super::display::style::MoraColor::new(255, 0, 255)),
            "white" => Some(super::display::style::MoraColor::new(255, 255, 255)),
            "black" => Some(super::display::style::MoraColor::new(0, 0, 0)),
            "orange" => Some(super::display::style::MoraColor::new(255, 165, 0)),
            "gray" | "grey" => Some(super::display::style::MoraColor::new(128, 128, 128)),
            _ => None,
        }
    }
}

