//! Re-exports display-protocol input types.
//!
//! MoraKeyEvent, MoraKeyCode, MoraKeyModifiers are aliases for the
//! canonical types defined in `display_protocol`. Crossterm conversions
//! are provided by display-protocol's `tui` feature.

pub use display_protocol::{
    KeyCode as MoraKeyCode, KeyEvent as MoraKeyEvent, KeyModifiers as MoraKeyModifiers,
};
