//! Re-exports display-protocol input types.
//!
//! MoraKeyEvent, MoraKeyCode, MoraKeyModifiers are aliases for the
//! canonical types defined in `display_protocol`. Crossterm conversions
//! are provided by display-protocol's `tui` feature.

pub use display_protocol::{KeyEvent as MoraKeyEvent, KeyCode as MoraKeyCode, KeyModifiers as MoraKeyModifiers};
