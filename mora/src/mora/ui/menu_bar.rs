use crate::mora::editor::MoraEditor;
use display_protocol::UiNode;
use display_protocol_jsx::jsx;

use super::pad_to_width;

/// Emacs-style menu bar. It is intentionally declarative and compact so Lisp
/// or future UI extensions can replace/augment the same top-level slot.
pub(super) fn build_menu_bar(editor: &MoraEditor, width: u16) -> UiNode {
    let t = &editor.theme;
    let fg = t.modeline_fg;
    let bg = t.modeline_bg;
    let menus = " File  Edit  Options  Buffers  Tools  Help ";
    let content = pad_to_width(menus, width as usize);

    jsx! {
        <Row width={width}>
            <Text color={fg} bg={bg}>{content}</Text>
        </Row>
    }
}
