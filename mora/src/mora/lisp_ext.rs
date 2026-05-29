use std::cell::RefCell;
use std::collections::HashMap;

use crate::lisp::eval::{EvalError, Evaluator};
use crate::lisp::types::Value;

use super::overlay::{OverlayFace, OverlayStore};
use super::undo_tree::{Snapshot, UndoTree};

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
    // --- Emacs-like state ---
    /// Buffer-local variables: var_name -> value
    pub buffer_local_vars: std::collections::HashMap<String, Value>,
    /// Mark ring: stack of saved cursor positions
    pub mark_ring: Vec<(usize, usize)>,
    /// Whether mark is active (region is selected)
    pub mark_active: bool,
    /// Mark position when active
    pub mark_pos: Option<(usize, usize)>,
    /// Kill ring entries
    pub kill_ring: Vec<String>,
    /// Kill ring index for yank-pop cycling
    pub kill_ring_idx: usize,
    /// Registers: char -> string value
    pub registers: std::collections::HashMap<char, String>,
    /// Narrowing range (None = not narrowed)
    pub narrow_start: Option<usize>,
    pub narrow_end: Option<usize>,
    /// Undo enabled flag
    pub undo_enabled: bool,
    /// Undo tree for branching history
    pub undo_tree: UndoTree,
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
            buffer_local_vars: std::collections::HashMap::new(),
            mark_ring: Vec::new(),
            mark_active: false,
            mark_pos: None,
            kill_ring: Vec::new(),
            kill_ring_idx: 0,
            registers: std::collections::HashMap::new(),
            narrow_start: None,
            narrow_end: None,
            undo_enabled: true,
            undo_tree: UndoTree::new(Snapshot {
                lines: vec![String::new()],
                cursor_row: 0,
                cursor_col: 0,
            }),
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
        let kill_ring_ns = eval.ns.find_or_create("mora.kill-ring");
        let mark_ns = eval.ns.find_or_create("mora.mark");
        let register_ns = eval.ns.find_or_create("mora.register");
        let var_ns = eval.ns.find_or_create("mora.var");
        let minibuffer_ns = eval.ns.find_or_create("mora.minibuffer");
        let region_ns = eval.ns.find_or_create("mora.region");
        let undo_ns = eval.ns.find_or_create("mora.undo");
        let tramp_ns = eval.ns.find_or_create("mora.tramp");
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
        // Kill ring operations
        let mut ns = kill_ring_ns.lock();
        ns.intern("kill-ring-yank", Value::Native(prim_kill_ring_yank));
        ns.intern_private("yank", Value::Native(prim_kill_ring_yank));
        ns.intern("kill-ring-push", Value::Native(prim_kill_ring_push));
        ns.intern_private("push", Value::Native(prim_kill_ring_push));
        ns.intern("kill-ring-pop", Value::Native(prim_kill_ring_pop));
        ns.intern_private("pop", Value::Native(prim_kill_ring_pop));
        ns.intern("kill-ring-pop-back", Value::Native(prim_kill_ring_pop_back));
        ns.intern_private("pop-back", Value::Native(prim_kill_ring_pop_back));
        ns.intern("kill-ring-count", Value::Native(prim_kill_ring_count));
        ns.intern_private("count", Value::Native(prim_kill_ring_count));
        ns.intern("kill-ring-contents", Value::Native(prim_kill_ring_contents));
        ns.intern_private("contents", Value::Native(prim_kill_ring_contents));
        drop(ns);
        // Mark ring operations
        let mut ns = mark_ns.lock();
        ns.intern("set-mark", Value::Native(prim_set_mark));
        ns.intern_private("set", Value::Native(prim_set_mark));
        ns.intern("goto-mark", Value::Native(prim_goto_mark));
        ns.intern_private("goto", Value::Native(prim_goto_mark));
        ns.intern("pop-mark", Value::Native(prim_pop_mark));
        ns.intern_private("pop", Value::Native(prim_pop_mark));
        ns.intern("mark-active?", Value::Native(prim_mark_active));
        ns.intern_private("active?", Value::Native(prim_mark_active));
        ns.intern("mark-position", Value::Native(prim_mark_position));
        ns.intern_private("position", Value::Native(prim_mark_position));
        ns.intern("deactivate-mark", Value::Native(prim_deactivate_mark));
        ns.intern_private("deactivate", Value::Native(prim_deactivate_mark));
        drop(ns);
        // Register operations
        let mut ns = register_ns.lock();
        ns.intern("register-set", Value::Native(prim_register_set));
        ns.intern_private("set", Value::Native(prim_register_set));
        ns.intern("register-get", Value::Native(prim_register_get));
        ns.intern_private("get", Value::Native(prim_register_get));
        ns.intern("register-list", Value::Native(prim_register_list));
        ns.intern_private("list", Value::Native(prim_register_list));
        drop(ns);
        // Buffer-local variable operations
        let mut ns = var_ns.lock();
        ns.intern("var-set", Value::Native(prim_var_set));
        ns.intern_private("set", Value::Native(prim_var_set));
        ns.intern("var-get", Value::Native(prim_var_get));
        ns.intern_private("get", Value::Native(prim_var_get));
        ns.intern("var-local", Value::Native(prim_var_local));
        ns.intern_private("local", Value::Native(prim_var_local));
        ns.intern("var-bound?", Value::Native(prim_var_bound));
        ns.intern_private("bound?", Value::Native(prim_var_bound));
        drop(ns);
        // Minibuffer operations
        let mut ns = minibuffer_ns.lock();
        ns.intern("read-string", Value::Native(prim_read_string));
        ns.intern("completing-read", Value::Native(prim_completing_read));
        ns.intern("y-or-n?", Value::Native(prim_y_or_n));
        drop(ns);
        let mut ns = region_ns.lock();
        ns.intern("region-beginning", Value::Native(prim_region_beginning));
        ns.intern_private("beginning", Value::Native(prim_region_beginning));
        ns.intern("region-end", Value::Native(prim_region_end));
        ns.intern_private("end", Value::Native(prim_region_end));
        ns.intern("region-active?", Value::Native(prim_region_active));
        ns.intern_private("active?", Value::Native(prim_region_active));
        ns.intern("delete-region", Value::Native(prim_delete_region));
        ns.intern_private("delete", Value::Native(prim_delete_region));
        ns.intern("buffer-substring", Value::Native(prim_buffer_substring));
        ns.intern_private("substring", Value::Native(prim_buffer_substring));
        drop(ns);
        // Undo operations
        let mut ns = undo_ns.lock();
        ns.intern("undo", Value::Native(prim_undo));
        ns.intern("redo", Value::Native(prim_redo));
        ns.intern("undo-boundary", Value::Native(prim_undo_boundary));
        ns.intern_private("boundary", Value::Native(prim_undo_boundary));
        ns.intern("undo-enabled?", Value::Native(prim_undo_enabled));
        ns.intern_private("enabled?", Value::Native(prim_undo_enabled));
        ns.intern("undo-tree-branches", Value::Native(prim_undo_tree_branches));
        ns.intern("undo-tree-switch-branch", Value::Native(prim_undo_tree_switch_branch));
        ns.intern("undo-tree-visualize", Value::Native(prim_undo_tree_visualize));
        ns.intern("undo-tree-node-count", Value::Native(prim_undo_tree_node_count));
        ns.intern("undo-tree-can-undo?", Value::Native(prim_undo_tree_can_undo));
        ns.intern("undo-tree-can-redo?", Value::Native(prim_undo_tree_can_redo));
        drop(ns);
        // Expanded hook operations (add to existing hook namespace)
        let mut ns = hook_ns.lock();
        ns.intern("remove-hook", Value::Native(prim_remove_hook));
        ns.intern_private("remove", Value::Native(prim_remove_hook));
        ns.intern("run-hook", Value::Native(prim_run_hook));
        ns.intern_private("run", Value::Native(prim_run_hook));
        ns.intern("hook-bound?", Value::Native(prim_hook_bound));
        ns.intern_private("bound?", Value::Native(prim_hook_bound));
        ns.intern("hooks-for", Value::Native(prim_hooks_for));
        ns.intern_private("for", Value::Native(prim_hooks_for));
        drop(ns);
        // Expanded buffer operations (add to existing buffer namespace)
        let mut ns = buffer_ns.lock();
        ns.intern("buffer-narrowed?", Value::Native(prim_buffer_narrowed));
        ns.intern_private("narrowed?", Value::Native(prim_buffer_narrowed));
        ns.intern("narrow-to-region", Value::Native(prim_narrow_to_region));
        ns.intern("widen", Value::Native(prim_widen));
        ns.intern("buffer-substring", Value::Native(prim_buffer_substring_range));
        ns.intern_private("substring-range", Value::Native(prim_buffer_substring_range));
        ns.intern("search-forward", Value::Native(prim_search_forward));
        ns.intern("search-backward", Value::Native(prim_search_backward));
        ns.intern("looking-at", Value::Native(prim_looking_at));
        ns.intern("buffer-list", Value::Native(prim_buffer_list));
        drop(ns);
        // TRAMP remote file operations
        let mut ns = tramp_ns.lock();
        ns.intern("tramp-read-file", Value::Native(prim_tramp_read_file));
        ns.intern("tramp-write-file", Value::Native(prim_tramp_write_file));
        ns.intern("tramp-shell-command", Value::Native(prim_tramp_shell_command));
        ns.intern("tramp-shell-capture", Value::Native(prim_tramp_shell_capture));
        ns.intern("tramp-exists?", Value::Native(prim_tramp_file_exists));
        ns.intern("tramp-mtime", Value::Native(prim_tramp_file_mtime));
        ns.intern("tramp-list-dir", Value::Native(prim_tramp_list_dir));
        ns.intern("tramp-mkdir", Value::Native(prim_tramp_mkdir));
        ns.intern("tramp-delete-file", Value::Native(prim_tramp_delete_file));
        ns.intern("tramp-rename-file", Value::Native(prim_tramp_rename_file));
        ns.intern("tramp-ping", Value::Native(prim_tramp_ping));
        ns.intern("tramp-connect", Value::Native(prim_tramp_connect));
        ns.intern("tramp-disconnect", Value::Native(prim_tramp_disconnect));
        ns.intern("tramp-connections", Value::Native(prim_tramp_connections));
        ns.intern("tramp-parse-path", Value::Native(prim_tramp_parse_path));
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
            "mora.kill-ring",
            "mora.mark",
            "mora.register",
            "mora.var",
            "mora.minibuffer",
            "mora.region",
            "mora.undo",
            "mora.tramp",
        ] {
            eval.ns
                .refer_all(ns_name, "user")
                .expect("Mora host namespaces and user namespace exist");
        }
    }
    pub fn eval(&mut self, code: &str) -> Result<Value, EvalError> {
        let forms = self.evaluator.read_cached(code)?;
        Ok(crate::lisp::vm::compile_and_run(&mut self.evaluator, &forms)?)
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

    crate::lisp::eval::with_evaluator(|eval| match entry.func {
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
                        scroll_y: scroll_top as u32,
                        scroll_x: 0,
                        viewport_width: 80,
                        viewport_height: height,
                        content_height: None,
                        content_width: None,
                        virtual_scroll: false,
                        scroll_policy: display_protocol::ScrollPolicy::Auto,
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
// --- Kill Ring primitives ---
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
// --- Mark Ring primitives ---
/// (set-mark) → set mark at current cursor position
fn prim_set_mark(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        let pos = (state.cursor_row, state.cursor_col);
        state.mark_active = true;
        state.mark_pos = Some(pos);
        state.mark_ring.push(pos);
        // Keep ring bounded
        if state.mark_ring.len() > 16 {
            state.mark_ring.remove(0);
        }
        Ok(Value::Nil)
    })
}
/// (goto-mark) → move cursor to mark position
fn prim_goto_mark(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        if let Some((row, col)) = state.mark_pos {
            state.cursor_row = row;
            state.cursor_col = col;
            Ok(Value::Nil)
        } else if let Some(&(row, col)) = state.mark_ring.last() {
            state.cursor_row = row;
            state.cursor_col = col;
            Ok(Value::Nil)
        } else {
            Err("no mark set".to_string())
        }
    })
}
/// (pop-mark) → pop mark ring, move cursor to previous mark
fn prim_pop_mark(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        if let Some((row, col)) = state.mark_ring.pop() {
            state.cursor_row = row;
            state.cursor_col = col;
            state.mark_pos = Some((row, col));
            Ok(Value::Bool(true))
        } else {
            Ok(Value::Bool(false))
        }
    })
}
/// (mark-active?) → is mark currently active?
fn prim_mark_active(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Bool(state.mark_active)))
}
/// (mark-position) → get current mark position [row col] or nil
fn prim_mark_position(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        match state.mark_pos {
            Some((row, col)) => Ok(Value::vector(vec![
                Value::Int(row as i64),
                Value::Int(col as i64),
            ])),
            None => Ok(Value::Nil),
        }
    })
}
/// (deactivate-mark) → deactivate the mark
fn prim_deactivate_mark(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        state.mark_active = false;
        Ok(Value::Nil)
    })
}
// --- Register primitives ---
/// (register-set ?char "value") → store value in named register
fn prim_register_set(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    let ch = name.chars().next().ok_or("register name must be a char")?;
    let value = extract_string(args, 1)?;
    with_editor_state_mut(|state| {
        state.registers.insert(ch, value);
        Ok(Value::Nil)
    })
}
/// (register-get ?char) → retrieve value from named register
fn prim_register_get(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    let ch = name.chars().next().ok_or("register name must be a char")?;
    with_editor_state(|state| {
        match state.registers.get(&ch) {
            Some(val) => Ok(Value::string(val)),
            None => Ok(Value::Nil),
        }
    })
}
/// (register-list) → map of all register names to values
fn prim_register_list(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        let pairs: Vec<(Value, Value)> = state.registers.iter()
            .map(|(ch, val)| (Value::keyword(ch.to_string()), Value::string(val)))
            .collect();
        Ok(Value::map(pairs))
    })
}
// --- Buffer-local Variable primitives ---
/// (var-set "name" value) → set buffer-local variable
fn prim_var_set(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    let value = args.get(1).cloned().ok_or("var-set requires a value")?;
    with_editor_state_mut(|state| {
        state.buffer_local_vars.insert(name, value);
        Ok(Value::Nil)
    })
}
/// (var-get "name") → get buffer-local variable value
fn prim_var_get(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    with_editor_state(|state| {
        match state.buffer_local_vars.get(&name) {
            Some(val) => Ok(val.clone()),
            None => Ok(Value::Nil),
        }
    })
}
/// (var-local "name" default) → set default value for buffer-local variable
fn prim_var_local(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    let default = args.get(1).cloned().unwrap_or(Value::Nil);
    with_editor_state_mut(|state| {
        if !state.buffer_local_vars.contains_key(&name) {
            state.buffer_local_vars.insert(name, default);
        }
        Ok(Value::Nil)
    })
}
/// (var-bound? "name") → check if buffer-local variable is bound
fn prim_var_bound(args: &[Value]) -> Result<Value, String> {
    let name = extract_string(args, 0)?;
    with_editor_state(|state| Ok(Value::Bool(state.buffer_local_vars.contains_key(&name))))
}
// --- Minibuffer primitives ---
/// (read-string "prompt" ["default"]) → read a string from the minibuffer
fn prim_read_string(args: &[Value]) -> Result<Value, String> {
    let _prompt = extract_string(args, 0)?;
    let default = args.get(1).and_then(|v| match v {
        Value::String(s) => Some(s.to_string()),
        _ => None,
    });
    // In headless/test mode, return default or empty
    // In interactive mode, this would display the minibuffer prompt
    Ok(Value::string(default.unwrap_or_default()))
}
/// (completing-read "prompt" [choices] ["default"]) → read with completion
fn prim_completing_read(args: &[Value]) -> Result<Value, String> {
    let _prompt = extract_string(args, 0)?;
    let _choices = args.get(1).cloned().unwrap_or(Value::Nil);
    let default = args.get(2).and_then(|v| match v {
        Value::String(s) => Some(s.to_string()),
        _ => None,
    });
    // In headless/test mode, return default or first choice
    Ok(Value::string(default.unwrap_or_default()))
}
/// (y-or-n? "prompt") → ask yes/no question
fn prim_y_or_n(args: &[Value]) -> Result<Value, String> {
    let _prompt = extract_string(args, 0)?;
    // In headless/test mode, default to true
    Ok(Value::Bool(true))
}
// --- Region primitives ---
/// (region-beginning) → position of region start
fn prim_region_beginning(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        if let Some((row, col)) = state.mark_pos {
            let mark_flat = row * 10000 + col;
            let cursor_flat = state.cursor_row * 10000 + state.cursor_col;
            let (r, c) = if mark_flat <= cursor_flat {
                (row, col)
            } else {
                (state.cursor_row, state.cursor_col)
            };
            Ok(Value::Int((r * 10000 + c) as i64))
        } else {
            Ok(Value::Int(
                (state.cursor_row * 10000 + state.cursor_col) as i64,
            ))
        }
    })
}
/// (region-end) → position of region end
fn prim_region_end(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        if let Some((row, col)) = state.mark_pos {
            let mark_flat = row * 10000 + col;
            let cursor_flat = state.cursor_row * 10000 + state.cursor_col;
            let (r, c) = if mark_flat >= cursor_flat {
                (row, col)
            } else {
                (state.cursor_row, state.cursor_col)
            };
            Ok(Value::Int((r * 10000 + c) as i64))
        } else {
            Ok(Value::Int(
                (state.cursor_row * 10000 + state.cursor_col) as i64,
            ))
        }
    })
}
/// (region-active?) → is the region currently active?
fn prim_region_active(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Bool(state.mark_active)))
}
/// (delete-region) → delete text between mark and cursor
fn prim_delete_region(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        if !state.mark_active {
            return Err("region is not active".to_string());
        }
        if let Some((mark_row, mark_col)) = state.mark_pos {
            let start_row = mark_row.min(state.cursor_row);
            let end_row = mark_row.max(state.cursor_row);
            let (start_col, end_col) = if mark_row == state.cursor_row {
                (mark_col.min(state.cursor_col), mark_col.max(state.cursor_col))
            } else if mark_row < state.cursor_row {
                (mark_col, state.cursor_col)
            } else {
                (state.cursor_col, mark_col)
            };
            if start_row == end_row {
                // Single line deletion
                if let Some(line) = state.lines.get_mut(start_row) {
                    let actual_end = end_col.min(line.len());
                    let actual_start = start_col.min(actual_end);
                    line.replace_range(actual_start..actual_end, "");
                }
            } else {
                // Multi-line deletion
                if start_row < state.lines.len() {
                    let start_line = state.lines[start_row].clone();
                    let end_line = state.lines.get(end_row).cloned().unwrap_or_default();
                    let prefix = if start_col <= start_line.len() {
                        &start_line[..start_col]
                    } else {
                        &start_line
                    };
                    let suffix = if end_col <= end_line.len() {
                        &end_line[end_col..]
                    } else {
                        ""
                    };
                    let merged = format!("{}{}", prefix, suffix);
                    state.lines.splice(start_row..=end_row, std::iter::once(merged));
                }
            }
            state.cursor_row = start_row;
            state.cursor_col = start_col;
            state.mark_active = false;
            state.mark_pos = None;
        }
        Ok(Value::Nil)
    })
}
/// (buffer-substring start end) → extract text between positions
fn prim_buffer_substring(args: &[Value]) -> Result<Value, String> {
    let start = extract_int(args, 0)? as usize;
    let end = extract_int(args, 1)? as usize;
    with_editor_state(|state| {
        let start_row = start / 10000;
        let start_col = start % 10000;
        let end_row = end / 10000;
        let end_col = end % 10000;
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
// --- Undo primitives ---
/// (undo) → undo last change
fn prim_undo(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        if !state.undo_enabled {
            return Err("undo is disabled".to_string());
        }
        if state.undo_tree.undo() {
            let snap = state.undo_tree.current().clone();
            state.lines = snap.lines;
            state.cursor_row = snap.cursor_row;
            state.cursor_col = snap.cursor_col;
            Ok(Value::Bool(true))
        } else {
            Ok(Value::Bool(false))
        }
    })
}
/// (redo) → redo last undone change
fn prim_redo(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        if state.undo_tree.redo() {
            let snap = state.undo_tree.current().clone();
            state.lines = snap.lines;
            state.cursor_row = snap.cursor_row;
            state.cursor_col = snap.cursor_col;
            Ok(Value::Bool(true))
        } else {
            Ok(Value::Bool(false))
        }
    })
}
/// (undo-boundary) → push an undo boundary (snapshot current state)
fn prim_undo_boundary(_args: &[Value]) -> Result<Value, String> {
    with_editor_state_mut(|state| {
        if state.undo_enabled {
            state.undo_tree.record(Snapshot {
                lines: state.lines.clone(),
                cursor_row: state.cursor_row,
                cursor_col: state.cursor_col,
            });
        }
        Ok(Value::Nil)
    })
}
/// (undo-enabled?) → is undo enabled?
fn prim_undo_enabled(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Bool(state.undo_enabled)))
}
/// (undo-tree-branches) → number of branches at current point
fn prim_undo_tree_branches(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Int(state.undo_tree.branch_count() as i64)))
}
/// (undo-tree-switch-branch N) → switch to branch N at current point
fn prim_undo_tree_switch_branch(args: &[Value]) -> Result<Value, String> {
    let n = extract_int(args, 0)? as usize;
    with_editor_state_mut(|state| {
        if state.undo_tree.switch_branch(n) {
            let snap = state.undo_tree.current().clone();
            state.lines = snap.lines;
            state.cursor_row = snap.cursor_row;
            state.cursor_col = snap.cursor_col;
            Ok(Value::Bool(true))
        } else {
            Ok(Value::Bool(false))
        }
    })
}
/// (undo-tree-visualize) → string representation of the tree
fn prim_undo_tree_visualize(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::string(state.undo_tree.visualize())))
}
/// (undo-tree-node-count) → total nodes in tree
fn prim_undo_tree_node_count(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Int(state.undo_tree.node_count() as i64)))
}
/// (undo-tree-can-undo?) → can undo from current position?
fn prim_undo_tree_can_undo(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Bool(state.undo_tree.can_undo())))
}
/// (undo-tree-can-redo?) → can redo from current position?
fn prim_undo_tree_can_redo(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| Ok(Value::Bool(state.undo_tree.can_redo())))
}
// --- Extended Hook primitives ---
/// (remove-hook "hook-name" handler) → remove a handler from a hook
fn prim_remove_hook(args: &[Value]) -> Result<Value, String> {
    let hook_name = extract_string(args, 0)?;
    // Remove by index or by matching handler
    match args.get(1) {
        Some(Value::Int(idx)) => {
            let idx = *idx as usize;
            with_editor_state_mut(|state| {
                if let Some(handlers) = state.hooks.get_mut(&hook_name) {
                    if idx < handlers.len() {
                        handlers.remove(idx);
                    }
                }
                Ok(Value::Nil)
            })
        }
        Some(handler) => {
            // Remove by identity (last matching)
            let handler_str = format!("{:?}", handler);
            with_editor_state_mut(|state| {
                if let Some(handlers) = state.hooks.get_mut(&hook_name) {
                    if let Some(pos) = handlers.iter().rposition(|h| format!("{:?}", h) == handler_str) {
                        handlers.remove(pos);
                    }
                }
                Ok(Value::Nil)
            })
        }
        None => Err("remove-hook requires hook name and handler".to_string()),
    }
}
/// (run-hook "hook-name") → run all handlers for a hook
fn prim_run_hook(args: &[Value]) -> Result<Value, String> {
    let hook_name = extract_string(args, 0)?;
    with_editor_state(|state| {
        match state.hooks.get(&hook_name) {
            Some(handlers) => {
                let count = handlers.len();
                // In headless mode, just count how many would run
                Ok(Value::Int(count as i64))
            }
            None => Ok(Value::Int(0)),
        }
    })
}
/// (hook-bound? "hook-name") → does this hook have handlers?
fn prim_hook_bound(args: &[Value]) -> Result<Value, String> {
    let hook_name = extract_string(args, 0)?;
    with_editor_state(|state| {
        Ok(Value::Bool(
            state.hooks.get(&hook_name).map_or(false, |h| !h.is_empty()),
        ))
    })
}
/// (hooks-for "hook-name") → list handler count for a hook
fn prim_hooks_for(args: &[Value]) -> Result<Value, String> {
    let hook_name = extract_string(args, 0)?;
    with_editor_state(|state| {
        Ok(Value::Int(
            state.hooks.get(&hook_name).map_or(0, |h| h.len()) as i64,
        ))
    })
}
// --- Extended Buffer primitives ---
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
/// (search-forward "pattern") → search forward for pattern, move cursor, return position or nil
fn prim_search_forward(args: &[Value]) -> Result<Value, String> {
    let pattern = extract_string(args, 0)?;
    with_editor_state_mut(|state| {
        // Search from current cursor position forward
        let start_col_offset = if state.cursor_col < state.lines.get(state.cursor_row).map_or(0, |l| l.len()) {
            state.cursor_col + 1
        } else {
            0
        };
        for row in state.cursor_row..state.lines.len() {
            let line = &state.lines[row];
            let search_from = if row == state.cursor_row { start_col_offset } else { 0 };
            if search_from < line.len() {
                if let Some(pos) = line[search_from..].find(&pattern) {
                    let col = search_from + pos;
                    state.cursor_row = row;
                    state.cursor_col = col;
                    return Ok(Value::Int((row * 10000 + col) as i64));
                }
            }
        }
        Ok(Value::Nil)
    })
}
/// (search-backward "pattern") → search backward for pattern
fn prim_search_backward(args: &[Value]) -> Result<Value, String> {
    let pattern = extract_string(args, 0)?;
    with_editor_state_mut(|state| {
        // Search from current cursor position backward
        for row in (0..=state.cursor_row).rev() {
            let line = state.lines.get(row).map(|s| s.as_str()).unwrap_or("");
            let search_end = if row == state.cursor_row {
                state.cursor_col
            } else {
                line.len()
            };
            if search_end > 0 {
                if let Some(pos) = line[..search_end].rfind(&pattern) {
                    state.cursor_row = row;
                    state.cursor_col = pos;
                    return Ok(Value::Int((row * 10000 + pos) as i64));
                }
            }
        }
        Ok(Value::Nil)
    })
}
/// (looking-at "pattern") → does text at cursor match pattern?
fn prim_looking_at(args: &[Value]) -> Result<Value, String> {
    let pattern = extract_string(args, 0)?;
    with_editor_state(|state| {
        if let Some(line) = state.lines.get(state.cursor_row) {
            if state.cursor_col < line.len() {
                return Ok(Value::Bool(line[state.cursor_col..].starts_with(&pattern)));
            }
        }
        Ok(Value::Bool(false))
    })
}
/// (buffer-list) → list of buffer names (currently just current buffer)
fn prim_buffer_list(_args: &[Value]) -> Result<Value, String> {
    with_editor_state(|state| {
        let name = state.file_path.as_deref().unwrap_or("*scratch*");
        Ok(Value::vector(vec![Value::string(name)]))
    })
}
// --- TRAMP remote file primitives ---
fn parse_tramp_path_arg(args: &[Value], idx: usize) -> Result<super::tramp::RemotePath, String> {
    let path_str = extract_string(args, idx)?;
    super::tramp::RemotePath::parse(&path_str)
        .ok_or_else(|| format!("invalid TRAMP path: {}", path_str))
}
/// (tramp-read-file "/ssh:user@host:/path") → string content
fn prim_tramp_read_file(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    let content = super::tramp::read_file(&rp)?;
    Ok(Value::string(content))
}
/// (tramp-write-file "/ssh:user@host:/path" "content") → nil
fn prim_tramp_write_file(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    let content = extract_string(args, 1)?;
    super::tramp::write_file(&rp, &content)?;
    Ok(Value::Nil)
}
/// (tramp-shell-command "/ssh:user@host:/path" "ls -la") → [stdout exit-code]
fn prim_tramp_shell_command(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    let cmd = extract_string(args, 1)?;
    let (stdout, code) = super::tramp::shell_command(&rp, &cmd)?;
    Ok(Value::vector(vec![
        Value::string(stdout),
        Value::Int(code as i64),
    ]))
}
/// (tramp-shell-capture "/ssh:user@host:/path" "hostname") → stdout string
fn prim_tramp_shell_capture(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    let cmd = extract_string(args, 1)?;
    let stdout = super::tramp::shell_capture(&rp, &cmd)?;
    Ok(Value::string(stdout))
}
/// (tramp-exists? "/ssh:user@host:/path") → bool
fn prim_tramp_file_exists(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    let exists = super::tramp::file_exists(&rp)?;
    Ok(Value::Bool(exists))
}
/// (tramp-mtime "/ssh:user@host:/path") → unix timestamp or nil
fn prim_tramp_file_mtime(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    match super::tramp::file_mtime(&rp) {
        Ok(mtime) => Ok(Value::Int(mtime)),
        Err(_) => Ok(Value::Nil),
    }
}
/// (tramp-list-dir "/ssh:user@host:/path") → vector of filenames
fn prim_tramp_list_dir(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    let entries = super::tramp::list_directory(&rp)?;
    Ok(Value::vector(
        entries.into_iter().map(Value::string).collect(),
    ))
}
/// (tramp-mkdir "/ssh:user@host:/path") → nil
fn prim_tramp_mkdir(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    super::tramp::make_directory(&rp)?;
    Ok(Value::Nil)
}
/// (tramp-delete-file "/ssh:user@host:/path") → nil
fn prim_tramp_delete_file(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    super::tramp::delete_file(&rp)?;
    Ok(Value::Nil)
}
/// (tramp-rename-file "/ssh:user@host:/old" "/new/path") → nil
fn prim_tramp_rename_file(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    let new_path = extract_string(args, 1)?;
    super::tramp::rename_file(&rp, &new_path)?;
    Ok(Value::Nil)
}
/// (tramp-ping "/ssh:user@host:/") → bool
fn prim_tramp_ping(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    let ok = super::tramp::ping(&rp)?;
    Ok(Value::Bool(ok))
}
/// (tramp-connect "/ssh:user@host:/") → target string
fn prim_tramp_connect(args: &[Value]) -> Result<Value, String> {
    let rp = parse_tramp_path_arg(args, 0)?;
    super::tramp::pool().touch(&rp.ssh_target());
    Ok(Value::string(rp.ssh_target()))
}
/// (tramp-disconnect "user@host") → nil
fn prim_tramp_disconnect(args: &[Value]) -> Result<Value, String> {
    let target = extract_string(args, 0)?;
    super::tramp::pool().disconnect(&target);
    Ok(Value::Nil)
}
/// (tramp-connections) → vector of active connections
fn prim_tramp_connections(_args: &[Value]) -> Result<Value, String> {
    let conns = super::tramp::pool().list();
    Ok(Value::vector(
        conns.into_iter().map(Value::string).collect(),
    ))
}
/// (tramp-parse-path "/ssh:user@host:/path") → map with :method, :user, :host, :port, :path
fn prim_tramp_parse_path(args: &[Value]) -> Result<Value, String> {
    let path_str = extract_string(args, 0)?;
    let rp = super::tramp::RemotePath::parse(&path_str)
        .ok_or_else(|| format!("invalid TRAMP path: {}", path_str))?;
    let mut pairs: Vec<(Value, Value)> = vec![
        (Value::keyword("method"), Value::string(&rp.method)),
        (Value::keyword("host"), Value::string(&rp.host)),
        (Value::keyword("path"), Value::string(&rp.path)),
    ];
    if let Some(user) = &rp.user {
        pairs.push((Value::keyword("user"), Value::string(user)));
    }
    if let Some(port) = rp.port {
        pairs.push((Value::keyword("port"), Value::Int(port as i64)));
    }
    Ok(Value::map(pairs))
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
    fn buffer_symbols_are_available() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        // Test buffer-name works
        let name = bridge.eval("(buffer-name)").unwrap();
        assert_eq!(name, Value::string("*scratch*"));
        // Test buffer-line-count works
        let count = bridge.eval("(buffer-line-count)").unwrap();
        assert_eq!(count, Value::Int(1));
        // Test buffer-content works
        let content = bridge.eval("(buffer-content)").unwrap();
        assert_eq!(content, Value::string(""));
        // Test buffer-set-content works
        bridge.eval("(buffer-set-content \"hello\")").unwrap();
        let content = bridge.eval("(buffer-content)").unwrap();
        assert_eq!(content, Value::string("hello"));
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
    // --- Kill Ring Tests ---
    #[test]
    fn kill_ring_push_and_yank() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        bridge
            .eval("(kill-ring-push \"hello world\")")
            .unwrap();
        let result = bridge.eval("(kill-ring-yank)").unwrap();
        assert_eq!(result, Value::string("hello world"));
        assert_eq!(
            bridge.eval("(kill-ring-count)").unwrap(),
            Value::Int(1)
        );
        take_editor_state();
    }
    #[test]
    fn kill_ring_pop_cycling() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        bridge.eval("(kill-ring-push \"first\")").unwrap();
        bridge.eval("(kill-ring-push \"second\")").unwrap();
        bridge.eval("(kill-ring-push \"third\")").unwrap();
        // yank returns most recent
        assert_eq!(bridge.eval("(kill-ring-yank)").unwrap(), Value::string("third"));
        // pop forward cycles
        assert_eq!(bridge.eval("(kill-ring-pop)").unwrap(), Value::string("first"));
        // pop back cycles backward
        assert_eq!(bridge.eval("(kill-ring-pop-back)").unwrap(), Value::string("third"));
        let contents = bridge.eval("(kill-ring-contents)").unwrap();
        match contents {
            Value::Vector(v) => assert_eq!(v.len(), 3),
            _ => panic!("expected vector"),
        }
        take_editor_state();
    }
    #[test]
    fn kill_ring_empty_returns_nil() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        assert_eq!(bridge.eval("(kill-ring-yank)").unwrap(), Value::Nil);
        assert_eq!(bridge.eval("(kill-ring-count)").unwrap(), Value::Int(0));
        take_editor_state();
    }
    // --- Mark Ring Tests ---
    // --- Mark Ring Tests ---
    #[test]
    fn set_mark_and_goto() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        // Set up buffer with enough lines
        bridge.eval("(buffer-set-content \"line0\nline1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10\nline11\")").unwrap();
        // Move cursor to line 5, col 3
        bridge.eval("(cursor-set! 5 3)").unwrap();
        bridge.eval("(set-mark)").unwrap();
        assert_eq!(bridge.eval("(mark-active?)").unwrap(), Value::Bool(true));
        let pos = bridge.eval("(mark-position)").unwrap();
        match pos {
            Value::Vector(v) => {
                assert_eq!(v[0], Value::Int(5));
                assert_eq!(v[1], Value::Int(3));
            }
            _ => panic!("expected vector"),
        }
        // Move cursor elsewhere
        bridge.eval("(cursor-set! 10 0)").unwrap();
        // Goto mark
        bridge.eval("(goto-mark)").unwrap();
        assert_eq!(bridge.eval("(cursor-row)").unwrap(), Value::Int(5));
        assert_eq!(bridge.eval("(cursor-col)").unwrap(), Value::Int(3));
        take_editor_state();
    }
    #[test]
    fn pop_mark_ring() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        // Set up buffer with enough lines
        bridge.eval("(buffer-set-content \"line0\nline1\nline2\nline3\nline4\nline5\")").unwrap();
        bridge.eval("(cursor-set! 1 0)").unwrap();
        bridge.eval("(set-mark)").unwrap();
        bridge.eval("(cursor-set! 2 0)").unwrap();
        bridge.eval("(set-mark)").unwrap();
        bridge.eval("(cursor-set! 3 0)").unwrap();
        // Pop mark -> goes to row 2
        bridge.eval("(pop-mark)").unwrap();
        assert_eq!(bridge.eval("(cursor-row)").unwrap(), Value::Int(2));
        // Deactivate mark
        bridge.eval("(deactivate-mark)").unwrap();
        assert_eq!(bridge.eval("(mark-active?)").unwrap(), Value::Bool(false));
        take_editor_state();
    }
    // --- Register Tests ---
    #[test]
    fn register_set_and_get() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        bridge.eval("(register-set \"a\" \"hello\")").unwrap();
        assert_eq!(
            bridge.eval("(register-get \"a\")").unwrap(),
            Value::string("hello")
        );
        assert_eq!(bridge.eval("(register-get \"z\")").unwrap(), Value::Nil);
        take_editor_state();
    }
    // --- Buffer-Local Variable Tests ---
    #[test]
    fn var_set_and_get() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        bridge.eval("(var-set \"tab-width\" 4)").unwrap();
        assert_eq!(
            bridge.eval("(var-get \"tab-width\")").unwrap(),
            Value::Int(4)
        );
        assert_eq!(bridge.eval("(var-bound? \"tab-width\")").unwrap(), Value::Bool(true));
        assert_eq!(bridge.eval("(var-bound? \"unknown\")").unwrap(), Value::Bool(false));
        assert_eq!(bridge.eval("(var-get \"unknown\")").unwrap(), Value::Nil);
        take_editor_state();
    }
    #[test]
    fn var_local_sets_default_only() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        bridge.eval("(var-local \"indent-tabs-mode\" true)").unwrap();
        assert_eq!(
            bridge.eval("(var-get \"indent-tabs-mode\")").unwrap(),
            Value::Bool(true)
        );
        // var-local should not overwrite existing value
        bridge.eval("(var-local \"indent-tabs-mode\" false)").unwrap();
        assert_eq!(
            bridge.eval("(var-get \"indent-tabs-mode\")").unwrap(),
            Value::Bool(true)
        );
        take_editor_state();
    }
    // --- Region Tests ---
    #[test]
    fn region_operations() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        assert_eq!(bridge.eval("(region-active?)").unwrap(), Value::Bool(false));
        bridge.eval("(set-mark)").unwrap();
        assert_eq!(bridge.eval("(region-active?)").unwrap(), Value::Bool(true));
        // Region beginning should be at cursor (0,0)
        assert_eq!(bridge.eval("(region-beginning)").unwrap(), Value::Int(0));
        assert_eq!(bridge.eval("(region-end)").unwrap(), Value::Int(0));
        take_editor_state();
    }
    fn delete_region_removes_text() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        // Set up content: "hello world"
        bridge.eval("(buffer-set-content \"hello world\")").unwrap();
        bridge.eval("(cursor-set! 0 5)").unwrap(); // cursor at space
        bridge.eval("(set-mark)").unwrap();
        bridge.eval("(cursor-set! 0 0)").unwrap(); // mark at start
        bridge.eval("(delete-region)").unwrap();
        assert_eq!(
            bridge.eval("(buffer-content)").unwrap(),
            Value::string(" world")
        );
        take_editor_state();
    }
    // --- Undo Tests ---
    #[test]
    fn undo_redo_cycle() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        // Record initial state
        bridge.eval("(undo-boundary)").unwrap();
        // Make a change and record
        bridge.eval("(buffer-set-content \"modified\")").unwrap();
        bridge.eval("(undo-boundary)").unwrap();
        // Undo should restore previous state (empty)
        let result = bridge.eval("(undo)").unwrap();
        assert_eq!(result, Value::Bool(true));
        assert_eq!(bridge.eval("(buffer-content)").unwrap(), Value::string(""));
        // Redo should restore the modification
        let result = bridge.eval("(redo)").unwrap();
        assert_eq!(result, Value::Bool(true));
        assert_eq!(bridge.eval("(buffer-content)").unwrap(), Value::string("modified"));
        // Undo again
        let result = bridge.eval("(undo)").unwrap();
        assert_eq!(result, Value::Bool(true));
        assert_eq!(bridge.eval("(buffer-content)").unwrap(), Value::string(""));
        // Make a DIFFERENT edit (creates branch instead of overwriting)
        bridge.eval("(buffer-set-content \"alternate\")").unwrap();
        bridge.eval("(undo-boundary)").unwrap();
        assert_eq!(
            bridge.eval("(buffer-content)").unwrap(),
            Value::string("alternate")
        );
        // Go back — should have 2 branches now
        bridge.eval("(undo)").unwrap();
        assert_eq!(
            bridge.eval("(undo-tree-branches)").unwrap(),
            Value::Int(2)
        );
        take_editor_state();
    }
    // --- Hook Extension Tests ---
    #[test]
    fn hook_query_operations() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        // Initially no hooks
        assert_eq!(bridge.eval("(hook-bound? \"after-save\")").unwrap(), Value::Bool(false));
        assert_eq!(bridge.eval("(hooks-for \"after-save\")").unwrap(), Value::Int(0));
        // Add a hook
        bridge.eval("(add-hook \"after-save\" (fn [] nil))").unwrap();
        assert_eq!(bridge.eval("(hook-bound? \"after-save\")").unwrap(), Value::Bool(true));
        assert_eq!(bridge.eval("(hooks-for \"after-save\")").unwrap(), Value::Int(1));
        // Remove by index
        bridge.eval("(remove-hook \"after-save\" 0)").unwrap();
        assert_eq!(bridge.eval("(hook-bound? \"after-save\")").unwrap(), Value::Bool(false));
        take_editor_state();
    }
    // --- Minibuffer Tests ---
    #[test]
    fn minibuffer_read_string() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        // In headless mode, read-string returns default
        let result = bridge
            .eval("(read-string \"Name: \" \"default-name\")")
            .unwrap();
        assert_eq!(result, Value::string("default-name"));
        // Without default, returns empty
        let result = bridge.eval("(read-string \"Query: \")").unwrap();
        assert_eq!(result, Value::string(""));
        take_editor_state();
    }
    #[test]
    fn minibuffer_y_or_n() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        // In headless mode, y-or-n? defaults to true
        let result = bridge.eval("(y-or-n? \"Save? \")").unwrap();
        assert_eq!(result, Value::Bool(true));
        take_editor_state();
    }
    // --- Narrowing Tests ---
    #[test]
    fn narrow_and_widen() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        bridge.eval("(buffer-set-content \"line1\nline2\nline3\nline4\")").unwrap();
        assert_eq!(bridge.eval("(buffer-narrowed?)").unwrap(), Value::Bool(false));
        bridge.eval("(narrow-to-region 1 3)").unwrap();
        assert_eq!(bridge.eval("(buffer-narrowed?)").unwrap(), Value::Bool(true));
        bridge.eval("(widen)").unwrap();
        assert_eq!(bridge.eval("(buffer-narrowed?)").unwrap(), Value::Bool(false));
        take_editor_state();
    }
    // --- Text Search Tests ---
    #[test]
    fn search_forward_finds_pattern() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        bridge.eval("(buffer-set-content \"hello world foo bar\")").unwrap();
        bridge.eval("(cursor-set! 0 0)").unwrap();
        let result = bridge.eval("(search-forward \"world\")").unwrap();
        assert_eq!(result, Value::Int(6)); // "world" starts at col 6
        assert_eq!(bridge.eval("(cursor-col)").unwrap(), Value::Int(6));
        // Search for missing pattern returns nil
        bridge.eval("(cursor-set! 0 0)").unwrap();
        let result = bridge.eval("(search-forward \"notfound\")").unwrap();
        assert_eq!(result, Value::Nil);
        take_editor_state();
    }
    #[test]
    fn search_backward_finds_pattern() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        bridge.eval("(buffer-set-content \"hello world hello\")").unwrap();
        bridge.eval("(cursor-set! 0 15)").unwrap(); // near end
        let result = bridge.eval("(search-backward \"hello\")").unwrap();
        assert_eq!(result, Value::Int(0)); // first "hello" at col 0
        take_editor_state();
    }
    #[test]
    fn looking_at_checks_at_cursor() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        bridge.eval("(buffer-set-content \"hello world\")").unwrap();
        bridge.eval("(cursor-set! 0 0)").unwrap();
        assert_eq!(
            bridge.eval("(looking-at \"hello\")").unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            bridge.eval("(looking-at \"world\")").unwrap(),
            Value::Bool(false)
        );
        bridge.eval("(cursor-set! 0 6)").unwrap();
        assert_eq!(
            bridge.eval("(looking-at \"world\")").unwrap(),
            Value::Bool(true)
        );
        take_editor_state();
    }
    // --- Buffer List Test ---
    #[test]
    fn buffer_list_returns_current() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        let result = bridge.eval("(buffer-list)").unwrap();
        match result {
            Value::Vector(v) => {
                assert_eq!(v.len(), 1);
                assert_eq!(v[0], Value::string("*scratch*"));
            }
            _ => panic!("expected vector"),
        }
        take_editor_state();
    }
    // --- Integration: Emacs-like init.mora pattern ---
    #[test]
    fn emacs_like_init_config() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        // Simulate a user's init.mora using emacs-like primitives
        bridge
            .eval(
                r#"
                ;; Set buffer-local variables
                (var-set "tab-width" 4)
                (var-set "indent-tabs-mode" false)
                ;; Define a command using mark and region
                (defn delete-line
                  "Delete current line."
                  []
                  (interactive)
                  (set-mark)
                  (cursor-end-of-line)
                  (delete-region))
                ;; Set up hooks
                (add-hook "before-save"
                  (fn []
                    (editor-message (str "Saving: " (buffer-name)))))
                ;; Store a snippet in register
                (register-set "s" "fn main() {\n    \n}")
                ;; Push to kill ring
                (kill-ring-push "import std;")
                ;; Undo boundary before major change
                (undo-boundary)
                (buffer-set-content "new content")
                (undo-boundary)
                "#,
            )
            .unwrap();
        // Verify everything worked
        assert_eq!(bridge.eval("(var-get \"tab-width\")").unwrap(), Value::Int(4));
        assert_eq!(
            bridge.eval("(register-get \"s\")").unwrap(),
            Value::string("fn main() {\n    \n}")
        );
        assert_eq!(bridge.eval("(kill-ring-yank)").unwrap(), Value::string("import std;"));
        assert!(bridge.has_command("delete-line"));
        assert_eq!(bridge.eval("(hook-bound? \"before-save\")").unwrap(), Value::Bool(true));
        take_editor_state();
    }
    // --- Undo-Tree Tests ---
    #[test]
    fn undo_tree_branching_preserves_alternate_history() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        // Initial state
        bridge.eval("(undo-boundary)").unwrap();
        // Make edit A
        bridge.eval("(buffer-set-content \"A\")").unwrap();
        bridge.eval("(undo-boundary)").unwrap();
        // Make edit B
        bridge.eval("(buffer-set-content \"B\")").unwrap();
        bridge.eval("(undo-boundary)").unwrap();
        // Undo to A
        bridge.eval("(undo)").unwrap();
        assert_eq!(bridge.eval("(buffer-content)").unwrap(), Value::string("A"));
        // Make edit C (creates branch instead of destroying B)
        bridge.eval("(buffer-set-content \"C\")").unwrap();
        bridge.eval("(undo-boundary)").unwrap();
        assert_eq!(bridge.eval("(buffer-content)").unwrap(), Value::string("C"));
        // Go back to A — should have 2 branches
        bridge.eval("(undo)").unwrap();
        assert_eq!(bridge.eval("(buffer-content)").unwrap(), Value::string("A"));
        assert_eq!(
            bridge.eval("(undo-tree-branches)").unwrap(),
            Value::Int(2)
        );
        // Switch to branch 0
        bridge.eval("(undo-tree-switch-branch 0)").unwrap();
        let branch0 = bridge.eval("(buffer-content)").unwrap();
        // Go back, switch to branch 1
        bridge.eval("(undo)").unwrap();
        bridge.eval("(undo-tree-switch-branch 1)").unwrap();
        let branch1 = bridge.eval("(buffer-content)").unwrap();
        // Both branches accessible
        assert_ne!(branch0, branch1);
        take_editor_state();
    }
    #[test]
    fn undo_tree_visualize_shows_structure() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        bridge.eval("(undo-boundary)").unwrap();
        bridge.eval("(buffer-set-content \"A\")").unwrap();
        bridge.eval("(undo-boundary)").unwrap();
        bridge.eval("(undo)").unwrap();
        bridge.eval("(buffer-set-content \"B\")").unwrap();
        bridge.eval("(undo-boundary)").unwrap();
        let vis = bridge.eval("(undo-tree-visualize)").unwrap();
        match vis {
            Value::String(s) => {
                assert!(s.contains("●"), "should show active node");
                assert!(s.contains("○"), "should show inactive nodes");
            }
            _ => panic!("expected string"),
        }
        let count = bridge.eval("(undo-tree-node-count)").unwrap();
        // root + boundary-of-A + A + boundary-of-B + B = 5 nodes
        // (each boundary records a new node in tree)
        assert!(matches!(count, Value::Int(n) if n >= 3));
        take_editor_state();
    }
    #[test]
    fn undo_tree_can_undo_redo() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        // Fresh tree at root — nothing to undo
        assert_eq!(
            bridge.eval("(undo-tree-can-undo?)").unwrap(),
            Value::Bool(false)
        );
        assert_eq!(
            bridge.eval("(undo-tree-can-redo?)").unwrap(),
            Value::Bool(false)
        );
        // Record an edit
        bridge.eval("(undo-boundary)").unwrap();
        bridge.eval("(buffer-set-content \"edit\")").unwrap();
        bridge.eval("(undo-boundary)").unwrap();
        assert_eq!(
            bridge.eval("(undo-tree-can-undo?)").unwrap(),
            Value::Bool(true)
        );
        // Undo to previous state
        bridge.eval("(undo)").unwrap();
        assert_eq!(
            bridge.eval("(undo-tree-can-redo?)").unwrap(),
            Value::Bool(true)
        );
        take_editor_state();
    }
    // --- TRAMP Tests ---
    #[test]
    fn tramp_parse_path_works() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        let result = bridge
            .eval(r#"(tramp-parse-path "/ssh:user@host:/home/user/file.txt")"#)
            .unwrap();
        match result {
            Value::Map(m) => {
                let method = m.get(&Value::keyword("method")).unwrap();
                assert_eq!(*method, Value::string("ssh"));
                let host = m.get(&Value::keyword("host")).unwrap();
                assert_eq!(*host, Value::string("host"));
                let user = m.get(&Value::keyword("user")).unwrap();
                assert_eq!(*user, Value::string("user"));
                let path = m.get(&Value::keyword("path")).unwrap();
                assert_eq!(*path, Value::string("/home/user/file.txt"));
            }
            _ => panic!("expected map"),
        }
        take_editor_state();
    }
    #[test]
    fn tramp_parse_path_with_port() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        let result = bridge
            .eval(r#"(tramp-parse-path "/ssh:admin@server#2222:/etc/config")"#)
            .unwrap();
        match result {
            Value::Map(m) => {
                let port = m.get(&Value::keyword("port")).unwrap();
                assert_eq!(*port, Value::Int(2222));
            }
            _ => panic!("expected map"),
        }
        take_editor_state();
    }
    #[test]
    fn tramp_parse_path_no_user() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        let result = bridge
            .eval(r#"(tramp-parse-path "/scp:example.com:/tmp/data")"#)
            .unwrap();
        match result {
            Value::Map(m) => {
                assert_eq!(*m.get(&Value::keyword("method")).unwrap(), Value::string("scp"));
                assert!(!m.contains_key(&Value::keyword("user")));
            }
            _ => panic!("expected map"),
        }
        take_editor_state();
    }
    #[test]
    fn tramp_connections_empty_initially() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        let result = bridge.eval("(tramp-connections)").unwrap();
        match result {
            Value::Vector(v) => assert!(v.is_empty()),
            _ => panic!("expected vector"),
        }
        take_editor_state();
    }
    #[test]
    fn tramp_invalid_path_errors() {
        set_editor_state(EditorState::new());
        let mut bridge = MoraLispBridge::new();
        let result = bridge.eval(r#"(tramp-parse-path "/not-a-tramp-path")"#);
        assert!(result.is_err());
        let result = bridge.eval(r#"(tramp-parse-path "/ssh:")"#);
        assert!(result.is_err());
        take_editor_state();
    }
}
