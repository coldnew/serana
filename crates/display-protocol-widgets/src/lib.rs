//! display-protocol-widgets: reusable UI widgets for display-protocol
//!
//! All widgets work on both TUI and WGPU backends. They are stateless
//! visual descriptions — build them with current state, get a `UiNode` tree,
//! then paint.

pub mod palette;

mod button;
pub use button::{Button, ButtonStyle};

mod checkbox;
pub use checkbox::Checkbox;

mod badge;
pub use badge::{Badge, BadgeStyle};
