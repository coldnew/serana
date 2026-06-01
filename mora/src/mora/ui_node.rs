use super::editor::MoraEditor;
use display_protocol::UiNode;

pub fn build_ui(editor: &mut MoraEditor, width: u16, height: u16, show_menu_bar: bool) -> UiNode {
    super::ui::build_ui(editor, width, height, show_menu_bar)
}
