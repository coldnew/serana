use display_protocol::{UiNode, Color, Style};
use super::editor::MoraEditor;
use super::keymap::EditorMode;
use super::status_line;

/// Build a UiNode tree from the current editor state.
/// This is the declarative UI equivalent of EditorWidget::render().
pub fn build_ui(editor: &MoraEditor, width: u16, height: u16) -> UiNode {
    let help_height = 1u16;
    let status_height = 1u16;
    let cmd_height: u16 = if matches!(
        editor.mode(),
        EditorMode::Command | EditorMode::SearchForward | EditorMode::SearchBackward
    ) {
        1
    } else {
        0
    };
    let editor_height = height.saturating_sub(help_height + status_height + cmd_height);

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

    // Macro indicator
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

    // Mark indicator
    if editor.mark_ring.is_active() {
        children.push(
            UiNode::text(" [MARK] ")
                .color(Color::new(0, 255, 136))
                .bold(),
        );
    }

    // Minor modes
    let minor_indicator = editor.minor_modes.modeline_string();
    if !minor_indicator.is_empty() {
        children.push(
            UiNode::text(minor_indicator)
                .color(Color::new(180, 130, 255))
                .bold(),
        );
    }

    // Fill + position
    let right = format!("{}{}", pos, total);
    children.push(UiNode::text(right).color(dim));

    UiNode::row(children).width(width)
}

fn build_command_line(editor: &MoraEditor, width: u16) -> UiNode {
    let prompt = match editor.mode() {
        EditorMode::Command => ":",
        EditorMode::SearchForward => "/",
        EditorMode::SearchBackward => "?",
        _ => "",
    };
    let input = editor.command_input();

    UiNode::row(vec![
        UiNode::text(prompt).color(Color::new(0, 180, 255)).bold(),
        UiNode::text(input.to_string()).color(Color::new(232, 236, 244)),
    ]).width(width)
}

fn build_editor_area(editor: &MoraEditor, width: u16, height: u16) -> UiNode {
    let buf = editor.buffer();
    let view = editor.current_view();
    let syntax_tokens = buf.highlight_tokens();

    let mut rows = Vec::new();
    let gutter_width = format!("{}", buf.line_count().max(999)).len() as u16 + 1;

    for i in 0..height as usize {
        let line_idx = view.scroll_top + i;
        if line_idx < buf.line_count() {
            let line = buf.line(line_idx);
            let line_num = format!("{:>width$}", line_idx + 1, width = gutter_width as usize - 1);

            let gutter = UiNode::text(format!("{} ", line_num))
                .color(Color::new(70, 75, 85));

            // Build styled line content with syntax highlighting
            let mut spans = vec![gutter];
            for ch in line.chars() {
                spans.push(UiNode::text(ch.to_string()).color(Color::new(232, 236, 244)));
            }

            rows.push(UiNode::row(spans));
        } else {
            // Empty line marker
            rows.push(
                UiNode::row(vec![
                    UiNode::text(" ").color(Color::new(70, 75, 85)),
                    UiNode::text("~").color(Color::new(50, 55, 65)),
                ])
            );
        }
    }

    UiNode::column(rows).width(width)
}
