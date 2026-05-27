//! Re-exports display-protocol style types.
//!
//! MoraColor, MoraStyle, StyledSpan, StyledLine are aliases for the
//! canonical types defined in `display_protocol`. Ratatui conversions
//! are provided by display-protocol's `tui` feature.

pub use display_protocol::{Color as MoraColor, Style as MoraStyle};
pub use display_protocol::{StyledSpan, StyledLine};
