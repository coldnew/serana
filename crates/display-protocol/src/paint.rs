use crate::buffer::ScreenBuffer;
use crate::layout::{compute_layout, LayoutResult, wrap_text};
use crate::types::Color;
use crate::ui::*;

/// Paint a UiNode tree into a ScreenBuffer.
///
/// This is the rendering bridge: computes layout, then paints cells.
pub fn paint(node: &UiNode, width: u16, height: u16) -> ScreenBuffer {
    let layout = compute_layout(node, width, height);
    let mut buf = ScreenBuffer::new(width, height);
    paint_node(node, &layout, &mut buf);
    buf
}

/// Paint with an existing buffer (for incremental updates).
pub fn paint_into(node: &UiNode, buf: &mut ScreenBuffer) {
    let layout = compute_layout(node, buf.width, buf.height);
    paint_node(node, &layout, buf);
}

fn paint_node(node: &UiNode, layout: &LayoutResult, buf: &mut ScreenBuffer) {
    match node {
        UiNode::Text(t) => paint_text(t, layout, buf),
        UiNode::Span(s) => paint_span(s, layout, buf),
        UiNode::Box(b) => paint_box(b, layout, buf),
        UiNode::Row(f) | UiNode::Column(f) => {
            for (i, child) in f.children.iter().enumerate() {
                if let Some(child_layout) = layout.children.get(i) {
                    paint_node(child, child_layout, buf);
                }
            }
        }
        UiNode::Divider(d) => paint_divider(d, layout, buf),
        UiNode::ProgressBar(p) => paint_progress(p, layout, buf),
        UiNode::List(l) => paint_list(l, layout, buf),
        UiNode::ListItem(child) => paint_node(child, layout, buf),
        UiNode::Table(t) => paint_table(t, layout, buf),
        UiNode::ScrollView(s) => paint_scroll_view(s, layout, buf),
        UiNode::Show { when, child } => {
            if *when {
                if let Some(child_layout) = layout.children.first() {
                    paint_node(child, child_layout, buf);
                }
            }
        }
        UiNode::For { children } => {
            for (i, child) in children.iter().enumerate() {
                if let Some(child_layout) = layout.children.get(i) {
                    paint_node(child, child_layout, buf);
                }
            }
        }
        UiNode::None => {}
    }
}

fn paint_text(node: &TextNode, layout: &LayoutResult, buf: &mut ScreenBuffer) {
    let x = layout.x as u16;
    let y = layout.y as u16;

    match node.wrap {
        Wrap::NoWrap | Wrap::Truncate => {
            let max_w = layout.width as u16;
            let text: String = if node.wrap == Wrap::Truncate {
                node.content.chars().take(max_w as usize).collect()
            } else {
                node.content.clone()
            };
            buf.write_styled(x, y, &text, &node.style);
        }
        Wrap::Wrap => {
            let lines = wrap_text(&node.content, layout.width as u16);
            for (i, line) in lines.iter().enumerate() {
                if y + i as u16 >= buf.height { break; }
                buf.write_styled(x, y + i as u16, line, &node.style);
            }
        }
    }
}

fn paint_span(node: &SpanNode, layout: &LayoutResult, buf: &mut ScreenBuffer) {
    let x = layout.x as u16;
    let y = layout.y as u16;
    buf.write_styled(x, y, &node.content, &node.style);
}

fn paint_box(node: &BoxNode, layout: &LayoutResult, buf: &mut ScreenBuffer) {
    let x = layout.x as u16;
    let y = layout.y as u16;
    let w = layout.width as u16;
    let h = layout.height as u16;

    // Background fill
    let bg = node.style.bg.unwrap_or(Color::BLACK);
    if bg != Color::BLACK {
        buf.fill_char(x, y, w, h, ' ', node.style.fg.unwrap_or(Color::WHITE), bg);
    }

    // Border
    if node.border.has_any() {
        let border_style = node.border.style.unwrap_or(node.style);
        let border_fg = border_style.fg.unwrap_or(Color::WHITE);
        buf.draw_border(x, y, w, h, border_fg, bg, node.title.as_deref());
    }

    // Paint children
    for (i, child) in node.children.iter().enumerate() {
        if let Some(child_layout) = layout.children.get(i) {
            paint_node(child, child_layout, buf);
        }
    }
}

fn paint_divider(node: &DividerNode, layout: &LayoutResult, buf: &mut ScreenBuffer) {
    let x = layout.x as u16;
    let y = layout.y as u16;
    let w = layout.width as u16;
    let fg = node.style.fg.unwrap_or(Color::WHITE);
    let bg = node.style.bg.unwrap_or(Color::BLACK);
    let ch = node.char.unwrap_or('─');
    buf.hline(x, y, w, ch, fg, bg);
}

fn paint_progress(node: &ProgressNode, layout: &LayoutResult, buf: &mut ScreenBuffer) {
    let x = layout.x as u16;
    let y = layout.y as u16;
    let w = node.width;
    let ratio = if node.max > 0.0 { (node.value / node.max).clamp(0.0, 1.0) } else { 0.0 };
    let filled = (w as f32 * ratio) as u16;

    let filled_fg = node.filled_style.fg.unwrap_or(Color::GREEN);
    let filled_bg = node.filled_style.bg.unwrap_or(Color::new(0, 80, 0));
    let empty_fg = node.empty_style.fg.unwrap_or(Color::new(80, 80, 80));
    let empty_bg = node.empty_style.bg.unwrap_or(Color::BLACK);

    for dx in 0..w {
        if dx < filled {
            buf.set_char(x + dx, y, '█', filled_fg, filled_bg, false, false, false, false, false, false);
        } else {
            buf.set_char(x + dx, y, '░', empty_fg, empty_bg, false, false, false, false, false, false);
        }
    }

    if node.show_percent {
        let pct = format!("{}%", (ratio * 100.0) as u8);
        let pct_x = x + w + 1;
        buf.write_str(pct_x, y, &pct, Color::WHITE, Color::BLACK, false, false);
    }
}

fn paint_list(node: &ListNode, layout: &LayoutResult, buf: &mut ScreenBuffer) {
    let x = layout.x as u16;
    let y = layout.y as u16;
    let fg = node.style.fg.unwrap_or(Color::WHITE);
    let bg = node.style.bg.unwrap_or(Color::BLACK);

    for (i, item) in node.items.iter().enumerate() {
        let item_y = y + i as u16;
        let marker = match node.marker {
            ListMarker::None => String::new(),
            ListMarker::Bullet => "• ".to_string(),
            ListMarker::Number => format!("{}.", i + 1),
            ListMarker::Dash => "- ".to_string(),
        };
        buf.write_str(x, item_y, &marker, fg, bg, false, false);

        if let Some(child_layout) = layout.children.get(i) {
            paint_node(item, child_layout, buf);
        }
    }
}

fn paint_table(node: &TableNode, layout: &LayoutResult, buf: &mut ScreenBuffer) {
    let x = layout.x as u16;
    let y = layout.y as u16;
    let w = layout.width as u16;
    let fg = node.style.fg.unwrap_or(Color::WHITE);
    let bg = node.style.bg.unwrap_or(Color::BLACK);
    let hdr_fg = node.header_style.fg.unwrap_or(Color::WHITE);

    let col_count = node.headers.len().max(1);
    let col_width = w / col_count as u16;

    // Headers
    for (i, header) in node.headers.iter().enumerate() {
        let cx = x + i as u16 * col_width;
        buf.write_str(cx, y, header, hdr_fg, bg, true, false);
    }

    // Separator
    if node.border {
        buf.hline(x, y + 1, w, '─', fg, bg);
    }

    // Rows
    for (row_i, row) in node.rows.iter().enumerate() {
        let ry = y + 2 + row_i as u16;
        for (col_i, cell) in row.iter().enumerate() {
            let cx = x + col_i as u16 * col_width;
            buf.write_str(cx, ry, cell, fg, bg, false, false);
        }
    }
}

fn paint_scroll_view(node: &ScrollNode, layout: &LayoutResult, buf: &mut ScreenBuffer) {
    let x = layout.x as u16;
    let y = layout.y as u16;
    let h = layout.height as u16;

    if let Some(child_layout) = layout.children.first() {
        // Offset child by scroll_top
        let mut adjusted = child_layout.clone();
        adjusted.y -= node.scroll_top as f32;
        paint_node(&node.child, &adjusted, buf);
    }

    // Draw scroll indicator
    let total_h = layout.children.first().map(|c| c.height as u16).unwrap_or(h);
    if total_h > h {
        let ratio = node.scroll_top as f32 / total_h as f32;
        let indicator_y = y + (ratio * h as f32) as u16;
        buf.set_char(x + layout.width as u16 - 1, indicator_y, '▐', Color::WHITE, Color::BLACK, false, false, false, false, false, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paint_text() {
        let node = UiNode::text("Hello");
        let buf = paint(&node, 20, 5);
        assert_eq!(buf.get(0, 0).ch, 'H');
        assert_eq!(buf.get(4, 0).ch, 'o');
    }

    #[test]
    fn test_paint_box_with_border() {
        let node = UiNode::boxed(vec![UiNode::text("Content")])
            .border(Border::all(None))
            .title("Test");
        let buf = paint(&node, 20, 5);
        assert_eq!(buf.get(0, 0).ch, '┌');
        assert_eq!(buf.get(19, 0).ch, '┐');
    }

    #[test]
    fn test_paint_column() {
        let node = UiNode::column(vec![
            UiNode::text("A"),
            UiNode::text("B"),
        ]);
        let buf = paint(&node, 20, 5);
        assert_eq!(buf.get(0, 0).ch, 'A');
        assert_eq!(buf.get(0, 1).ch, 'B');
    }

    #[test]
    fn test_paint_progress() {
        let node = UiNode::progress(50.0, 100.0).width(10);
        let buf = paint(&node, 30, 5);
        // First 5 should be filled, last 5 empty
        assert_eq!(buf.get(0, 0).ch, '█');
        assert_eq!(buf.get(5, 0).ch, '░');
    }

    #[test]
    fn test_paint_divider() {
        let node = UiNode::divider();
        let buf = paint(&node, 10, 3);
        for x in 0..10 {
            assert_eq!(buf.get(x, 0).ch, '─');
        }
    }

    #[test]
    fn test_paint_list() {
        let node = UiNode::list(vec![
            UiNode::text("Item 1"),
            UiNode::text("Item 2"),
        ]).marker(ListMarker::Bullet);
        let buf = paint(&node, 20, 5);
        assert_eq!(buf.get(0, 0).ch, '•');
        assert_eq!(buf.get(0, 1).ch, '•');
    }
}
