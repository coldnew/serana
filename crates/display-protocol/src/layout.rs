use crate::ui::*;

/// Computed layout result for a node.
#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub children: Vec<LayoutResult>,
}

/// Compute layout for a UiNode tree within the given bounds.
///
/// Simple flexbox-like layout engine. Rows lay out children horizontally,
/// columns lay out children vertically, boxes wrap children in a container.
pub fn compute_layout(node: &UiNode, max_width: u16, max_height: u16) -> LayoutResult {
    let mut result = LayoutResult {
        x: 0.0,
        y: 0.0,
        width: max_width as f32,
        height: max_height as f32,
        children: Vec::new(),
    };
    layout_node(node, &mut result, max_width, max_height, 0.0, 0.0);
    result
}

fn layout_node(
    node: &UiNode,
    result: &mut LayoutResult,
    available_width: u16,
    available_height: u16,
    offset_x: f32,
    offset_y: f32,
) {
    match node {
        UiNode::Text(t) => {
            let content_width = text_width(&t.content);
            let wrap_width = match t.wrap {
                Wrap::NoWrap | Wrap::Truncate => content_width as f32,
                Wrap::Wrap => available_width as f32,
            };
            let lines = wrap_text(&t.content, wrap_width as u16);
            result.x = offset_x;
            result.y = offset_y;
            result.width = wrap_width.min(content_width as f32);
            result.height = lines.len() as f32;
        }

        UiNode::Span(s) => {
            let w = text_width(&s.content) as f32;
            result.x = offset_x;
            result.y = offset_y;
            result.width = w;
            result.height = 1.0;
        }

        UiNode::Box(b) => {
            let pad = &b.padding;
            let border_h: u16 = if b.border.has_any() { 1 } else { 0 };

            let inner_w = b.width
                .unwrap_or(available_width)
                .saturating_sub(pad.horizontal_total() + border_h * 2);
            let inner_h = b.height.unwrap_or(available_height);

            result.x = offset_x;
            result.y = offset_y;
            result.width = inner_w as f32 + pad.horizontal_total() as f32 + (border_h * 2) as f32;
            result.height = inner_h as f32;

            let content_x = offset_x + pad.left as f32 + border_h as f32;
            let mut content_y = offset_y + pad.top as f32 + border_h as f32;

            for child in &b.children {
                let mut child_result = LayoutResult {
                    x: 0.0, y: 0.0, width: 0.0, height: 0.0, children: Vec::new(),
                };
                layout_node(child, &mut child_result, inner_w, inner_h, content_x, content_y);
                content_y += child_result.height;
                result.children.push(child_result);
            }

            // Expand height if auto-sized
            if b.height.is_none() {
                let total_h = content_y - offset_y + pad.bottom as f32 + border_h as f32;
                result.height = total_h;
            }
        }

        UiNode::Row(f) => {
            let pad = &f.padding;
            let avail_w = f.width.unwrap_or(available_width).saturating_sub(pad.horizontal_total());
            let avail_h = f.height.unwrap_or(available_height);

            result.x = offset_x;
            result.y = offset_y;
            result.width = f.width.unwrap_or(available_width) as f32;
            result.height = f.height.unwrap_or(1) as f32;

            // Measure fixed children, count flex-grow
            let mut fixed_total: f32 = 0.0;
            let mut flex_grow_total: f32 = 0.0;
            let child_count = f.children.len();

            for child in &f.children {
                let (fixed_w, fg) = measure_child_fixed_width(child);
                if fg > 0.0 {
                    flex_grow_total += fg;
                } else {
                    fixed_total += fixed_w;
                }
            }

            let gap_total = if child_count > 1 { f.gap * (child_count as u16 - 1) } else { 0 };
            let remaining = (avail_w as f32 - gap_total as f32 - fixed_total).max(0.0);

            let mut cx = offset_x + pad.left as f32;
            let cy = offset_y + pad.top as f32;

            for child in &f.children {
                let (fixed_w, fg) = measure_child_fixed_width(child);
                let w = if fg > 0.0 { remaining * fg / flex_grow_total } else { fixed_w };

                let mut child_result = LayoutResult {
                    x: 0.0, y: 0.0, width: w, height: 0.0, children: Vec::new(),
                };
                layout_node(child, &mut child_result, w as u16, avail_h, cx, cy);
                child_result.width = w;
                result.children.push(child_result);
                cx += w + f.gap as f32;
            }

            if f.height.is_none() {
                let max_h = result.children.iter().map(|c| c.height).fold(0.0_f32, f32::max);
                result.height = max_h + pad.vertical_total() as f32;
            }
        }

        UiNode::Column(f) => {
            let pad = &f.padding;
            let avail_w = f.width.unwrap_or(available_width).saturating_sub(pad.horizontal_total());
            let avail_h = f.height.unwrap_or(available_height);

            result.x = offset_x;
            result.y = offset_y;
            result.width = f.width.unwrap_or(available_width) as f32;
            result.height = f.height.unwrap_or(avail_h) as f32;

            let mut cy = offset_y + pad.top as f32;
            let cx = offset_x + pad.left as f32;

            for child in &f.children {
                let mut child_result = LayoutResult {
                    x: 0.0, y: 0.0, width: 0.0, height: 0.0, children: Vec::new(),
                };
                layout_node(child, &mut child_result, avail_w, avail_h, cx, cy);
                let h = child_result.height;
                result.children.push(child_result);
                cy += h + f.gap as f32;
            }

            if f.height.is_none() {
                let total_h = cy - offset_y - f.gap as f32 + pad.bottom as f32;
                result.height = total_h;
            }
        }

        UiNode::Divider(d) => {
            result.x = offset_x;
            result.y = offset_y;
            result.width = available_width as f32;
            result.height = 1.0;
            let _ = d;
        }

        UiNode::ProgressBar(p) => {
            result.x = offset_x;
            result.y = offset_y;
            result.width = p.width as f32;
            result.height = 1.0;
        }

        UiNode::List(l) => {
            result.x = offset_x;
            result.y = offset_y;
            result.width = available_width as f32;
            result.height = l.items.len() as f32;

            let mut cy = offset_y;
            for item in &l.items {
                let mut child_result = LayoutResult {
                    x: 0.0, y: 0.0, width: 0.0, height: 0.0, children: Vec::new(),
                };
                layout_node(item, &mut child_result, available_width.saturating_sub(2), available_height, offset_x + 2.0, cy);
                let h = child_result.height.max(1.0);
                result.children.push(child_result);
                cy += h;
            }
        }

        UiNode::ListItem(child) => {
            let mut child_result = LayoutResult {
                x: 0.0, y: 0.0, width: 0.0, height: 0.0, children: Vec::new(),
            };
            layout_node(child, &mut child_result, available_width, available_height, offset_x, offset_y);
            *result = child_result;
        }

        UiNode::Table(t) => {
            let col_count = t.headers.len().max(1);
            let col_width = available_width / col_count as u16;
            let row_count = t.rows.len() + 1; // +1 for header

            result.x = offset_x;
            result.y = offset_y;
            result.width = available_width as f32;
            result.height = row_count as f32;
            let _ = col_width;
        }

        UiNode::ScrollView(s) => {
            result.x = offset_x;
            result.y = offset_y;
            result.width = s.viewport_width as f32;
            result.height = s.viewport_height as f32;

            let mut child_result = LayoutResult {
                x: 0.0, y: 0.0, width: 0.0, height: 0.0, children: Vec::new(),
            };
            layout_node(&s.child, &mut child_result, available_width, available_height, offset_x, offset_y);
            result.children.push(child_result);
        }

        UiNode::Show { when, child } => {
            if *when {
                layout_node(child, result, available_width, available_height, offset_x, offset_y);
            } else {
                result.x = offset_x;
                result.y = offset_y;
                result.width = 0.0;
                result.height = 0.0;
            }
        }

        UiNode::For { children } => {
            result.x = offset_x;
            result.y = offset_y;
            result.width = available_width as f32;
            result.height = 0.0;

            let mut cy = offset_y;
            for child in children {
                let mut child_result = LayoutResult {
                    x: 0.0, y: 0.0, width: 0.0, height: 0.0, children: Vec::new(),
                };
                layout_node(child, &mut child_result, available_width, available_height, offset_x, cy);
                let h = child_result.height;
                result.children.push(child_result);
                cy += h;
            }
            result.height = cy - offset_y;
        }

        // ── Editor / IDE / Agent widgets ──

        UiNode::Input(i) => {
            let w = i.width.unwrap_or(available_width);
            result.x = offset_x;
            result.y = offset_y;
            result.width = w as f32;
            result.height = 1.0;
        }

        UiNode::TextArea(t) => {
            let visible_lines = t.height;
            result.x = offset_x;
            result.y = offset_y;
            result.width = available_width as f32;
            result.height = visible_lines as f32;
        }

        UiNode::TabBar(t) => {
            result.x = offset_x;
            result.y = offset_y;
            result.width = available_width as f32;
            result.height = 1.0;
            let _ = t;
        }

        UiNode::TreeView(tv) => {
            let count: usize = tv.items.iter().map(|item| item.visible_count()).sum();
            result.x = offset_x;
            result.y = offset_y;
            result.width = available_width as f32;
            result.height = count as f32;
        }

        UiNode::SplitPane(s) => {
            result.x = offset_x;
            result.y = offset_y;
            result.width = available_width as f32;
            result.height = available_height as f32;

            let (first_w, first_h, second_w, second_h) = match s.orientation {
                Orientation::Horizontal => {
                    let first_w = (available_width as f32 * s.ratio) as u16;
                    let second_w = available_width.saturating_sub(first_w).saturating_sub(1);
                    (first_w, available_height, second_w, available_height)
                }
                Orientation::Vertical => {
                    let first_h = (available_height as f32 * s.ratio) as u16;
                    let second_h = available_height.saturating_sub(first_h).saturating_sub(1);
                    (available_width, first_h, available_width, second_h)
                }
            };

            let mut first_result = LayoutResult {
                x: 0.0, y: 0.0, width: 0.0, height: 0.0, children: Vec::new(),
            };
            layout_node(&s.first, &mut first_result, first_w, first_h, offset_x, offset_y);
            result.children.push(first_result);

            let (second_ox, second_oy) = match s.orientation {
                Orientation::Horizontal => (offset_x + first_w as f32 + 1.0, offset_y),
                Orientation::Vertical => (offset_x, offset_y + first_h as f32 + 1.0),
            };

            let mut second_result = LayoutResult {
                x: 0.0, y: 0.0, width: 0.0, height: 0.0, children: Vec::new(),
            };
            layout_node(&s.second, &mut second_result, second_w, second_h, second_ox, second_oy);
            result.children.push(second_result);
        }

        UiNode::StatusBar(sb) => {
            result.x = offset_x;
            result.y = offset_y;
            result.width = available_width as f32;
            result.height = 1.0;

            // Layout left items
            let mut lx = offset_x;
            for child in &sb.left {
                let mut child_result = LayoutResult {
                    x: 0.0, y: 0.0, width: 0.0, height: 0.0, children: Vec::new(),
                };
                layout_node(child, &mut child_result, available_width, 1, lx, offset_y);
                lx += child_result.width;
                result.children.push(child_result);
            }
            // Layout right items (from right edge)
            let mut rx = offset_x + available_width as f32;
            for child in &sb.right {
                let mut child_result = LayoutResult {
                    x: 0.0, y: 0.0, width: 0.0, height: 0.0, children: Vec::new(),
                };
                layout_node(child, &mut child_result, available_width, 1, rx, offset_y);
                rx -= child_result.width;
                result.children.push(child_result);
            }
        }

        // ── WGPU-only widgets ──

        UiNode::Canvas(c) => {
            result.x = offset_x;
            result.y = offset_y;
            result.width = c.width as f32;
            result.height = c.height as f32;
        }

        UiNode::Overlay(o) => {
            let mut child_result = LayoutResult {
                x: 0.0, y: 0.0, width: 0.0, height: 0.0, children: Vec::new(),
            };
            layout_node(&o.child, &mut child_result, available_width, available_height, 0.0, 0.0);
            result.x = o.x as f32;
            result.y = o.y as f32;
            result.width = child_result.width;
            result.height = child_result.height;
            result.children.push(child_result);
        }

        UiNode::None => {
            result.x = offset_x;
            result.y = offset_y;
            result.width = 0.0;
            result.height = 0.0;
        }
    }
}

/// Measure a child's fixed width and flex-grow factor.
fn measure_child_fixed_width(node: &UiNode) -> (f32, f32) {
    match node {
        UiNode::Text(t) => (text_width(&t.content) as f32, 0.0),
        UiNode::Span(s) => (text_width(&s.content) as f32, 0.0),
        UiNode::Box(b) => (b.width.map(|w| w as f32).unwrap_or(0.0), 0.0),
        UiNode::Row(f) | UiNode::Column(f) => (f.width.map(|w| w as f32).unwrap_or(0.0), f.flex_grow),
        UiNode::Divider(_) => (0.0, 1.0), // flex-grow by default
        UiNode::ProgressBar(p) => (p.width as f32, 0.0),
        UiNode::List(_) => (0.0, 1.0),
        UiNode::ListItem(c) => measure_child_fixed_width(c),
        UiNode::Table(_) => (0.0, 1.0),
        UiNode::ScrollView(_) => (0.0, 1.0),
        UiNode::Show { when, child } => {
            if *when { measure_child_fixed_width(child) } else { (0.0, 0.0) }
        }
        UiNode::For { .. } => (0.0, 1.0),
        UiNode::Input(i) => (i.width.map(|w| w as f32).unwrap_or(0.0), 0.0),
        UiNode::TextArea(_) => (0.0, 1.0),
        UiNode::TabBar(_) => (0.0, 1.0),
        UiNode::TreeView(_) => (0.0, 1.0),
        UiNode::SplitPane(_) => (0.0, 1.0),
        UiNode::StatusBar(_) => (0.0, 1.0),
        UiNode::Canvas(c) => (c.width as f32, 0.0),
        UiNode::Overlay(_) => (0.0, 0.0),
        UiNode::None => (0.0, 0.0),
    }
}

/// Calculate display width of a string, accounting for CJK wide characters.
pub fn text_width(s: &str) -> u16 {
    s.chars().map(char_display_width).sum::<usize>() as u16
}

/// Display width of a single character. CJK ideographs, fullwidth forms,
/// and certain emoji are 2 cells wide; everything else is 1.
pub fn char_display_width(ch: char) -> usize {
    let cp = ch as u32;
    match cp {
        // CJK Unified Ideographs
        0x4E00..=0x9FFF => 2,
        // CJK Unified Ideographs Extension A
        0x3400..=0x4DBF => 2,
        // CJK Unified Ideographs Extension B
        0x20000..=0x2A6DF => 2,
        // CJK Compatibility Ideographs
        0xF900..=0xFAFF => 2,
        // Fullwidth Forms
        0xFF01..=0xFF60 => 2,
        0xFFE0..=0xFFE6 => 2,
        // CJK Radicals / Kangxi Radicals
        0x2E80..=0x2FDF => 2,
        // Hiragana / Katakana (some fonts render wide)
        0x3040..=0x309F => 2,
        0x30A0..=0x30FF => 2,
        // Bopomofo
        0x3100..=0x312F => 2,
        // Enclosed CJK
        0x3200..=0x32FF => 2,
        // CJK Compatibility Forms
        0xFE30..=0xFE4F => 2,
        // Emoji (most are wide)
        0x1F300..=0x1F9FF => 2,
        0x2600..=0x26FF => 1, // Misc symbols (some narrow)
        0x2700..=0x27BF => 1, // Dingbats (some narrow)
        _ => 1,
    }
}

/// Wrap text into lines that fit within the given width.
pub fn wrap_text(text: &str, width: u16) -> Vec<String> {
    if width == 0 { return vec![text.to_string()]; }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width: u16 = 0;

    for word in text.split_whitespace() {
        let word_width = word.chars().count() as u16;
        if current_width > 0 && current_width + 1 + word_width > width {
            lines.push(std::mem::take(&mut current_line));
            current_width = 0;
        }
        if current_width > 0 {
            current_line.push(' ');
            current_width += 1;
        }
        current_line.push_str(word);
        current_width += word_width;
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Color;

    #[test]
    fn test_text_layout() {
        let node = UiNode::text("Hello World");
        let layout = compute_layout(&node, 80, 24);
        assert_eq!(layout.x, 0.0);
        assert_eq!(layout.y, 0.0);
        assert_eq!(layout.width, 11.0);
        assert_eq!(layout.height, 1.0);
    }

    #[test]
    fn test_column_layout() {
        let node = UiNode::column(vec![
            UiNode::text("Line 1"),
            UiNode::text("Line 2"),
        ]);
        let layout = compute_layout(&node, 80, 24);
        assert_eq!(layout.children.len(), 2);
        assert_eq!(layout.children[0].y, 0.0);
        assert_eq!(layout.children[1].y, 1.0);
    }

    #[test]
    fn test_box_with_padding() {
        let node = UiNode::boxed(vec![UiNode::text("Content")])
            .padding(Padding::all(1));
        let layout = compute_layout(&node, 80, 24);
        assert_eq!(layout.children.len(), 1);
        // Child should be offset by padding
        assert_eq!(layout.children[0].x, 1.0);
        assert_eq!(layout.children[0].y, 1.0);
    }

    #[test]
    fn test_row_layout() {
        let node = UiNode::row(vec![
            UiNode::text("A"),
            UiNode::text("B"),
            UiNode::text("C"),
        ]).gap(1);
        let layout = compute_layout(&node, 80, 24);
        assert_eq!(layout.children.len(), 3);
        assert_eq!(layout.children[0].x, 0.0);
        assert_eq!(layout.children[1].x, 2.0); // 1 + gap(1)
        assert_eq!(layout.children[2].x, 4.0); // 1 + gap(1) + 1 + gap(1)
    }

    #[test]
    fn test_show_hidden() {
        let node = UiNode::show(false, UiNode::text("hidden"));
        let layout = compute_layout(&node, 80, 24);
        assert_eq!(layout.width, 0.0);
        assert_eq!(layout.height, 0.0);
    }

    #[test]
    fn test_wrap_text() {
        let lines = wrap_text("Hello World Foo Bar", 10);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "Hello");
        assert_eq!(lines[1], "World Foo");
        assert_eq!(lines[2], "Bar");
    }

    #[test]
    fn test_cjk_text_width() {
        assert_eq!(text_width("Hello"), 5);
        assert_eq!(text_width("你好"), 4); // 2 CJK chars = 4 cells
        assert_eq!(text_width("Hi你好"), 6); // 2 ASCII + 2 CJK = 6
        assert_eq!(text_width("Ａ"), 2); // Fullwidth A
    }
}
