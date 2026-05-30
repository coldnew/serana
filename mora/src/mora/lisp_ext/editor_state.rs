use std::cell::RefCell;
use std::collections::VecDeque;
use std::collections::HashMap;

use crate::lisp::types::Value;

use super::super::overlay::OverlayStore;
use super::super::undo_tree::{Snapshot, UndoTree};

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
    pub hooks: HashMap<String, Vec<Value>>,
    pub keybindings: HashMap<String, Value>,
    pub overlays: OverlayStore,
    pub ui_builders: Vec<Value>,
    // --- Emacs-like state ---
    /// Buffer-local variables: var_name -> value
    pub buffer_local_vars: HashMap<String, Value>,
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
    /// Expand-region level tracking
    pub expand_region_level: usize,
    /// Last edit positions for goto-last-change
    pub last_changes: VecDeque<(usize, usize)>,
    pub undo_tree: UndoTree,
    pub focus_mode: bool,
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
            hooks: HashMap::new(),
            keybindings: HashMap::new(),
            overlays: OverlayStore::new(),
            ui_builders: Vec::new(),
            buffer_local_vars: HashMap::new(),
            mark_ring: Vec::new(),
            mark_active: false,
            mark_pos: None,
            kill_ring: Vec::new(),
            kill_ring_idx: 0,
            registers: HashMap::new(),
            narrow_start: None,
            narrow_end: None,
            undo_enabled: true,
            expand_region_level: 0,
            last_changes: VecDeque::new(),
            undo_tree: UndoTree::new(Snapshot {
                lines: vec![String::new()],
                cursor_row: 0,
                cursor_col: 0,
            }),
            focus_mode: false,
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
    EDITOR_STATE.with(|s| s.borrow_mut().take())
}
