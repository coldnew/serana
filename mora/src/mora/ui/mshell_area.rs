use crate::mora::editor::MoraEditor;
use display_protocol::UiNode;
use display_protocol_jsx::jsx;

use super::{jsx_column, jsx_row};

/// Build mshell output area — shows shell output history.
pub(super) fn build_mshell_area(editor: &MoraEditor, width: u16, height: u16) -> UiNode {
    let output = &editor.mshell.output_lines;
    let t = &editor.theme;
    let output_fg = t.foreground;
    let prompt_fg = t.echo_prompt;

    let start = if output.len() > height as usize {
        output.len() - height as usize
    } else {
        0
    };

    let mut rows = Vec::new();
    for line in &output[start..] {
        let prompt = editor.mshell.prompt();
        if line.starts_with(&prompt) {
            let (p, rest) = line.split_at(prompt.len());
            rows.push(jsx_row(
                vec![
                    UiNode::text(p.to_string()).color(prompt_fg).bold(),
                    UiNode::text(rest.to_string()).color(output_fg),
                ],
                width,
            ));
        } else {
            rows.push(jsx_row(
                vec![UiNode::text(line.clone()).color(output_fg)],
                width,
            ));
        }
    }

    while rows.len() < height as usize {
        rows.push(jsx_row(vec![UiNode::text(" ")], width));
    }

    jsx_column(rows, width, height)
}

/// Build mshell echo area — shows shell prompt + input with cursor.
pub(super) fn build_mshell_echo(editor: &MoraEditor, width: u16) -> UiNode {
    let prompt = editor.mshell.prompt();
    let input = &editor.mshell.input;
    let cursor_pos = editor.mshell.cursor;
    let t = &editor.theme;
    let prompt_fg = t.echo_prompt;
    let input_fg = t.echo_input;
    let cursor_bg = t.cursor;
    let cursor_fg = t.background;

    let before_cursor = &input[..cursor_pos.min(input.len())];
    let cursor_char = if cursor_pos < input.len() {
        input[cursor_pos..].chars().next().unwrap_or(' ')
    } else {
        ' '
    };
    let after_cursor = if cursor_pos < input.len() {
        let next_char_end = cursor_pos + cursor_char.len_utf8();
        &input[next_char_end..]
    } else {
        ""
    };

    jsx! {
        <Row width={width}>
            <Text color={prompt_fg} bold>{prompt}</Text>
            <Text color={input_fg}>{before_cursor.to_string()}</Text>
            <Text color={cursor_fg} bg={cursor_bg}>{cursor_char.to_string()}</Text>
            <Text color={input_fg}>{after_cursor.to_string()}</Text>
        </Row>
    }
}
