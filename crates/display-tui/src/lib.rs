pub mod conversions;
mod input;
pub mod protocol_widgets;
mod terminal;

pub use input::{crossterm_to_input_event, crossterm_to_key_event, poll_input};
pub use protocol_widgets::{render_widget_spec_to_lines, trim_rendered_lines};
pub use terminal::{install_panic_hook, TuiTerminal};

use display_protocol::{paint, ScreenBuffer, UiNode};

/// Render a declarative UI node through the TUI buffer path.
pub fn render_ui_node_to_screen_buffer(node: &UiNode, width: u16, height: u16) -> ScreenBuffer {
    paint(node, width, height)
}

/// Render a declarative UI node to trimmed terminal lines for examples/tests.
pub fn render_ui_node_to_lines(node: &UiNode, width: u16, height: u16) -> Vec<String> {
    let buffer = render_ui_node_to_screen_buffer(node, width, height);
    let mut lines = Vec::new();
    for y in 0..buffer.height {
        let mut line = String::new();
        for x in 0..buffer.width {
            line.push(buffer.get(x, y).ch);
        }
        while line.ends_with(' ') {
            line.pop();
        }
        lines.push(line);
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    lines
}
