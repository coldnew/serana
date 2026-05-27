//! Display Protocol - Shared frontend/backend protocol for mora and serana
//!
//! This crate defines the message types for communication between an editor core
//! and display frontends. The architecture follows a client-server model:
//!
//! ```text
//! Core (headless) ──[FrameUpdate]──► Frontend (TUI/WGPU/Web)
//!                  ◄──[InputEvent]──
//!                  ──[DisplayCmd]──►
//! ```
//!
//! - `FrameUpdate`: Core → Frontend. Contains the rendered grid, cursor, chrome.
//! - `InputEvent`: Frontend → Core. Keyboard, mouse, resize events.
//! - `DisplayCmd`: Core → Frontend. Imperative commands (window title, bell, etc.)

mod types;
mod frame;
mod input;
mod command;
pub mod wire;

#[cfg(feature = "tui")]
mod conversions;

pub use types::{Color, Style, StyledSpan, StyledLine};
pub use frame::{Cell, Grid, CursorState, CursorStyle, StatusLine, FrameUpdate};
pub use input::{KeyEvent, KeyCode, KeyModifiers, InputEvent, MouseEventKind};
pub use command::{DisplayCmd, PopupItem, PopupItemKind};
pub use wire::{WireMessage, PROTOCOL_VERSION};
