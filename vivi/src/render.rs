use display_protocol::buffer::ScreenBuffer;
use display_protocol::{Color, Style, StyledLine, UiNode, TextNode, Wrap};
use display_protocol_jsx::jsx;

use crate::editor::Editor;
use crate::mode::Mode;

/// Render the editor state using the declarative jsx! UI tree.
///
/// The editor layout is a Column with three children:
///   1. TextArea  — line numbers + text content (fills remaining height)
///   2. StatusBar — mode indicator, file name, cursor position
///   3. Text      — command line input or status message
///
/// The jsx! macro builds the skeleton; runtime data (buffer lines,
/// cursor position, scroll offset) is set on the nodes after construction.
pub fn render(editor: &Editor, width: u16, height: u16) -> ScreenBuffer {
    let mut buf = ScreenBuffer::new(width, height);

    if width == 0 || height == 0 {
        return buf;
    }

    let text_area_height = (height as usize).saturating_sub(2) as u16;

    // ── Convert buffer lines to StyledLine ──────────────────────────
    let lines: Vec<StyledLine> = (0..editor.buffer().line_count())
        .map(|i| StyledLine::plain(editor.buffer().line(i)))
        .collect();

    // ── Mode-dependent status bar style ─────────────────────────────
    let mode_bg = match editor.mode() {
        Mode::Normal => Color::new(60, 60, 60),
        Mode::Insert => Color::new(0, 128, 0),
        Mode::Visual | Mode::VisualLine => Color::new(100, 100, 200),
        Mode::Command => Color::new(180, 140, 60),
        Mode::Replace => Color::new(180, 60, 60),
    };

    // ── Status bar content ──────────────────────────────────────────
    let modified_label = if editor.buffer().is_modified() { " [+] " } else { " " };
    let file_info = format!("{}{}", editor.buffer().display_name(), modified_label);
    let pos_str = format!(
        "{}:{}/{} ",
        editor.cursor().col + 1,
        editor.cursor().row + 1,
        editor.buffer().line_count()
    );

    // ── Build the editor layout skeleton with jsx! ──────────────────
    //
    // The <TextArea gutter /> sets up the structure. Children are
    // placeholders — we fill real data below.
    let mut tree = jsx! {
        <Column height={height}>
            <TextArea gutter />
            <StatusBar>
                <Left>
                    <Text bold>{editor.mode().indicator()}</Text>
                </Left>
                <Right>
                    <Text>{file_info.as_str()}</Text>
                    <Text dim>{pos_str.as_str()}</Text>
                </Right>
            </StatusBar>
            <Text></Text>
        </Column>
    };

    // ── Patch runtime data into the tree ────────────────────────────

    if let UiNode::Column(ref mut col) = tree {
        // ── TextArea (child 0): lines, cursor, scroll ───────────────
        if let UiNode::TextArea(ref mut ta) = col.children[0] {
            ta.lines = lines;
            ta.height = text_area_height;
            ta.focused = true;
            ta.gutter = true;
            ta.style = Style::new().fg(Color::new(204, 204, 204));
            ta.cursor_style = Style::new()
                .fg(Color::BLACK)
                .bg(Color::new(204, 204, 204));
            ta.cursor_line = editor.cursor().row as u16;
            ta.cursor_col = editor.cursor().col as u16;
            ta.scroll_top = editor.scroll().row as u16;
            ta.scroll_left = editor.scroll().col as u16;
            ta.selection = editor.selection().cloned();
        }

        // ── StatusBar (child 1): mode + file info ───────────────────
        if let UiNode::StatusBar(ref mut sb) = col.children[1] {
            sb.left = vec![
                UiNode::Text(TextNode {
                    content: format!(" {} ", editor.mode().indicator()),
                    style: Style::new().bg(mode_bg),
                    wrap: Wrap::default(),
                }),
            ];
            sb.right = vec![
                UiNode::Text(TextNode {
                    content: file_info,
                    style: Style::new().fg(Color::new(204, 204, 204)).bold(),
                    wrap: Wrap::default(),
                }),
                UiNode::Text(TextNode {
                    content: pos_str,
                    style: Style::new().fg(Color::new(204, 204, 204)).dim(),
                    wrap: Wrap::default(),
                }),
            ];
            sb.style = Style::new().bg(Color::new(60, 60, 60));
        }

        // ── Command line / message area (child 2) ───────────────────
        if let UiNode::Text(ref mut t) = col.children[2] {
            match editor.mode() {
                Mode::Command => {
                    t.content = format!(":{}", editor.command_input());
                    t.style = Style::new().fg(Color::new(204, 204, 204));
                }
                _ => {
                    if let Some(msg) = editor.message() {
                        t.content = msg.to_string();
                        t.style = Style::new().fg(Color::new(204, 204, 204));
                    }
                }
            }
        }
    }

    // ── Paint the declarative tree into the ScreenBuffer ────────────
    display_protocol::paint_into(&tree, &mut buf);

    buf
}

/// Calculate the width needed for line numbers.
pub fn line_number_width(line_count: usize, screen_width: usize) -> usize {
    if line_count == 0 {
        return 4;
    }
    let digits = format!("{}", line_count).len();
    (digits.max(3) + 1).min(screen_width / 3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Buffer;
    use crate::editor::Editor;

    #[test]
    fn test_render_empty_buffer() {
        let editor = Editor::new(Buffer::new());
        let buf = render(&editor, 80, 24);
        assert_eq!(buf.width, 80);
        assert_eq!(buf.height, 24);
        // Screen should have content somewhere
        assert!(buf.width > 0 && buf.height > 0);
    }

    #[test]
    fn test_render_with_content() {
        let mut buf = Buffer::new();
        buf.insert_str(0, 0, "hello world");
        buf.insert_line(1, "second line".into());
        let editor = Editor::new(buf);
        let screen = render(&editor, 80, 24);
        assert_eq!(screen.width, 80);
        assert_eq!(screen.height, 24);
    }

    #[test]
    fn test_render_command_mode() {
        let mut editor = Editor::new(Buffer::new());
        // Enter command mode
        use display_protocol::{KeyCode, KeyEvent, KeyModifiers};
        editor.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::EMPTY));
        editor.handle_key(KeyEvent::char('w'));
        let screen = render(&editor, 80, 24);
        assert_eq!(screen.width, 80);
        // Verify ':' appears somewhere in the bottom row
        let mut found_colon = false;
        for x in 0..80 {
            let cell = screen.get(x, 23);
            if cell.ch == ':' {
                found_colon = true;
                break;
            }
        }
        assert!(found_colon, "Expected ':' in command line area");
    }

    #[test]
    fn test_render_shrinking_terminal() {
        let mut buf = Buffer::new();
        buf.insert_str(0, 0, "long text that exceeds terminal width");
        let editor = Editor::new(buf);
        let screen = render(&editor, 20, 10);
        assert_eq!(screen.width, 20);
        assert_eq!(screen.height, 10);
    }
}