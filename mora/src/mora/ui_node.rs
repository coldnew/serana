use display_protocol::{UiNode, Color};
use display_protocol_jsx::jsx;
use super::editor::MoraEditor;
use super::keymap::EditorMode;
use super::syntax;
use super::lisp_ext;

/// Build a UiNode tree from the current editor state with an Emacs-like UI.
///
/// Layout (top to bottom):
///   - Editor area (buffer content with line numbers)
///   - Modeline (inverse, shows filename, modified, position, percentage)
///   - Echo area / minibuffer (when active)
///
/// If Lisp UI builders are registered, they augment/replace the default UI.
pub fn build_ui(editor: &MoraEditor, width: u16, height: u16) -> UiNode {
    if let Some(lisp_ui) = lisp_ext::build_lisp_ui(width, height) {
        return lisp_ui;
    }

    let modeline_height = 1u16;
    let cmd_height: u16 = if editor.minibuffer_active() { 1 } else { 0 };
    let editor_height = height.saturating_sub(modeline_height + cmd_height);

    let mut children = Vec::with_capacity(3);
    children.push(build_editor_area(editor, width, editor_height));
    children.push(build_modeline(editor, width));

    if cmd_height > 0 {
        children.push(build_echo_area(editor, width));
    }

    UiNode::column(children)
}

/// Emacs-style modeline: inverse video bar showing buffer name, modified flag,
/// position, and percentage.
fn build_modeline(editor: &MoraEditor, width: u16) -> UiNode {
    let buf = editor.buffer();
    let filename = buf.filename();
    let modified = if buf.modified { "**" } else { "--" };

    let line = buf.cursor.row + 1;
    let col = buf.cursor.col + 1;
    let total_lines = buf.line_count().max(1);
    let pct = if total_lines <= 1 {
        100
    } else {
        (line * 100) / total_lines
    };

    let mode_str = match editor.mode() {
        EditorMode::Normal => "",
        EditorMode::Insert => " Isearch",
        EditorMode::Emacs => "",
        EditorMode::Command | EditorMode::SearchForward | EditorMode::SearchBackward => {
            " Minibuf"
        }
        EditorMode::ReplaceChar => " Replace",
        EditorMode::Visual => " Select",
        EditorMode::Iedit => " Iedit",
    };

    let left = format!(" {}{} ", modified, filename);
    let right = format!(" L{}:C{} ({}%){} ", line, col, pct, mode_str);

    let left_len = left.len();
    let right_len = right.len();
    let fill = (width as usize).saturating_sub(left_len + right_len);
    let dashes = "-".repeat(fill);

    let modeline_str = format!("{}{}{}", left, dashes, right);

    let fg = Color::new(200, 200, 200);
    let bg = Color::new(40, 42, 54);

    jsx! {
        <Text color={fg} bg={bg} bold>{modeline_str}</Text>
    }
}

/// Echo area / minibuffer line
fn build_echo_area(editor: &MoraEditor, width: u16) -> UiNode {
    let prompt = editor.minibuffer_prompt();
    let input = editor.command_input();

    let prompt_fg = Color::new(80, 160, 240);
    let input_fg = Color::new(232, 236, 244);

    let mut row = Vec::with_capacity(2);
    row.push(UiNode::text(prompt).color(prompt_fg).bold());
    row.push(UiNode::text(input.to_string()).color(input_fg));
    UiNode::row(row).width(width)
}

fn style_to_color(
    s: super::display::style::MoraStyle,
) -> (Option<Color>, Option<Color>, bool, bool, bool, bool, bool, bool) {
    let fg = s.fg.map(|c| Color::new(c.r, c.g, c.b));
    let bg = s.bg.map(|c| Color::new(c.r, c.g, c.b));
    (fg, bg, s.bold, s.italic, s.underline, s.strikethrough, s.dim, s.reverse)
}

fn syntax_style_for_col(
    tokens: &[syntax::HighlightToken],
    col: usize,
) -> Option<super::display::style::MoraStyle> {
    for tok in tokens {
        if col >= tok.start && col < tok.end {
            return Some(syntax::style_for_kind(tok.kind));
        }
    }
    None
}

/// Build editor area with syntax highlighting, line numbers, cursor, and selection.
fn build_editor_area(editor: &MoraEditor, width: u16, height: u16) -> UiNode {
    let buf = editor.buffer();
    let view = editor.view();
    let gutter_w = view.gutter_width;
    let text_width = width.saturating_sub(gutter_w);
    let narrow_start = buf.narrow_start.unwrap_or(0);
    let narrow_end = buf.narrow_end.unwrap_or(buf.line_count().saturating_sub(1));
    let total = buf.line_count();
    let (vis_start, vis_end) = view.visible_range(total);
    let render_start = vis_start.max(narrow_start);
    let render_end = vis_end.min(narrow_end + 1);

    let full_content: String = buf.lines.join("\n");
    let highlighter = syntax::create_highlighter(buf.major_mode.name(), &full_content);

    let gutter_dim = Color::new(107, 114, 128);
    let gutter_current = Color::new(0, 180, 255);
    let text_fg = Color::new(232, 236, 244);
    let current_line_fg = Color::new(0, 180, 255);
    let tilde_color = Color::new(55, 60, 72);
    let cursor_fg = Color::new(15, 18, 22);
    let cursor_bg = Color::new(0, 180, 255);
    let sel_bg = Color::new(30, 80, 120);

    let in_visual = editor.mode() == EditorMode::Visual && editor.mark_ring.is_active();
    let (sel_start, sel_end) = if in_visual {
        if let Some(mark) = editor.mark_ring.peek() {
            let cursor = buf.cursor;
            let (a, b) = if mark.row < cursor.row
                || (mark.row == cursor.row && mark.col <= cursor.col)
            {
                (*mark, cursor)
            } else {
                (cursor, *mark)
            };
            (Some(a), Some(b))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let mut rows = Vec::new();
    let mut display_row: u16 = 0;

    for line_idx in render_start..render_end {
        if buf.is_line_folded(line_idx) {
            continue;
        }
        if display_row >= height {
            break;
        }

        let is_current = line_idx == buf.cursor.row;

        let gutter_c = if is_current { gutter_current } else { gutter_dim };
        let line_num = if is_current {
            format!("{:>w$}", line_idx + 1, w = gutter_w as usize - 1)
        } else {
            format!("{:>w$} ", line_idx + 1, w = gutter_w as usize - 1)
        };

        let mut spans = vec![
            UiNode::text(line_num).color(gutter_c).bold(),
            UiNode::text(" ").color(gutter_c),
        ];

        let line_text = buf.line(line_idx);
        let highlight_tokens = highlighter.highlight_line(line_text, line_idx);
        let line_byte_offset = buf.byte_offset(line_idx, 0);

        let mut byte_col = 0usize;
        let mut display_col: u16 = 0;
        let mut span_text = String::new();
        let mut span_fg: Option<Color> = None;
        let mut span_bg: Option<Color> = None;
        let mut span_bold = false;

        let flush_span =
            |spans: &mut Vec<UiNode>,
             text: &mut String,
             fg: Option<Color>,
             bg: Option<Color>,
             bold: bool| {
                if !text.is_empty() {
                    let mut node = UiNode::text(std::mem::take(text));
                    if let Some(c) = fg {
                        node = node.color(c);
                    }
                    if let Some(c) = bg {
                        node = node.bg(c);
                    }
                    if bold {
                        node = node.bold();
                    }
                    spans.push(node);
                }
            };

        for ch in line_text.chars() {
            if display_col >= text_width {
                break;
            }

            let is_cursor = is_current
                && byte_col == buf.cursor.col
                && editor.mode() != EditorMode::Normal;
            let in_sel = sel_start.map_or(false, |start| {
                sel_end.map_or(false, |end| {
                    line_idx > start.row && line_idx < end.row
                        || (line_idx == start.row
                            && line_idx == end.row
                            && byte_col >= start.col
                            && byte_col < end.col)
                        || (line_idx == start.row && byte_col >= start.col)
                        || (line_idx == end.row && byte_col < end.col)
                })
            });

            let is_invisible = buf.overlays.is_invisible(line_byte_offset + byte_col);
            if is_invisible {
                byte_col += ch.len_utf8();
                continue;
            }

            let (ch_fg, ch_bg, ch_bold) = if is_cursor {
                (Some(cursor_fg), Some(cursor_bg), true)
            } else if in_sel {
                (Some(text_fg), Some(sel_bg), false)
            } else if let Some(syn_style) = syntax_style_for_col(&highlight_tokens, byte_col) {
                let (fg, bg, bold, ..) = style_to_color(syn_style);
                (fg, bg, bold)
            } else if is_current {
                (Some(current_line_fg), None, false)
            } else {
                (Some(text_fg), None, false)
            };

            if ch_fg != span_fg || ch_bg != span_bg || ch_bold != span_bold {
                flush_span(&mut spans, &mut span_text, span_fg, span_bg, span_bold);
                span_fg = ch_fg;
                span_bg = ch_bg;
                span_bold = ch_bold;
            }

            if ch == '\t' {
                let tab_stop = 4 - (display_col % 4);
                for _ in 0..tab_stop {
                    if display_col >= text_width {
                        break;
                    }
                    span_text.push(' ');
                    display_col += 1;
                }
            } else {
                span_text.push(ch);
                display_col += 1;
            }

            byte_col += ch.len_utf8();
        }

        flush_span(&mut spans, &mut span_text, span_fg, span_bg, span_bold);

        if is_current {
            while display_col < text_width {
                spans.push(UiNode::text(" ").bg(Color::new(25, 28, 35)));
                display_col += 1;
            }
        }

        rows.push(UiNode::row(spans));
        display_row += 1;
    }

    for _ in display_row..height {
        rows.push(UiNode::row(vec![UiNode::text("~").color(tilde_color)]));
    }

    UiNode::column(rows).width(width)
}
