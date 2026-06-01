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
//! ## Declarative UI System
//!
//! In addition to the grid-based protocol, this crate provides a declarative
//! UI system inspired by React/storm-rs:
//!
//! ```ignore
//! use display_protocol::*;
//! use display_protocol_rsx::rsx;
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
//! let commands = collect_render_commands(&tree, 80, 24);
//! let buf = render_commands_to_buffer(commands.commands(), 80, 24);
//! ```
//!
//! The `rsx!` macro is provided by the `display-protocol-rsx` crate.
//! The `jsx!` proc-macro is provided by `display-protocol-jsx`.

mod command;
mod frame;
mod input;
mod types;
pub mod wire;

// Declarative UI system
pub mod buffer;
pub mod layout;
pub mod paint;
pub mod render;
pub mod renderer;
pub mod ui;

pub use command::{DisplayCmd, PopupItem, PopupItemKind};
pub use frame::{Cell, CursorState, CursorStyle, FrameUpdate, Grid, StatusLine};
pub use input::{InputEvent, KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
pub use types::{Color, MultiSelection, Selection, SelectionMode, Style, StyledLine, StyledSpan};
pub use wire::{WireMessage, PROTOCOL_VERSION};

// Declarative UI re-exports
pub use buffer::{ScreenBuffer, ScreenCell};
pub use layout::{compute_layout, LayoutResult};
pub use paint::{collect_render_commands, paint};
pub use render::{
    apply_render_command, render_commands_to_buffer, RenderCommand, RenderCommandArray,
    RenderTarget,
};
pub use renderer::{AnsiRenderer, Renderer};
pub use ui::{
    Align, AnnotationKind, Border, BoxNode, CanvasNode, DividerNode, FlexNode, FocusDirection,
    FocusOrder, GutterAnnotation, InputNode, Justify, ListMarker, ListNode, MenuAnchor, MenuItem,
    MenuNode, Orientation, OverlayNode, Padding, ProgressNode, ScrollNode, ScrollPolicy, SpanNode,
    SplitPaneNode, StatusBarNode, TabBarNode, TabItem, TableNode, TextAreaNode, TextNode, TreeItem,
    TreeViewNode, UiNode, WidgetEvent, Wrap,
};
