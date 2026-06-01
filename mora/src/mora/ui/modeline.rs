use crate::mora::editor::MoraEditor;
use crate::mora::keymap::EditorMode;
use display_protocol::UiNode;
use display_protocol_jsx::jsx;

/// Emacs-style modeline matching the real Emacs mode-line format.
///
/// Format: --:**-  *scratch*  (Lisp Interaction)  --L1--C1----  All
///
/// Components:
///   - Modified flag: `**` (modified) or `--` (unmodified)
///   - Dash separator
///   - Buffer name (with `*...*` for special buffers)
///   - Major mode (with submode if active)
///   - Position with dashes: `--L<line>--C<col>----`
///   - Percentage or `Top`/`Bot`/`All`
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

    let left = format!("{}-  {}  ", modified_flag, buf_name);
    let mode_info = format!("({}{})", major_mode, submode);
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
