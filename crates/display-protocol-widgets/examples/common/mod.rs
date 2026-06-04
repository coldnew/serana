use display_protocol::UiNode;

pub fn print_node(node: &UiNode, width: u16, height: u16) {
    println!("Rendered TUI:");
    for line in display_tui::render_ui_node_to_lines(node, width, height) {
        println!("{line}");
    }
}
