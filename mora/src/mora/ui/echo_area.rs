use crate::mora::editor::MoraEditor;
use display_protocol::UiNode;
use display_protocol_jsx::jsx;

/// Echo area: shows minibuffer prompt+input when active,
/// or the status_message (like Emacs echo area).
pub(super) fn build_echo_area(editor: &MoraEditor, width: u16) -> UiNode {
    let t = &editor.theme;
    let prompt_fg = t.echo_prompt;
    let input_fg = t.echo_input;
    let msg_fg = t.echo_msg;

    if editor.minibuffer_active() {
        let prompt = editor.minibuffer_prompt();
        let input = editor.command_input();
        jsx! {
            <Row width={width}>
                <Text color={prompt_fg} bold>{prompt}</Text>
                <Text color={input_fg}>{input.to_string()}</Text>
            </Row>
        }
    } else {
        let msg = editor.status_message();
        if msg.is_empty() {
            jsx! { <Row width={width}><Text>""</Text></Row> }
        } else {
            jsx! { <Row width={width}><Text color={msg_fg}>{msg.to_string()}</Text></Row> }
        }
    }
}
