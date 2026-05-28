use super::display::style::{MoraColor, MoraStyle, StyledLine, StyledSpan};
use super::editor::MoraEditor;
use super::keymap::EditorMode;

pub fn render_status_line(editor: &MoraEditor, width: usize) -> StyledLine {
    let mode = editor.mode();
    let (mode_label, mode_style) = match mode {
        EditorMode::Normal => (
            " NORMAL ",
            MoraStyle::new()
                .fg(MoraColor::new(15, 18, 22))
                .bg(MoraColor::new(0, 180, 255))
                .bold(),
        ),
        EditorMode::Insert => (
            " INSERT ",
            MoraStyle::new()
                .fg(MoraColor::new(15, 18, 22))
                .bg(MoraColor::new(0, 255, 136))
                .bold(),
        ),
        EditorMode::Command | EditorMode::SearchForward | EditorMode::SearchBackward => (
            " CMD ",
            MoraStyle::new()
                .fg(MoraColor::new(15, 18, 22))
                .bg(MoraColor::new(212, 192, 144))
                .bold(),
        ),
        EditorMode::Emacs => (
            " EMACS ",
            MoraStyle::new()
                .fg(MoraColor::new(15, 18, 22))
                .bg(MoraColor::new(255, 179, 71))
                .bold(),
        ),
        EditorMode::ReplaceChar => (
            " REPLACE ",
            MoraStyle::new()
                .fg(MoraColor::new(15, 18, 22))
                .bg(MoraColor::new(255, 71, 87))
                .bold(),
        ),
        EditorMode::Visual => (
            " VISUAL ",
            MoraStyle::new()
                .fg(MoraColor::new(15, 18, 22))
                .bg(MoraColor::new(180, 130, 20))
                .bold(),
        ),
        EditorMode::Iedit => (
            " IEDIT ",
            MoraStyle::new()
                .fg(MoraColor::new(15, 18, 22))
                .bg(MoraColor::new(200, 80, 200))
                .bold(),
        ),
    };

    let buf = editor.buffer();
    let filename = buf.filename();
    let modified = if buf.modified { " [+]" } else { "" };
    let mode_name = format!(" {} ", buf.major_mode.name());
    let pos = format!(" {}:{} ", buf.cursor.row + 1, buf.cursor.col + 1);
    let total = format!(" /{} ", buf.line_count());

    let dim = MoraStyle::new().fg(MoraColor::new(107, 114, 128));
    let bright = MoraStyle::new()
        .fg(MoraColor::new(232, 236, 244))
        .bold();
    let modified_style = MoraStyle::new()
        .fg(MoraColor::new(255, 179, 71))
        .bold();
    let mode_name_style = MoraStyle::new().fg(MoraColor::new(140, 160, 200));

    let mut spans = vec![
        StyledSpan::new(mode_label, mode_style),
        StyledSpan::new(" ", dim),
        StyledSpan::new(filename.to_string(), bright),
        StyledSpan::new(modified.to_string(), modified_style),
        StyledSpan::new(mode_name, mode_name_style),
    ];

    let macro_indicator = if editor.macro_state.is_recording() {
        " [*REC*] "
    } else if editor.macro_state.is_playing() {
        " [PLAY] "
    } else {
        ""
    };
    if !macro_indicator.is_empty() {
        spans.push(StyledSpan::new(
            macro_indicator,
            MoraStyle::new()
                .fg(MoraColor::new(255, 71, 87))
                .bold()
                .blink(),
        ));
    }

    let mark_indicator = if editor.mark_ring.is_active() {
        " [MARK] "
    } else {
        ""
    };
    if !mark_indicator.is_empty() {
        spans.push(StyledSpan::new(
            mark_indicator,
            MoraStyle::new()
                .fg(MoraColor::new(0, 255, 136))
                .bold(),
        ));
    }

    let minor_indicator = editor.minor_modes.modeline_string();
    if !minor_indicator.is_empty() {
        spans.push(StyledSpan::new(
            minor_indicator,
            MoraStyle::new()
                .fg(MoraColor::new(180, 130, 255))
                .bold(),
        ));
    }

    let used: usize = spans.iter().map(|s| s.width()).sum();
    let right = format!("{}{}", pos, total);
    let fill = width.saturating_sub(used + right.len());
    spans.push(StyledSpan::new(" ".repeat(fill), dim));
    spans.push(StyledSpan::new(right, dim));

    StyledLine::new(spans)
}

pub fn render_command_line(editor: &MoraEditor, width: usize) -> StyledLine {
    let prompt = editor.minibuffer_prompt();
    let input = editor.command_input();

    let style = MoraStyle::new().fg(MoraColor::new(232, 236, 244));
    let prompt_style = MoraStyle::new()
        .fg(MoraColor::new(0, 180, 255))
        .bold();

    let content = format!("{}{}", prompt, input);
    let padded = format!("{:width$}", content, width = width);

    StyledLine::new(vec![
        StyledSpan::new(format!("{}", prompt), prompt_style),
        StyledSpan::new(padded[prompt.len()..].to_string(), style),
    ])
}

pub fn render_help_bar(mode: EditorMode, width: usize) -> StyledLine {
    let hints = match mode {
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

    let style = MoraStyle::new().fg(MoraColor::new(107, 114, 128));
    let padded = format!("{:width$}", hints, width = width);
    StyledLine::new(vec![StyledSpan::new(padded, style)])
}
