use display_protocol::{paint, ScreenBuffer, UiNode};

pub fn print_node(node: &UiNode, width: u16, height: u16) {
    let buffer = paint(node, width, height);
    println!("Rendered TUI:");
    print_buffer(&buffer);
}

fn print_buffer(buffer: &ScreenBuffer) {
    let lines = rendered_lines(buffer);
    let mut last = lines.len();
    while last > 0 && lines[last - 1].trim().is_empty() {
        last -= 1;
    }
    for line in &lines[..last] {
        println!("{line}");
    }
}

fn rendered_lines(buffer: &ScreenBuffer) -> Vec<String> {
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
    lines
}
