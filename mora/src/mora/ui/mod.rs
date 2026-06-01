use crate::mora::editor::MoraEditor;
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
///   - Menu bar, when enabled
///   - Editor or shell area
///   - Modeline
///   - Echo area / minibuffer
///
/// If Lisp UI builders are registered, they augment/replace the default UI.
pub fn build_ui(editor: &mut MoraEditor, width: u16, height: u16, show_menu_bar: bool) -> UiNode {
    if let Some(lisp_ui) = editor.lisp_bridge.build_ui(width, height) {
        return lisp_ui;
    }

    if let Some(lisp_ui) = editor
        .lisp_bridge
        .build_ui_component("frame", width, height)
    {
        return lisp_ui;
    }

    let menu_height = if show_menu_bar { 1u16 } else { 0u16 };
    let modeline_height = 1u16;
    let echo_height = 1u16;
    let editor_height = height.saturating_sub(menu_height + modeline_height + echo_height);

    if editor.mshell.is_active() {
        let mut children = Vec::new();
        if show_menu_bar {
            let menu_bar = editor
                .lisp_bridge
                .build_ui_component("menu-bar", width, menu_height)
                .unwrap_or_else(|| menu_bar::build_menu_bar(editor, width));
            children.push(menu_bar);
        }
        let shell_area = editor
            .lisp_bridge
            .build_ui_component("shell-area", width, editor_height)
            .unwrap_or_else(|| mshell_area::build_mshell_area(editor, width, editor_height));
        let modeline = editor
            .lisp_bridge
            .build_ui_component("modeline", width, modeline_height)
            .unwrap_or_else(|| modeline::build_modeline(editor, width));
        let echo = editor
            .lisp_bridge
            .build_ui_component("shell-echo-area", width, echo_height)
            .or_else(|| {
                editor
                    .lisp_bridge
                    .build_ui_component("echo-area", width, echo_height)
            })
            .unwrap_or_else(|| mshell_area::build_mshell_echo(editor, width));
        children.push(shell_area);
        children.push(modeline);
        children.push(echo);
        jsx_column(children, width, height)
    } else {
        let mut children = Vec::new();
        if show_menu_bar {
            let menu_bar = editor
                .lisp_bridge
                .build_ui_component("menu-bar", width, menu_height)
                .unwrap_or_else(|| menu_bar::build_menu_bar(editor, width));
            children.push(menu_bar);
        }
        let editor_area = editor
            .lisp_bridge
            .build_ui_component("editor-area", width, editor_height)
            .unwrap_or_else(|| editor_area::build_editor_area(editor, width, editor_height));
        let modeline = editor
            .lisp_bridge
            .build_ui_component("modeline", width, modeline_height)
            .unwrap_or_else(|| modeline::build_modeline(editor, width));
        let echo = editor
            .lisp_bridge
            .build_ui_component("echo-area", width, echo_height)
            .unwrap_or_else(|| echo_area::build_echo_area(editor, width));
        children.push(editor_area);
        children.push(modeline);
        children.push(echo);
        jsx_column(children, width, height)
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
