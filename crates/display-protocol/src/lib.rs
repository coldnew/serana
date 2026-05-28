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
//! ## Declarative UI System
//!
//! In addition to the grid-based protocol, this crate provides a declarative
//! UI system inspired by React/storm-rs:
//!
//! ```ignore
//! use display_protocol::*;
//!
//! let tree = rsx! {
//!     Column(gap = 1) {
//!         Text(bold) { "Hello, World!" }
//!         Divider()
//!         Row(gap = 2) {
//!             Text(color = Color::new(255, 0, 0)) { "Red" }
//!             Text(color = Color::new(0, 255, 0)) { "Green" }
//!         }
//!     }
//! };
//!
//! let buf = paint(&tree, 80, 24);
//! ```

mod types;
mod frame;
mod input;
mod command;
pub mod wire;

// Declarative UI system
pub mod ui;
pub mod rsx;
pub mod buffer;
pub mod layout;
pub mod paint;
pub mod renderer;

#[cfg(feature = "tui")]
mod conversions;

pub use types::{Color, Style, StyledSpan, StyledLine};
pub use frame::{Cell, Grid, CursorState, CursorStyle, StatusLine, FrameUpdate};
pub use input::{KeyEvent, KeyCode, KeyModifiers, InputEvent, MouseEventKind};
pub use command::{DisplayCmd, PopupItem, PopupItemKind};
pub use wire::{WireMessage, PROTOCOL_VERSION};

// Declarative UI re-exports
pub use ui::{
    UiNode, TextNode, BoxNode, FlexNode, SpanNode, ListNode, DividerNode,
    ProgressNode, TableNode, ScrollNode,
    InputNode, TextAreaNode, TabBarNode, TabItem, TreeViewNode, TreeItem,
    SplitPaneNode, CanvasNode, OverlayNode, StatusBarNode, Orientation,
    Padding, Border, Wrap, Align, Justify, ListMarker,
};
pub use buffer::{ScreenBuffer, ScreenCell};
pub use layout::{LayoutResult, compute_layout};
pub use paint::{paint, paint_into};
pub use renderer::{Renderer, AnsiRenderer};
