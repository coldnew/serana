use crate::mora::display::style::MoraStyle;
use crate::mora::editor::MoraEditor;
use crate::mora::keymap::EditorMode;
use crate::mora::syntax;
use display_protocol::{Color, Selection, SelectionMode, Style, StyledLine, StyledSpan, UiNode};

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
pub(super) fn build_editor_area(editor: &MoraEditor, _width: u16, height: u16) -> UiNode {
    let buf = editor.buffer();
    let view = editor.view();
    let show_line_numbers = editor.minor_modes.is_enabled("line-numbers")
        || editor.minor_modes.is_enabled("relative-line-numbers");
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

    let mut lines = Vec::new();

    for line_idx in render_start..render_end {
        if buf.is_line_folded(line_idx) {
            continue;
        }

        let is_current = line_idx == buf.cursor.row;
        let mut spans = Vec::new();

        let line_text = buf.line(line_idx);
        let highlight_tokens = highlighter.highlight_line(line_text, line_idx);
        let line_byte_offset = buf.byte_offset(line_idx, 0);

        let mut byte_col = 0usize;
        let mut display_col: u16 = 0;
        let mut span_text = String::new();
        let mut span_style = Style::default();

        let flush_span = |spans: &mut Vec<StyledSpan>, text: &mut String, style: Style| {
            if !text.is_empty() {
                spans.push(StyledSpan::new(std::mem::take(text), style));
            }
        };

        for ch in line_text.chars() {
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

            let next_style = Style {
                fg: ch_fg,
                bg: ch_bg,
                bold: ch_bold,
                ..Style::default()
            };

            if next_style != span_style {
                flush_span(&mut spans, &mut span_text, span_style);
                span_style = next_style;
            }

            if ch == '\t' {
                let tab_stop = 4 - (display_col % 4);
                for _ in 0..tab_stop {
                    span_text.push(' ');
                    display_col += 1;
                }
            } else {
                span_text.push(ch);
                display_col += 1;
            }

            byte_col += ch.len_utf8();
        }

        flush_span(&mut spans, &mut span_text, span_style);
        lines.push(StyledLine::new(spans));
    }

    let selection = if in_visual {
        sel_start.zip(sel_end).map(|(start, end)| {
            Selection::range(
                start.row as u32,
                start.col as u32,
                end.row as u32,
                end.col as u32,
                SelectionMode::Char,
            )
        })
    } else {
        None
    };

    let mut node = display_protocol::TextAreaNode {
        lines,
        cursor_line: buf.cursor.row as u16,
        cursor_col: buf.cursor.col as u16,
        selection: None,
        scroll_top: 0,
        scroll_left: 0,
        height,
        style: Style::default().fg(text_fg).bg(t.background),
        cursor_style: Style::default().fg(cursor_fg).bg(cursor_bg),
        selection_style: Style::default().bg(sel_bg),
        gutter: show_line_numbers,
        focused: true,
    }
    .scroll(view.scroll_top as u16, 0)
    .cursor(buf.cursor.row as u16, buf.cursor.col as u16);
    if let Some(sel) = selection {
        node = node.selection(sel);
    } else {
        node = node.clear_selection();
    }
    UiNode::TextArea(node)
}
