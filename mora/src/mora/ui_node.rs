use display_protocol::{UiNode, Color, Style};
use super::editor::MoraEditor;
use super::keymap::EditorMode;
use super::syntax;
use super::lisp_ext;

/// Build a UiNode tree from the current editor state.
/// If Lisp UI builders are registered, they augment/replace the default UI.
pub fn build_ui(editor: &MoraEditor, width: u16, height: u16) -> UiNode {
    let help_height = 1u16;
    let status_height = 1u16;
    let cmd_height: u16 = if editor.minibuffer_active() {
        1
    } else {
        0
    };
    let editor_height = height.saturating_sub(help_height + status_height + cmd_height);

    // Try Lisp UI builders first
    if let Some(lisp_ui) = lisp_ext::build_lisp_ui(width, height) {
        return lisp_ui;
    }

    let mut children = vec![
        build_help_bar(editor, width),
        build_editor_area(editor, width, editor_height),
        build_status_line(editor, width),
    ];

    if cmd_height > 0 {
        children.push(build_command_line(editor, width));
    }

    UiNode::column(children)
}

fn build_help_bar(editor: &MoraEditor, width: u16) -> UiNode {
    let hints = match editor.mode() {
        EditorMode::Normal => {
            "i:Insert  f/F/t/T:Find  ;/,:Repeat  *:Search  ~:Case  S:Sub  .:Repeat  %:Match  /:Search  ::Cmd  v:Visual  u:Undo  Ctrl-E:Emacs"
        }
        EditorMode::Insert => "Esc:Normal  Ctrl-S:Save  Arrows:Move",
        EditorMode::Command => "Enter:Exec  Tab:Complete  Esc:Cancel",
        EditorMode::SearchForward | EditorMode::SearchBackward => {
            "Enter:Search  Esc:Cancel  n:Next  N:Prev"
        }
        EditorMode::Emacs => {
            "C-g:Normal  M-x:Commands  C-SPC:Mark  C-w:Kill  M-w:Copy  C-y:Yank  C-t:Transp  M-c:Cap  M-u:Up  M-l:Low  M-/:Complete  M-z:Zap  C-o:Line  C-;:Iedit"
        }
        EditorMode::ReplaceChar => "Press char to replace with  Esc:Cancel",
        EditorMode::Visual => "hjkl/arrows:Move  w/b/e:Word  d/x:Kill  y:Copy  o:Swap  I/A:Insert  Esc/C-g:Exit",
        EditorMode::Iedit => "Type:Edit all  Tab/Shift-Tab:Cycle  C-n/C-p:Nav  Backspace/Del:Delete  Esc/C-g:Exit",
    };

    UiNode::text(hints)
        .color(Color::new(107, 114, 128))
        .width(width)
}

fn build_status_line(editor: &MoraEditor, width: u16) -> UiNode {
    let mode = editor.mode();
    let (mode_label, mode_fg, mode_bg) = match mode {
        EditorMode::Normal => (" NORMAL ", Color::new(15, 18, 22), Color::new(0, 180, 255)),
        EditorMode::Insert => (" INSERT ", Color::new(15, 18, 22), Color::new(0, 255, 136)),
        EditorMode::Command | EditorMode::SearchForward | EditorMode::SearchBackward => {
            (" CMD ", Color::new(15, 18, 22), Color::new(212, 192, 144))
        }
        EditorMode::Emacs => (" EMACS ", Color::new(15, 18, 22), Color::new(255, 179, 71)),
        EditorMode::ReplaceChar => (" REPLACE ", Color::new(15, 18, 22), Color::new(255, 71, 87)),
        EditorMode::Visual => (" VISUAL ", Color::new(15, 18, 22), Color::new(180, 130, 20)),
        EditorMode::Iedit => (" IEDIT ", Color::new(15, 18, 22), Color::new(200, 80, 200)),
    };

    let buf = editor.buffer();
    let filename = buf.filename();
    let modified = if buf.modified { " [+]" } else { "" };
    let mode_name = format!(" {} ", buf.major_mode.name());
    let pos = format!(" {}:{} ", buf.cursor.row + 1, buf.cursor.col + 1);
    let total = format!(" /{} ", buf.line_count());

    let dim = Color::new(107, 114, 128);
    let bright = Color::new(232, 236, 244);
    let modified_color = Color::new(255, 179, 71);
    let mode_name_color = Color::new(140, 160, 200);

    let mut children = vec![
        UiNode::text(mode_label).color(mode_fg).bg(mode_bg).bold(),
        UiNode::text(" ").color(dim),
        UiNode::text(filename.to_string()).color(bright).bold(),
        UiNode::text(modified.to_string()).color(modified_color).bold(),
        UiNode::text(mode_name).color(mode_name_color),
    ];

    if editor.macro_state.is_recording() {
        children.push(
            UiNode::text(" [*REC*] ")
                .color(Color::new(255, 71, 87))
                .bold(),
        );
    } else if editor.macro_state.is_playing() {
        children.push(
            UiNode::text(" [PLAY] ")
                .color(Color::new(255, 71, 87))
                .bold(),
        );
    }

    if editor.mark_ring.is_active() {
        children.push(
            UiNode::text(" [MARK] ")
                .color(Color::new(0, 255, 136))
                .bold(),
        );
    }

    let minor_indicator = editor.minor_modes.modeline_string();
    if !minor_indicator.is_empty() {
        children.push(
            UiNode::text(minor_indicator)
                .color(Color::new(180, 130, 255))
                .bold(),
        );
    }

    let right = format!("{}{}", pos, total);
    children.push(UiNode::text(right).color(dim));

    UiNode::row(children).width(width)
}

fn build_command_line(editor: &MoraEditor, width: u16) -> UiNode {
    let prompt = editor.minibuffer_prompt();
    let input = editor.command_input();

    UiNode::row(vec![
        UiNode::text(prompt).color(Color::new(0, 180, 255)).bold(),
        UiNode::text(input.to_string()).color(Color::new(232, 236, 244)),
    ]).width(width)
}

fn style_to_color(s: super::display::style::MoraStyle) -> (Option<Color>, Option<Color>, bool, bool, bool, bool, bool, bool) {
    let fg = s.fg.map(|c| Color::new(c.r, c.g, c.b));
    let bg = s.bg.map(|c| Color::new(c.r, c.g, c.b));
    (fg, bg, s.bold, s.italic, s.underline, s.strikethrough, s.dim, s.reverse)
}

fn syntax_style_for_col(tokens: &[syntax::HighlightToken], col: usize) -> Option<super::display::style::MoraStyle> {
    for tok in tokens {
        if col >= tok.start && col < tok.end {
            return Some(syntax::style_for_kind(tok.kind));
        }
    }
    None
}

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
            let (a, b) = if mark.row < cursor.row || (mark.row == cursor.row && mark.col <= cursor.col) {
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

        // Gutter
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

        // Text content with syntax highlighting
        let line_text = buf.line(line_idx);
        let highlight_tokens = highlighter.highlight_line(line_text, line_idx);
        let line_byte_offset = buf.byte_offset(line_idx, 0);

        let mut byte_col = 0;
        let mut display_col: u16 = 0;
        let mut span_start = 0;
        let mut span_text = String::new();
        let mut span_fg: Option<Color> = None;
        let mut span_bg: Option<Color> = None;
        let mut span_bold = false;

        let flush_span = |spans: &mut Vec<UiNode>, text: &mut String, fg: Option<Color>, bg: Option<Color>, bold: bool| {
            if !text.is_empty() {
                let mut node = UiNode::text(std::mem::take(text));
                if let Some(c) = fg { node = node.color(c); }
                if let Some(c) = bg { node = node.bg(c); }
                if bold { node = node.bold(); }
                spans.push(node);
            }
        };

        for ch in line_text.chars() {
            if display_col >= text_width {
                break;
            }

            let is_cursor = is_current && byte_col == buf.cursor.col && editor.mode() != EditorMode::Normal;
            let in_sel = sel_start.map_or(false, |start| {
                sel_end.map_or(false, |end| {
                    line_idx > start.row && line_idx < end.row
                        || (line_idx == start.row && line_idx == end.row && byte_col >= start.col && byte_col < end.col)
                        || (line_idx == start.row && byte_col >= start.col)
                        || (line_idx == end.row && byte_col < end.col)
                })
            });

            let is_invisible = buf.overlays.is_invisible(line_byte_offset + byte_col);
            if is_invisible {
                byte_col += ch.len_utf8();
                continue;
            }

            // Determine style for this character
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

            // Check if style changed — flush current span
            if ch_fg != span_fg || ch_bg != span_bg || ch_bold != span_bold {
                flush_span(&mut spans, &mut span_text, span_fg, span_bg, span_bold);
                span_fg = ch_fg;
                span_bg = ch_bg;
                span_bold = ch_bold;
            }

            // Handle tab expansion
            if ch == '\t' {
                let tab_stop = 4 - (display_col % 4);
                for _ in 0..tab_stop {
                    if display_col >= text_width { break; }
                    span_text.push(' ');
                    display_col += 1;
                }
            } else {
                span_text.push(ch);
                display_col += 1;
            }

            byte_col += ch.len_utf8();
        }

        // Flush remaining span
        flush_span(&mut spans, &mut span_text, span_fg, span_bg, span_bold);

        // Fill rest of line
        if is_current {
            while display_col < text_width {
                spans.push(UiNode::text(" ").bg(Color::new(25, 28, 35)));
                display_col += 1;
            }
        }

        rows.push(UiNode::row(spans));
        display_row += 1;
    }

    // Empty lines with tilde
    for _ in display_row..height {
        rows.push(
            UiNode::row(vec![
                UiNode::text("~").color(tilde_color),
            ])
        );
    }

    UiNode::column(rows).width(width)
}
