use crate::mora::editor::MoraEditor;
use crate::mora::keymap::EditorMode;
use display_protocol::UiNode;
use display_protocol_jsx::jsx;

/// Emacs-style modeline for the default frame.
///
/// The line keeps the same shape as the stock Emacs frame:
/// modified flag, buffer name, mode, minor-mode indicators, and position.
pub(super) fn build_modeline(editor: &MoraEditor, width: u16) -> UiNode {
    let buf = editor.buffer();
    let filename = buf.filename();
    let modified_flag = if buf.modified { "**" } else { "--" };

    let line = buf.cursor.row + 1;
    let total_lines = buf.line_count().max(1);
    let pct = if total_lines <= 1 {
        100
    } else {
        (line * 100) / total_lines
    };

    let buf_name = if filename.is_empty() {
        "*scratch*"
    } else {
        filename
    };

    let major_mode = buf.major_mode.name();
    let minor_modes = editor.minor_modes.modeline_string();
    let submode = match editor.mode() {
        EditorMode::Normal => " Evil",
        EditorMode::Insert => "",
        EditorMode::Emacs => "",
        EditorMode::Command | EditorMode::SearchForward | EditorMode::SearchBackward => " Minibuf",
        EditorMode::ReplaceChar => " Replace",
        EditorMode::Visual => " Select",
        EditorMode::Iedit => " Iedit",
    };

    let pos_pct = if total_lines <= 1 || pct >= 100 {
        "All".to_string()
    } else if pct == 0 {
        "Top".to_string()
    } else if pct >= 99 {
        "Bot".to_string()
    } else {
        format!("{:>3}%", pct)
    };

    let left = format!("{}  {}  ", modified_flag, buf_name);
    let mode_info = if minor_modes.is_empty() {
        format!("({}{})", major_mode, submode)
    } else {
        format!("({}{}{})", major_mode, minor_modes, submode)
    };
    let right = format!(" {}", pos_pct);

    let left_len = left.len();
    let mode_len = mode_info.len();
    let right_len = right.len();
    let total_used = left_len + mode_len + right_len;
    let fill = (width as usize).saturating_sub(total_used);
    let padding = " ".repeat(fill);

    let modeline_str = format!("{}{}{}{}", left, mode_info, padding, right);
    let fg = editor.theme.modeline_fg;
    let bg = editor.theme.modeline_bg;

    jsx! {
        <Text color={fg} bg={bg} bold>{modeline_str}</Text>
    }
}
