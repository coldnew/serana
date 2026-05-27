mod terminal;
mod input;

pub use terminal::TuiTerminal;
pub use input::{poll_input, crossterm_to_input_event, crossterm_to_key_event};
