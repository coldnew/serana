use std::cell::RefCell;
use std::collections::HashMap;

use mora_lisp::eval::{EvalError, Evaluator};
use mora_lisp::types::Value;

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
    static COMMAND_REGISTRY: RefCell<HashMap<String, CommandEntry>> = RefCell::new(HashMap::new());
}

#[derive(Clone)]
struct CommandEntry {
    func: Value,
    doc: Option<String>,
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
    EDITOR_STATE.with(|s| s.borrow_mut().take())
}

pub struct MoraLispBridge {
    pub evaluator: Evaluator,
}

impl MoraLispBridge {
    pub fn new() -> Self {
        clear_command_registry();
        let mut evaluator = Evaluator::new();
        Self::register_editor_primitives(&mut evaluator);
        Self { evaluator }
    }

    fn register_editor_primitives(eval: &mut Evaluator) {
        let buffer_ns = eval.ns.find_or_create("mora.buffer");
        let cursor_ns = eval.ns.find_or_create("mora.cursor");
        let mode_ns = eval.ns.find_or_create("mora.mode");
        let editor_ns = eval.ns.find_or_create("mora.editor");
        let window_ns = eval.ns.find_or_create("mora.window");
        let hook_ns = eval.ns.find_or_create("mora.hook");
        let keymap_ns = eval.ns.find_or_create("mora.keymap");
        let overlay_ns = eval.ns.find_or_create("mora.overlay");
        let shell_ns = eval.ns.find_or_create("mora.shell");
        let ui_ns = eval.ns.find_or_create("mora.ui");
        let command_ns = eval.ns.find_or_create("mora.command");

        // Buffer operations
        let mut ns = buffer_ns.lock();
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
        ns.intern(
            "buffer-current-line",
            Value::Native(prim_buffer_current_line),
        );
        ns.intern_private("current-line", Value::Native(prim_buffer_current_line));
        ns.intern("buffer-line-at", Value::Native(prim_buffer_line_at));
        ns.intern_private("line-at", Value::Native(prim_buffer_line_at));
        ns.intern("buffer-insert!", Value::Native(prim_buffer_insert));
        ns.intern_private("insert!", Value::Native(prim_buffer_insert));
        ns.intern(
            "buffer-replace-line!",
            Value::Native(prim_buffer_replace_line),
        );
        ns.intern_private("replace-line!", Value::Native(prim_buffer_replace_line));
        drop(ns);

        // Cursor operations
        let mut ns = cursor_ns.lock();
        ns.intern("cursor-row", Value::Native(prim_cursor_row));
        ns.intern_private("row", Value::Native(prim_cursor_row));
        ns.intern("cursor-col", Value::Native(prim_cursor_col));
        ns.intern_private("col", Value::Native(prim_cursor_col));
        ns.intern("cursor-set!", Value::Native(prim_cursor_set));
        ns.intern_private("set!", Value::Native(prim_cursor_set));
        ns.intern("cursor-goto-line", Value::Native(prim_cursor_goto_line));
        ns.intern_private("goto-line", Value::Native(prim_cursor_goto_line));
        ns.intern("cursor-forward!", Value::Native(prim_cursor_forward));
        ns.intern_private("forward!", Value::Native(prim_cursor_forward));
        ns.intern("cursor-backward!", Value::Native(prim_cursor_backward));
        ns.intern_private("backward!", Value::Native(prim_cursor_backward));
        ns.intern("cursor-beginning-of-line", Value::Native(prim_cursor_bol));
        ns.intern_private("beginning-of-line", Value::Native(prim_cursor_bol));
        ns.intern("cursor-end-of-line", Value::Native(prim_cursor_eol));
        ns.intern_private("end-of-line", Value::Native(prim_cursor_eol));
        drop(ns);

        // Mode operations
        let mut ns = mode_ns.lock();
        ns.intern("current-mode", Value::Native(prim_current_mode));
        ns.intern_private("current", Value::Native(prim_current_mode));
        ns.intern("set-mode!", Value::Native(prim_set_mode));
        ns.intern_private("set!", Value::Native(prim_set_mode));
        ns.intern("set-minor-mode!", Value::Native(prim_set_minor_mode));
        ns.intern_private("toggle-minor!", Value::Native(prim_set_minor_mode));
        drop(ns);

        // Editor operations
        let mut ns = editor_ns.lock();
        ns.intern("editor-message", Value::Native(prim_editor_message));
        ns.intern_private("message", Value::Native(prim_editor_message));
        ns.intern("editor-quit", Value::Native(prim_editor_quit));
        ns.intern_private("quit", Value::Native(prim_editor_quit));
        ns.intern("editor-status", Value::Native(prim_editor_status));
        ns.intern_private("status", Value::Native(prim_editor_status));

        // Constants
        ns.intern("*mora-version*", Value::string(env!("CARGO_PKG_VERSION")));
        ns.intern("*newline*", Value::string("\n"));
        ns.intern("*tab*", Value::string("\t"));
        drop(ns);

        // Window operations
        let mut ns = window_ns.lock();
        ns.intern("window-count", Value::Native(prim_window_count));
        ns.intern_private("count", Value::Native(prim_window_count));
        drop(ns);

        // Hook system
        let mut ns = hook_ns.lock();
        ns.intern("add-hook", Value::Native(prim_add_hook));
        ns.intern_private("add", Value::Native(prim_add_hook));
        drop(ns);

        // Keybinding
        let mut ns = keymap_ns.lock();
        ns.intern("define-key", Value::Native(prim_define_key));
        ns.intern_private("define", Value::Native(prim_define_key));
        drop(ns);

        // Overlay operations
        let mut ns = overlay_ns.lock();
        ns.intern("make-overlay", Value::Native(prim_make_overlay));
        ns.intern_private("make", Value::Native(prim_make_overlay));
        ns.intern("overlay-put-face", Value::Native(prim_overlay_put_face));
        ns.intern_private("put-face", Value::Native(prim_overlay_put_face));
        ns.intern(
            "overlay-put-property",
            Value::Native(prim_overlay_put_property),
        );
        ns.intern_private("put-property", Value::Native(prim_overlay_put_property));
        ns.intern("overlay-delete", Value::Native(prim_overlay_delete));
        ns.intern_private("delete", Value::Native(prim_overlay_delete));
        ns.intern("overlay-get", Value::Native(prim_overlay_get));
        ns.intern_private("get", Value::Native(prim_overlay_get));
        ns.intern("overlays-at", Value::Native(prim_overlays_at));
        ns.intern_private("at", Value::Native(prim_overlays_at));
        ns.intern(
            "overlay-put-invisible",
            Value::Native(prim_overlay_put_invisible),
        );
        ns.intern_private("put-invisible", Value::Native(prim_overlay_put_invisible));
        ns.intern(
            "overlay-put-read-only",
            Value::Native(prim_overlay_put_read_only),
        );
        ns.intern_private("put-read-only", Value::Native(prim_overlay_put_read_only));
        drop(ns);

        // Shell operations
        let mut ns = shell_ns.lock();
        ns.intern("shell-command", Value::Native(prim_shell_command));
        ns.intern_private("command", Value::Native(prim_shell_command));
        ns.intern("shell-capture", Value::Native(prim_shell_capture));
        ns.intern_private("capture", Value::Native(prim_shell_capture));
        drop(ns);

        // UI DSL
        let mut ns = ui_ns.lock();
        ns.intern(
            "register-ui-builder",
            Value::Native(prim_register_ui_builder),
        );
        ns.intern_private("register-builder", Value::Native(prim_register_ui_builder));
        ns.intern("ui-builders", Value::Native(prim_ui_builders));
        ns.intern_private("builders", Value::Native(prim_ui_builders));
        drop(ns);

        // Command registry
        let mut ns = command_ns.lock();
        ns.intern("register!", Value::Native(prim_command_register));
        ns.intern("execute!", Value::Native(prim_command_execute));
        ns.intern("exists?", Value::Native(prim_command_exists));
        ns.intern("names", Value::Native(prim_command_names));
        ns.intern("doc", Value::Native(prim_command_doc));
        drop(ns);

        for ns_name in [
            "mora.buffer",
            "mora.cursor",
            "mora.mode",
            "mora.editor",
            "mora.window",
            "mora.hook",
            "mora.keymap",
            "mora.overlay",
            "mora.shell",
            "mora.ui",
            "mora.command",
        ] {
            eval.ns
                .refer_all(ns_name, "user")
                .expect("Mora host namespaces and user namespace exist");
        }
    }

    pub fn eval(&mut self, code: &str) -> Result<Value, EvalError> {
        let forms = self.evaluator.read_cached(code)?;
        let mut result = Value::Nil;
        for form in forms {
            result = self.evaluator.eval(&form)?;
        }
        Ok(result)
    }

    pub fn load_init_file(&mut self) {
        let home_init = dirs::home_dir().map(|h| h.join(".mora").join("init.mora"));
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
                                eprintln!(
                                    "Panic loading {}: init file caused a crash",
                                    path.display()
                                );
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

    pub fn command_names(&self) -> Vec<String> {
        command_names()
    }

    pub fn has_command(&self, name: &str) -> bool {
        resolve_command_name(name).is_some()
    }

    pub fn execute_command(&mut self, name: &str) -> Result<Option<Value>, EvalError> {
        let Some(name) = resolve_command_name(name) else {
            return Ok(None);
        };
        let entry = command_entry(&name)
            .ok_or_else(|| EvalError::Custom(format!("command not found: {}", name)))?;
        let value = match entry.func {
            Value::Fn(f) => self.evaluator.call_fn(f, vec![])?,
            Value::Native(f) => f(&[]).map_err(EvalError::Custom)?,
            other => {
                return Err(EvalError::NotAFunction(format!(
                    "command {} is {}",
                    name,
                    other.type_name()
                )))
            }
        };
        Ok(Some(value))
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

fn clear_command_registry() {
    COMMAND_REGISTRY.with(|registry| registry.borrow_mut().clear());
}

fn short_command_name(name: &str) -> &str {
    name.rsplit_once('/')
        .map(|(_, short)| short)
        .unwrap_or(name)
}

fn command_names() -> Vec<String> {
    COMMAND_REGISTRY.with(|registry| {
        let registry = registry.borrow();
        let mut names: Vec<String> = registry.keys().cloned().collect();
        let mut short_counts = HashMap::new();
        for name in registry.keys() {
            *short_counts
                .entry(short_command_name(name).to_string())
                .or_insert(0) += 1;
        }
        for name in registry.keys() {
            let short = short_command_name(name);
            if short_counts.get(short) == Some(&1) {
                names.push(short.to_string());
            }
        }
        names.sort();
        names.dedup();
        names
    })
}

fn command_entry(name: &str) -> Option<CommandEntry> {
    COMMAND_REGISTRY.with(|registry| registry.borrow().get(name).cloned())
}

fn resolve_command_name(name: &str) -> Option<String> {
    COMMAND_REGISTRY.with(|registry| {
        let registry = registry.borrow();
        if registry.contains_key(name) {
            return Some(name.to_string());
        }

        let mut matches = registry
            .keys()
            .filter(|candidate| short_command_name(candidate) == name);
        let first = matches.next()?.to_string();
        if matches.next().is_none() {
            Some(first)
        } else {
            None
        }
    })
}

fn command_doc(name: &str) -> Option<String> {
    resolve_command_name(name).and_then(|name| command_entry(&name).and_then(|entry| entry.doc))
}

// --- Buffer primitives ---

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
        let target = line
            .saturating_sub(1)
            .min(state.lines.len().saturating_sub(1));
        state.cursor_row = target;
        state.cursor_col = 0;
        Ok(Value::Nil)
    })
}

// --- Mode primitives ---

fn prim_current_mode(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::keyword(state.mode.clone())))
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

fn prim_set_minor_mode(args: &[Value]) -> Result<Value, String> {
    let mode = extract_string(args, 0)?;
    with_editor_state_mut(|state| {
        if let Some(pos) = state.minor_modes.iter().position(|m| m == &mode) {
            state.minor_modes.remove(pos);
        } else {
            state.minor_modes.push(mode);
        }
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
    let n = args
        .get(0)
        .and_then(|v| match v {
            Value::Int(n) => Some(*n as usize),
            _ => None,
        })
        .unwrap_or(1);
    with_editor_state_mut(|state| {
        state.cursor_col += n;
        Ok(Value::Nil)
    })
}

fn prim_cursor_backward(args: &[Value]) -> Result<Value, String> {
    let n = args
        .get(0)
        .and_then(|v| match v {
            Value::Int(n) => Some(*n as usize),
            _ => None,
        })
        .unwrap_or(1);
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
    let invisible = args
        .get(1)
        .and_then(|v| match v {
            Value::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(true);
    with_editor_state_mut(|state| {
        if let Some(ov) = state.overlays.get_mut(id) {
            ov.invisible = invisible;
        }
        Ok(Value::Nil)
    })
}

fn prim_overlay_put_read_only(args: &[Value]) -> Result<Value, String> {
    let id = extract_int(args, 0)? as usize;
    let read_only = args
        .get(1)
        .and_then(|v| match v {
            Value::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(true);
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
                format!(
                    "Shell command succeeded (exit {})",
                    output.status.code().unwrap_or(0)
                )
            } else {
                trimmed.clone()
            };
            Ok(Value::string(trimmed))
        })
    } else {
        let msg = if stderr.is_empty() {
            format!(
                "Command failed (exit {})",
                output.status.code().unwrap_or(-1)
            )
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
    with_editor_state(|state| Ok(Value::Int(state.ui_builders.len() as i64)))
}

// --- Command registry primitives ---

fn prim_command_register(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    let func = args
        .get(1)
        .cloned()
        .ok_or_else(|| "register! requires a command function".to_string())?;
    if !matches!(func, Value::Fn(_) | Value::Native(_)) {
        return Err(format!(
            "command function must be callable, got {}",
            func.type_name()
        ));
    }
    let doc = args.get(2).and_then(|v| match v {
        Value::String(s) => Some(s.to_string()),
        _ => None,
    });

    COMMAND_REGISTRY.with(|registry| {
        registry
            .borrow_mut()
            .insert(name.clone(), CommandEntry { func, doc });
    });
    Ok(Value::string(name))
}

fn prim_command_execute(args: &[Value]) -> Result<Value, String> {
    let requested = extract_string(args, 0)?;
    let name = resolve_command_name(&requested)
        .ok_or_else(|| format!("command not found or ambiguous: {}", requested))?;
    let entry = command_entry(&name).ok_or_else(|| format!("command not found: {}", name))?;

    mora_lisp::eval::with_evaluator(|eval| match entry.func {
        Value::Fn(f) => eval.call_fn(f, vec![]).map_err(|e| e.to_string()),
        Value::Native(f) => f(&[]),
        other => Err(format!("command {} is {}", name, other.type_name())),
    })
}

fn prim_command_exists(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    Ok(Value::Bool(resolve_command_name(&name).is_some()))
}

fn prim_command_names(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::vector(
        command_names().into_iter().map(Value::string).collect(),
    ))
}

fn prim_command_doc(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    Ok(command_doc(&name).map(Value::string).unwrap_or(Value::Nil))
}

// --- Lisp Value → UiNode converter ---

fn map_get_kw<'a>(
    map: &'a std::collections::HashMap<Value, Value>,
    key: &str,
) -> Option<&'a Value> {
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

fn apply_style_to_text(
    mut node: display_protocol::UiNode,
    style: &display_protocol::Style,
) -> display_protocol::UiNode {
    if *style != display_protocol::Style::default() {
        node = node.bold().dim(); // reset
        if style.bold {
            node = node.bold();
        }
        if style.dim {
            node = node.dim();
        }
        if style.italic {
            node = node.italic();
        }
        if style.underline {
            node = node.underline();
        }
        if let Some(fg) = style.fg {
            node = node.color(fg);
        }
        if let Some(bg) = style.bg {
            node = node.bg(bg);
        }
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
            let children: Vec<display_protocol::UiNode> =
                v.iter().map(lisp_value_to_uinode).collect();
            display_protocol::UiNode::column(children)
        }
        Value::Map(map) => {
            let node_type = map_get_str_kw(map, "type").unwrap_or_default();
            let style = map_get_kw(map, "style")
                .map(lisp_value_to_style)
                .unwrap_or_default();

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

fn extract_children(
    map: &std::collections::HashMap<Value, Value>,
) -> Vec<display_protocol::UiNode> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_primitives_remain_available_unqualified() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();

        bridge.eval("(editor-message \"legacy\")").unwrap();

        with_editor_state(|state| {
            assert_eq!(state.status_message, "legacy");
        });
        take_editor_state();
    }

    #[test]
    fn editor_primitives_are_available_through_namespace_aliases() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();

        bridge
            .eval(
                r#"
                (ns coldnew.init)
                (require [mora.editor :as editor])
                (require [mora.buffer :as buffer])
                (editor/message (str "Buffer: " (buffer/name)))
                "#,
            )
            .unwrap();

        with_editor_state(|state| {
            assert_eq!(state.status_message, "Buffer: *scratch*");
        });
        take_editor_state();
    }

    #[test]
    fn defcommand_registers_and_executes_editor_command() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();

        bridge
            .eval(
                r#"
                (ns coldnew.commands)
                (require [mora.editor :as editor])
                (defcommand say-hello
                  "Say hello from a user command."
                  []
                  (editor/message "hello from command"))
                "#,
            )
            .unwrap();

        assert!(bridge.has_command("say-hello"));
        assert!(bridge
            .command_names()
            .contains(&"coldnew.commands/say-hello".to_string()));
        assert_eq!(
            bridge.eval("(mora.command/doc \"say-hello\")").unwrap(),
            Value::string("Say hello from a user command.")
        );

        bridge.execute_command("say-hello").unwrap();
        with_editor_state(|state| {
            assert_eq!(state.status_message, "hello from command");
        });
        take_editor_state();
    }

    #[test]
    fn interactive_defn_registers_and_executes_editor_command() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();

        bridge
            .eval(
                r#"
                (ns coldnew.commands)
                (require [mora.editor :as editor])
                (defn say-hello
                  "Say hello through interactive defn."
                  []
                  (interactive)
                  (editor/message "hello from interactive defn"))
                "#,
            )
            .unwrap();

        assert!(bridge.has_command("say-hello"));
        assert_eq!(
            bridge.eval("(mora.command/doc \"say-hello\")").unwrap(),
            Value::string("Say hello through interactive defn.")
        );

        bridge.execute_command("say-hello").unwrap();
        with_editor_state(|state| {
            assert_eq!(state.status_message, "hello from interactive defn");
        });
        take_editor_state();
    }
}
