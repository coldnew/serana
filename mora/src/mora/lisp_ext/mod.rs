pub mod buffer;
pub mod command;
pub mod core;
pub mod editing;
pub mod cursor;
pub mod editor_state;
pub mod helpers;
pub mod hook;
pub mod history;
pub mod kill_ring;
pub mod mark;
pub mod minibuffer;
pub mod mode;
pub mod overlay;
pub mod region;
pub mod register;
pub mod search;
pub mod shell;
pub mod tramp;
pub mod undo;
pub mod var;
pub mod visual;
pub mod window;
// Re-exports for backward compatibility
pub use core::MoraLispBridge;
pub use core::{build_lisp_ui, lisp_value_to_uinode};
pub use editor_state::{
    EditorState, set_editor_state, take_editor_state, with_editor_state, with_editor_state_mut,
};

#[cfg(test)]
mod tests;