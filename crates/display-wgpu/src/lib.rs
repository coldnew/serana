//! display-wgpu: WGPU backend for the display protocol.
//!
//! This crate provides a GPU-accelerated rendering backend for
//! `display-protocol` UI trees, analogous to how `display-tui`
//! provides a terminal-based backend.
//!
//! # Architecture
//!
//! ```text
//! UiNode tree ──► paint() ──► ScreenBuffer ──► WgpuRenderer ──► GPU ──► Screen
//! ```
//!
//! Each character cell is rendered as an instanced quad:
//! - Background fills the cell with the cell's bg color.
//! - Foreground glyph is sampled from a texture atlas.
//!
//! # Usage
//!
//! ```ignore
//! use display_wgpu::{WgpuWindow, WgpuConfig};
//! use display_protocol::*;
//!
//! let font = std::fs::read("fonts/Hack-Regular.ttf").unwrap();
//! let config = WgpuConfig::default();
//!
//! WgpuWindow::new(config, &font).run(|events, ctx| {
//!     UiNode::Column(FlexNode {
//!         children: vec![
//!             UiNode::text("Hello from WGPU!"),
//!             UiNode::text(format!("Grid: {}x{}", ctx.grid_cols, ctx.grid_rows)),
//!         ],
//!         ..Default::default()
//!     })
//! }).unwrap();
//! ```

pub mod atlas;
pub mod input;
pub mod renderer;
pub mod window;

pub use renderer::WgpuRenderer;
pub use window::{WgpuConfig, WgpuWindow, RenderCtx};
pub use atlas::GlyphAtlas;
pub use input::{winit_to_input_event, winit_key_to_code, winit_mods_to_proto};
