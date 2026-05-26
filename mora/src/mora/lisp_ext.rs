use std::cell::RefCell;

use mora_lisp::types::Value;
use mora_lisp::eval::{EvalError, Evaluator};

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

        // Constants
        ns.intern("*mora-version*", Value::string(env!("CARGO_PKG_VERSION")));
        ns.intern("*newline*", Value::string("\n"));
        ns.intern("*tab*", Value::string("\t"));
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

