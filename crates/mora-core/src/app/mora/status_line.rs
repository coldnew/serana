use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::editor::MoraEditor;
use super::keymap::EditorMode;

pub fn render_status_line(editor: &MoraEditor, width: usize) -> Line<'static> {
    let mode = editor.mode();
    let (mode_label, mode_style) = match mode {
        EditorMode::Normal => (
            " NORMAL ",
            Style::new()
                .fg(Color::Rgb(15, 18, 22))
                .bg(Color::Rgb(0, 180, 255))
                .add_modifier(Modifier::BOLD),
        ),
        EditorMode::Insert => (
            " INSERT ",
            Style::new()
                .fg(Color::Rgb(15, 18, 22))
                .bg(Color::Rgb(0, 255, 136))
                .add_modifier(Modifier::BOLD),
        ),
        EditorMode::Command | EditorMode::SearchForward | EditorMode::SearchBackward => (
            " CMD ",
            Style::new()
                .fg(Color::Rgb(15, 18, 22))
                .bg(Color::Rgb(212, 192, 144))
                .add_modifier(Modifier::BOLD),
        ),
        EditorMode::Emacs => (
            " EMACS ",
            Style::new()
                .fg(Color::Rgb(15, 18, 22))
                .bg(Color::Rgb(255, 179, 71))
                .add_modifier(Modifier::BOLD),
        ),
        EditorMode::ReplaceChar => (
            " REPLACE ",
            Style::new()
                .fg(Color::Rgb(15, 18, 22))
                .bg(Color::Rgb(255, 71, 87))
                .add_modifier(Modifier::BOLD),
        ),
        EditorMode::Visual => (
            " VISUAL ",
            Style::new()
                .fg(Color::Rgb(15, 18, 22))
                .bg(Color::Rgb(180, 130, 20))
                .add_modifier(Modifier::BOLD),
        ),
    };

    let buf = editor.buffer();
    let filename = buf.filename();
    let modified = if buf.modified { " [+]" } else { "" };
    let pos = format!(" {}:{} ", buf.cursor.row + 1, buf.cursor.col + 1);
    let total = format!(" /{} ", buf.line_count());

    let dim = Style::new().fg(Color::Rgb(107, 114, 128));
    let bright = Style::new()
        .fg(Color::Rgb(232, 236, 244))
        .add_modifier(Modifier::BOLD);
    let modified_style = Style::new()
        .fg(Color::Rgb(255, 179, 71))
        .add_modifier(Modifier::BOLD);

    let mut spans = vec![
        Span::styled(mode_label, mode_style),
        Span::styled(" ", dim),
        Span::styled(filename.to_string(), bright),
        Span::styled(modified.to_string(), modified_style),
    ];

    let macro_indicator = if editor.macro_state.is_recording() {
        " [*REC*] "
    } else if editor.macro_state.is_playing() {
        " [PLAY] "
    } else {
        ""
    };
    if !macro_indicator.is_empty() {
        spans.push(Span::styled(
            macro_indicator,
            Style::new()
                .fg(Color::Rgb(255, 71, 87))
                .add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK),
        ));
    }

    let mark_indicator = if editor.mark_ring.is_active() {
        " [MARK] "
    } else {
        ""
    };
    if !mark_indicator.is_empty() {
        spans.push(Span::styled(
            mark_indicator,
            Style::new()
                .fg(Color::Rgb(0, 255, 136))
                .add_modifier(Modifier::BOLD),
        ));
    }

    let used: usize = spans.iter().map(|s| s.width()).sum();
    let right = format!("{}{}", pos, total);
    let fill = width.saturating_sub(used + right.len());
    spans.push(Span::styled(" ".repeat(fill), dim));
    spans.push(Span::styled(right, dim));

    Line::from(spans)
}

pub fn render_command_line(editor: &MoraEditor, width: usize) -> Line<'static> {
    let prompt = match editor.mode() {
        EditorMode::Command => ":",
        EditorMode::SearchForward => "/",
        EditorMode::SearchBackward => "?",
        _ => "",
    };
    let input = editor.command_input();

    let style = Style::new().fg(Color::Rgb(232, 236, 244));
    let prompt_style = Style::new()
        .fg(Color::Rgb(0, 180, 255))
        .add_modifier(Modifier::BOLD);

    let content = format!("{}{}", prompt, input);
    let padded = format!("{:width$}", content, width = width);

    Line::from(vec![
        Span::styled(format!("{}", prompt), prompt_style),
        Span::styled(padded[prompt.len()..].to_string(), style),
    ])
}

pub fn render_help_bar(mode: EditorMode, width: usize) -> Line<'static> {
    let hints = match mode {
        EditorMode::Normal => {
            "i:Insert  /:Search  ::Cmd  v:Visual  Ctrl-S:Save  u:Undo  Ctrl-E:Emacs"
        }
        EditorMode::Insert => "Esc:Normal  Ctrl-S:Save  Arrows:Move",
        EditorMode::Command => "Enter:Exec  Esc:Cancel",
        EditorMode::SearchForward | EditorMode::SearchBackward => {
            "Enter:Search  Esc:Cancel  n:Next  N:Prev"
        }
        EditorMode::Emacs => {
            "C-g:Normal  C-x C-s:Save  C-x C-c:Quit  C-SPC:Mark  C-w:Kill  M-w:Copy  C-y:Yank  C-t:Transp  M-c:Cap  M-u:Up  M-l:Low  M-/:Complete  M-z:Zap  C-o:Line"
        }
        EditorMode::ReplaceChar => "Press char to replace with  Esc:Cancel",
        EditorMode::Visual => "hjkl/arrows:Move  w/b/e:Word  d/x:Kill  y:Copy  o:Swap  I/A:Insert  Esc/C-g:Exit",
    };

    let style = Style::new().fg(Color::Rgb(107, 114, 128));
    let padded = format!("{:width$}", hints, width = width);
    Line::from(Span::styled(padded, style))
}
