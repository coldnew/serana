pub mod buffer;
pub mod command;
pub mod core;
pub mod cursor;
pub mod editing;
pub mod editor_state;
pub mod grep;
pub mod helpers;
pub mod history;
pub mod hook;
pub mod iedit;
pub mod kill_ring;
pub mod leader;
pub mod mark;
pub mod minibuffer;
pub mod mode;
pub mod org;
pub mod overlay;
pub mod project;
pub mod region;
pub mod register;
pub mod search;
pub mod session;
pub mod shell;
pub mod smartparens;
pub mod tramp;
pub mod undo;
pub mod util;
pub mod var;
pub mod visual;
pub mod window;
// Re-exports for backward compatibility
pub use core::MoraLispBridge;
pub use core::{build_lisp_ui, build_lisp_ui_component, lisp_value_to_uinode};
pub use editor_state::{
    set_editor_state, take_editor_state, with_editor_state, with_editor_state_mut, EditorState,
};

pub mod hydra;
pub mod modeline;
#[cfg(test)]
mod tests;
pub mod theme;
