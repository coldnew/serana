use crate::mora::display::style::MoraStyle;
use crate::mora::editor::MoraEditor;
use crate::mora::keymap::EditorMode;
use crate::mora::syntax;
use display_protocol::{Color, UiNode};

use super::{jsx_column, jsx_row};

fn style_to_color(
    s: MoraStyle,
) -> (
    Option<Color>,
    Option<Color>,
    bool,
    bool,
    bool,
    bool,
    bool,
    bool,
) {
    let fg = s.fg;
    let bg = s.bg;
    (
        fg,
        bg,
        s.bold,
        s.italic,
        s.underline,
        s.strikethrough,
        s.dim,
        s.reverse,
    )
}

fn syntax_style_for_col(tokens: &[syntax::HighlightToken], col: usize) -> Option<MoraStyle> {
    for tok in tokens {
        if col >= tok.start && col < tok.end {
            return Some(syntax::style_for_kind(tok.kind));
        }
    }
    None
}

/// Build editor area with syntax highlighting, cursor, and selection.
pub(super) fn build_editor_area(editor: &MoraEditor, width: u16, height: u16) -> UiNode {
    let buf = editor.buffer();
    let view = editor.view();
    let show_line_numbers = editor.minor_modes.is_enabled("line-numbers")
        || editor.minor_modes.is_enabled("relative-line-numbers");
    let gutter_w = if show_line_numbers {
        view.gutter_width
    } else {
        0
    };
    let text_width = width.saturating_sub(gutter_w);
    let narrow_start = buf.narrow_start.unwrap_or(0);
    let narrow_end = buf.narrow_end.unwrap_or(buf.line_count().saturating_sub(1));
    let total = buf.line_count();
    let (vis_start, vis_end) = view.visible_range(total);
    let render_start = vis_start.max(narrow_start);
    let render_end = vis_end.min(narrow_end + 1);

    let full_content: String = buf.lines.join("\n");
    let highlighter = syntax::create_highlighter(buf.major_mode.name(), &full_content);

    let t = &editor.theme;
    let text_fg = t.foreground;
    let current_line_fg = t.foreground;
    let cursor_fg = t.background;
    let cursor_bg = t.cursor;
    let sel_bg = t.selection;
    let in_visual = editor.mode() == EditorMode::Visual && editor.mark_ring.is_active();
    let (sel_start, sel_end) = if in_visual {
        if let Some(mark) = editor.mark_ring.peek() {
            let cursor = buf.cursor;
            let (a, b) =
                if mark.row < cursor.row || (mark.row == cursor.row && mark.col <= cursor.col) {
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
        let mut spans = Vec::new();
        if show_line_numbers {
            let gutter_dim = t.gutter_dim;
            let gutter_current = t.gutter_current;
            let gutter_c = if is_current {
                gutter_current
            } else {
                gutter_dim
            };
            let line_num = if is_current {
                format!("{:>w$}", line_idx + 1, w = gutter_w as usize - 1)
            } else {
                format!("{:>w$} ", line_idx + 1, w = gutter_w as usize - 1)
            };

            spans.push(UiNode::text(line_num).color(gutter_c).bold());
            spans.push(UiNode::text(" ").color(gutter_c));
        }

        let line_text = buf.line(line_idx);
        let highlight_tokens = highlighter.highlight_line(line_text, line_idx);
        let line_byte_offset = buf.byte_offset(line_idx, 0);

        let mut byte_col = 0usize;
        let mut display_col: u16 = 0;
        let mut span_text = String::new();
        let mut span_fg: Option<Color> = None;
        let mut span_bg: Option<Color> = None;
        let mut span_bold = false;

        let flush_span = |spans: &mut Vec<UiNode>,
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

            let is_cursor =
                is_current && byte_col == buf.cursor.col && editor.mode() != EditorMode::Normal;
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

        rows.push(jsx_row(spans, width));
        display_row += 1;
    }

    // Emacs: empty lines beyond buffer content are blank (no ~ marker).
    // The ~ prefix is a Vim convention, not Emacs.
    for _ in display_row..height {
        rows.push(jsx_row(vec![UiNode::text(" ")], width));
    }

    jsx_column(rows, width, height)
}
