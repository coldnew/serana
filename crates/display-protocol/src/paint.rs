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
        // ── Editor / IDE / Agent widgets ──
        UiNode::Input(i) => paint_input(i, layout, buf),
        UiNode::TextArea(t) => paint_textarea(t, layout, buf),
        UiNode::TabBar(t) => paint_tab_bar(t, layout, buf),
        UiNode::TreeView(tv) => paint_tree_view(tv, layout, buf),
        UiNode::SplitPane(s) => paint_split_pane(s, layout, buf),
        UiNode::StatusBar(sb) => paint_status_bar(sb, layout, buf),
        // ── WGPU-only widgets ──
        UiNode::Canvas(c) => paint_canvas(c, layout, buf),
        UiNode::Overlay(o) => paint_overlay(o, layout, buf),
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

// ── Editor / IDE / Agent widget painters ──

fn paint_input(node: &InputNode, layout: &LayoutResult, buf: &mut ScreenBuffer) {
    let x = layout.x as u16;
    let y = layout.y as u16;
    let w = layout.width as u16;

    let fg = node.style.fg.unwrap_or(Color::WHITE);
    let bg = node.style.bg.unwrap_or(Color::BLACK);

    // Fill background
    buf.fill_char(x, y, w, 1, ' ', fg, bg);

    if node.value.is_empty() && !node.placeholder.is_empty() {
        // Show placeholder
        let ph_fg = Color::new(128, 128, 128);
        buf.write_str(x, y, &node.placeholder, ph_fg, bg, false, false);
    } else {
        buf.write_str(x, y, &node.value, fg, bg, node.style.bold, node.style.dim);
    }

    // Draw cursor
    if node.focused {
        let cx = x + node.cursor.min(w.saturating_sub(1));
        let cursor_fg = node.cursor_style.fg.unwrap_or(Color::BLACK);
        let cursor_bg = node.cursor_style.bg.unwrap_or(Color::WHITE);
        let ch = if (node.cursor as usize) < node.value.len() {
            node.value.chars().nth(node.cursor as usize).unwrap_or(' ')
        } else {
            ' '
        };
        buf.set_char(cx, y, ch, cursor_fg, cursor_bg, false, false, false, false, false, false);
    }
}

fn paint_textarea(node: &TextAreaNode, layout: &LayoutResult, buf: &mut ScreenBuffer) {
    let x = layout.x as u16;
    let y = layout.y as u16;
    let w = layout.width as u16;
    let h = node.height;

    let fg = node.style.fg.unwrap_or(Color::WHITE);
    let bg = node.style.bg.unwrap_or(Color::BLACK);

    let gutter_w: u16 = if node.gutter {
        let line_count = node.lines.len();
        format!("{}", line_count).len() as u16 + 1
    } else {
        0
    };

    let text_w = w.saturating_sub(gutter_w);

    for row in 0..h {
        let line_idx = node.scroll_top as usize + row as usize;
        let ry = y + row;

        // Clear line
        buf.fill_char(x, ry, w, 1, ' ', fg, bg);

        if line_idx >= node.lines.len() {
            continue;
        }

        // Gutter
        if node.gutter {
            let line_num = format!("{:>width$} ", line_idx + 1, width = gutter_w as usize - 1);
            let gutter_fg = Color::new(128, 128, 128);
            buf.write_str(x, ry, &line_num, gutter_fg, bg, false, false);
        }

        // Line content (scroll left)
        let line = &node.lines[line_idx];
        let visible: String = line.chars().skip(node.scroll_left as usize).take(text_w as usize).collect();
        buf.write_str(x + gutter_w, ry, &visible, fg, bg, node.style.bold, node.style.dim);
    }

    // Draw cursor
    if node.focused {
        let cursor_row = node.cursor_line.saturating_sub(node.scroll_top);
        let cursor_col = node.cursor_col.saturating_sub(node.scroll_left);
        if cursor_row < h && cursor_col < text_w {
            let cx = x + gutter_w + cursor_col;
            let cy = y + cursor_row;
            let cursor_fg = node.cursor_style.fg.unwrap_or(Color::BLACK);
            let cursor_bg = node.cursor_style.bg.unwrap_or(Color::WHITE);
            let ch = node.lines
                .get(node.cursor_line as usize)
                .and_then(|l| l.chars().nth(node.cursor_col as usize))
                .unwrap_or(' ');
            buf.set_char(cx, cy, ch, cursor_fg, cursor_bg, false, false, false, false, false, false);
        }
    }
}

fn paint_tab_bar(node: &TabBarNode, layout: &LayoutResult, buf: &mut ScreenBuffer) {
    let x = layout.x as u16;
    let y = layout.y as u16;
    let w = layout.width as u16;

    let fg = node.style.fg.unwrap_or(Color::WHITE);
    let bg = node.style.bg.unwrap_or(Color::BLACK);

    // Fill background
    buf.fill_char(x, y, w, 1, ' ', fg, bg);

    let mut cx = x;
    for (i, item) in node.items.iter().enumerate() {
        if cx >= x + w { break; }

        let is_active = i == node.active;

        let title = format!(" {}{} ", item.title, if item.modified { " ●" } else { "" });
        let title_w = title.len() as u16;
        if cx + title_w > x + w { break; }

        if is_active {
            let style = node.active_style;
            buf.write_styled(cx, y, &title, &style);
        } else {
            buf.write_str(cx, y, &title, fg, bg, false, false);
        }

        cx += title_w;
    }
}

fn paint_tree_view(node: &TreeViewNode, layout: &LayoutResult, buf: &mut ScreenBuffer) {
    let x = layout.x as u16;
    let w = layout.width as u16;

    let fg = node.style.fg.unwrap_or(Color::WHITE);
    let bg = node.style.bg.unwrap_or(Color::BLACK);
    let sel_fg = node.selected_style.fg.unwrap_or(Color::WHITE);
    let sel_bg = node.selected_style.bg.unwrap_or(Color::new(40, 60, 100));

    let mut row: u16 = 0;
    let mut flat_idx: usize = 0;

    fn paint_tree_item(
        item: &TreeItem,
        depth: u16,
        x: u16, w: u16,
        row: &mut u16,
        flat_idx: &mut usize,
        selected: Option<usize>,
        indent: u16,
        fg: Color, bg: Color,
        sel_fg: Color, sel_bg: Color,
        buf: &mut ScreenBuffer,
    ) {
        let y = *row;
        let is_selected = selected == Some(*flat_idx);
        let (item_fg, item_bg) = if is_selected { (sel_fg, sel_bg) } else { (fg, bg) };

        // Fill row
        buf.fill_char(x, y, w, 1, ' ', item_fg, item_bg);

        // Indent
        let prefix_x = x + depth * indent;

        // Expand/collapse indicator
        if !item.children.is_empty() {
            let indicator = if item.expanded { "▼ " } else { "▶ " };
            buf.write_str(prefix_x, y, indicator, item_fg, item_bg, false, false);
        } else {
            buf.write_str(prefix_x, y, "  ", item_fg, item_bg, false, false);
        }

        // Icon
        let label_x = prefix_x + 2;
        if let Some(ref icon) = item.icon {
            buf.write_str(label_x, y, icon, item_fg, item_bg, false, false);
            buf.write_str(label_x + icon.len() as u16 + 1, y, &item.label, item_fg, item_bg, false, false);
        } else {
            buf.write_str(label_x, y, &item.label, item_fg, item_bg, false, false);
        }

        *row += 1;
        *flat_idx += 1;

        if item.expanded {
            for child in &item.children {
                paint_tree_item(child, depth + 1, x, w, row, flat_idx, selected, indent, fg, bg, sel_fg, sel_bg, buf);
            }
        }
    }

    for item in &node.items {
        paint_tree_item(item, 0, x, w, &mut row, &mut flat_idx, node.selected, node.indent, fg, bg, sel_fg, sel_bg, buf);
    }
}

fn paint_split_pane(node: &SplitPaneNode, layout: &LayoutResult, buf: &mut ScreenBuffer) {
    // Paint first child
    if let Some(first_layout) = layout.children.first() {
        paint_node(&node.first, first_layout, buf);
    }

    // Paint divider
    let fg = node.divider_style.fg.unwrap_or(Color::new(80, 80, 80));
    let bg = node.divider_style.bg.unwrap_or(Color::BLACK);
    match node.orientation {
        Orientation::Horizontal => {
            let div_x = layout.x as u16 + (layout.width as f32 * node.ratio) as u16;
            let div_y = layout.y as u16;
            let div_h = layout.height as u16;
            for dy in 0..div_h {
                buf.set_char(div_x, div_y + dy, '│', fg, bg, false, false, false, false, false, false);
            }
        }
        Orientation::Vertical => {
            let div_y = layout.y as u16 + (layout.height as f32 * node.ratio) as u16;
            let div_x = layout.x as u16;
            let div_w = layout.width as u16;
            buf.hline(div_x, div_y, div_w, '─', fg, bg);
        }
    }

    // Paint second child
    if let Some(second_layout) = layout.children.get(1) {
        paint_node(&node.second, second_layout, buf);
    }
}

fn paint_status_bar(node: &StatusBarNode, layout: &LayoutResult, buf: &mut ScreenBuffer) {
    let x = layout.x as u16;
    let y = layout.y as u16;
    let w = layout.width as u16;

    let fg = node.style.fg.unwrap_or(Color::WHITE);
    let bg = node.style.bg.unwrap_or(Color::new(40, 40, 60));

    // Fill background
    buf.fill_char(x, y, w, 1, ' ', fg, bg);

    // Paint left items
    for (i, child) in node.left.iter().enumerate() {
        if let Some(child_layout) = layout.children.get(i) {
            paint_node(child, child_layout, buf);
        }
    }

    // Paint right items
    let right_offset = node.left.len();
    for (i, child) in node.right.iter().enumerate() {
        if let Some(child_layout) = layout.children.get(right_offset + i) {
            paint_node(child, child_layout, buf);
        }
    }
}

// ── WGPU-only widget painters ──

fn paint_canvas(_node: &CanvasNode, layout: &LayoutResult, _buf: &mut ScreenBuffer) {
    // TUI: reserve the rect but leave it blank.
    // The layout system already allocated width×height.
    // WGPU backend reads frame_id and renders pixel content separately.
    let _ = layout;
}

fn paint_overlay(node: &OverlayNode, layout: &LayoutResult, buf: &mut ScreenBuffer) {
    // TUI: paint child on top of existing content at absolute position.
    // No background clearing — overlay composites over existing cells.
    if let Some(child_layout) = layout.children.first() {
        paint_node(&node.child, child_layout, buf);
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

    #[test]
    fn test_paint_input() {
        let node = UiNode::input("hello").width(10).focused(true).cursor_pos(3);
        let buf = paint(&node, 20, 3);
        assert_eq!(buf.get(0, 0).ch, 'h');
        assert_eq!(buf.get(1, 0).ch, 'e');
        // Cursor at position 3 shows 'l' with inverted colors
        let cursor_cell = buf.get(3, 0);
        assert_eq!(cursor_cell.ch, 'l');
        assert_eq!(cursor_cell.fg, Color::BLACK);
        assert_eq!(cursor_cell.bg, Color::WHITE);
    }

    #[test]
    fn test_paint_input_placeholder() {
        let node = UiNode::input("").placeholder("type here");
        let buf = paint(&node, 20, 1);
        // Placeholder should render in gray
        assert_eq!(buf.get(0, 0).ch, 't');
        assert_eq!(buf.get(0, 0).fg, Color::new(128, 128, 128));
    }

    #[test]
    fn test_paint_textarea() {
        let node = UiNode::textarea(vec![
            "line one".into(),
            "line two".into(),
        ]).height(3).focused(true);
        let buf = paint(&node, 20, 3);
        assert_eq!(buf.get(0, 0).ch, 'l');
        assert_eq!(buf.get(0, 1).ch, 'l');
    }

    #[test]
    fn test_paint_textarea_with_gutter() {
        let node = UiNode::textarea(vec!["code".into()])
            .height(2).gutter(true);
        let buf = paint(&node, 20, 2);
        // Gutter shows "1 " for first line
        assert_eq!(buf.get(0, 0).ch, '1');
        assert_eq!(buf.get(1, 0).ch, ' ');
        // Code starts after gutter
        assert_eq!(buf.get(2, 0).ch, 'c');
    }

    #[test]
    fn test_paint_tab_bar() {
        let node = UiNode::tab_bar(vec![
            TabItem::new("file1.rs"),
            TabItem::new("file2.rs"),
        ]).active_tab(0);
        let buf = paint(&node, 40, 1);
        // Tab format: " file1.rs " with underline at row 0 for active tab
        assert_eq!(buf.get(1, 0).ch, 'f');
    }

    #[test]
    fn test_paint_tree_view() {
        let node = UiNode::tree_view(vec![
            TreeItem::new("src").children(vec![
                TreeItem::new("main.rs"),
            ]).expanded(true),
            TreeItem::new("Cargo.toml"),
        ]);
        let buf = paint(&node, 30, 5);
        // First row: "▼ src" (depth 0, prefix_x = x + 0 * indent)
        assert_eq!(buf.get(0, 0).ch, '▼');
        // Second row (expanded child): indented "  main.rs"
        assert_eq!(buf.get(2, 1).ch, ' ');
        assert_eq!(buf.get(4, 1).ch, 'm');
    }

    #[test]
    fn test_paint_split_pane() {
        let node = UiNode::split(
            Orientation::Horizontal,
            UiNode::text("left"),
            UiNode::text("right"),
        ).ratio(0.5);
        let buf = paint(&node, 20, 3);
        assert_eq!(buf.get(0, 0).ch, 'l');
        // Divider at midpoint
        assert_eq!(buf.get(10, 0).ch, '│');
        // Right content after divider
        assert_eq!(buf.get(11, 0).ch, 'r');
    }

    #[test]
    fn test_paint_canvas_blank() {
        let node = UiNode::canvas(10, 3, "test_frame");
        let buf = paint(&node, 20, 5);
        // Canvas should leave cells blank (default ' ')
        assert_eq!(buf.get(0, 0).ch, ' ');
        // But the area should be reserved
        let layout = crate::layout::compute_layout(&node, 20, 5);
        assert_eq!(layout.width, 10.0);
        assert_eq!(layout.height, 3.0);
    }

    #[test]
    fn test_paint_status_bar() {
        let node = UiNode::status_bar(
            vec![UiNode::text("left").bold()],
            vec![UiNode::text("right")],
        );
        let buf = paint(&node, 20, 1);
        assert_eq!(buf.get(0, 0).ch, 'l');
    }

    #[test]
    fn test_paint_overlay() {
        let node = UiNode::overlay(
            UiNode::boxed(vec![UiNode::text("popup")]),
            5, 2,
        );
        let layout = crate::layout::compute_layout(&node, 40, 20);
        assert_eq!(layout.x, 5.0);
        assert_eq!(layout.y, 2.0);
    }
}
