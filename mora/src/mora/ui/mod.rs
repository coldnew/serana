use crate::mora::editor::MoraEditor;
use crate::mora::lisp_ext;
use display_protocol::UiNode;
use display_protocol_jsx::jsx;

mod echo_area;
mod editor_area;
mod menu_bar;
mod modeline;
mod mshell_area;

/// Build a UiNode tree from the current editor state with an Emacs-like UI.
///
/// Layout (top to bottom):
///   - Menu bar
///   - Editor or shell area
///   - Modeline
///   - Echo area / minibuffer
///
/// If Lisp UI builders are registered, they augment/replace the default UI.
pub fn build_ui(editor: &MoraEditor, width: u16, height: u16) -> UiNode {
    if let Some(lisp_ui) = lisp_ext::build_lisp_ui(width, height) {
        return lisp_ui;
    }

    let menu_height = 1u16;
    let modeline_height = 1u16;
    let echo_height = 1u16;
    let editor_height = height.saturating_sub(menu_height + modeline_height + echo_height);
    let menu = menu_bar::build_menu_bar(editor, width);

    if editor.mshell.is_active() {
        let shell_area = mshell_area::build_mshell_area(editor, width, editor_height);
        let modeline = modeline::build_modeline(editor, width);
        let echo = mshell_area::build_mshell_echo(editor, width);

        jsx! {
            <Column>
                {menu}
                {shell_area}
                {modeline}
                {echo}
            </Column>
        }
    } else {
        let editor_area = editor_area::build_editor_area(editor, width, editor_height);
        let modeline = modeline::build_modeline(editor, width);
        let echo = echo_area::build_echo_area(editor, width);

        jsx! {
            <Column>
                {menu}
                {editor_area}
                {modeline}
                {echo}
            </Column>
        }
    }
}

pub(super) fn jsx_column(children: Vec<UiNode>, width: u16, height: u16) -> UiNode {
    jsx! {
        <Column width={width} height={height}>
            <For children={children} />
        </Column>
    }
}

pub(super) fn jsx_row(children: Vec<UiNode>, width: u16) -> UiNode {
    jsx! {
        <Row width={width}>
            <For children={children} />
        </Row>
    }
}

pub(super) fn pad_to_width(input: &str, width: usize) -> String {
    let mut output: String = input.chars().take(width).collect();
    let used = output.chars().count();
    if used < width {
        output.push_str(&" ".repeat(width - used));
    }
    output
}
