use super::editor::MoraEditor;
use display_protocol::UiNode;

pub fn build_ui(editor: &MoraEditor, width: u16, height: u16) -> UiNode {
    super::ui::build_ui(editor, width, height)
}
