use display_protocol::buffer::ScreenBuffer;
use display_protocol::{Color, Style, StyledLine, UiNode};
use display_protocol_jsx::jsx;

use crate::editor::Editor;
use crate::mode::Mode;

const BG: Color = Color::BLACK;
const FG: Color = Color::new(218, 218, 218);
const DIM: Color = Color::new(102, 102, 102);
const STATUS_FG: Color = Color::BLACK;
const STATUS_BG: Color = Color::new(218, 218, 218);
const VISUAL_BG: Color = Color::new(70, 80, 110);
const CURSOR_BG: Color = Color::new(204, 204, 204);
const REPLACE_CURSOR: Color = Color::new(255, 100, 100);

pub fn render(editor: &Editor, width: u16, height: u16) -> ScreenBuffer {
    let mut buf = ScreenBuffer::new(width, height);

    if width == 0 || height == 0 {
        return buf;
    }

    let tree = build_ui(editor, width, height);
    display_protocol::paint_into(&tree, &mut buf);
    paint_filler_lines(&mut buf, editor, text_area_height(height));

    buf
}

pub fn draw_mode_cursor(screen: &mut ScreenBuffer, editor: &Editor, visible: bool) {
    if !visible {
        return;
    }

    let (cx, cy) = cursor_position(editor, screen.width, screen.height);
    if cx >= screen.width || cy >= screen.height {
        return;
    }

    let mut cell = screen.get(cx, cy);
    match editor.mode() {
        Mode::Normal | Mode::Visual | Mode::VisualLine | Mode::Command => {
            cell.fg = Color::BLACK;
            cell.bg = CURSOR_BG;
        }
        Mode::Insert => {
            cell.underline = true;
            cell.underline_color = Some(CURSOR_BG);
        }
        Mode::Replace => {
            cell.underline = true;
            cell.underline_color = Some(REPLACE_CURSOR);
        }
    }
    screen.set(cx, cy, cell);
}

pub fn cursor_position(editor: &Editor, width: u16, height: u16) -> (u16, u16) {
    if width == 0 || height == 0 {
        return (0, 0);
    }

    if editor.mode() == Mode::Command {
        let x = (1 + editor.command_cursor() as u16).min(width.saturating_sub(1));
        return (x, height.saturating_sub(1));
    }

    let scroll = editor.scroll();
    let screen_col = editor.cursor().col.saturating_sub(scroll.col);
    let screen_row = editor.cursor().row.saturating_sub(scroll.row);
    let gutter = line_number_width(editor.buffer().line_count(), width as usize) as u16;
    let x = gutter
        .saturating_add(screen_col as u16)
        .min(width.saturating_sub(1));
    let y = (screen_row as u16).min(text_area_height(height).saturating_sub(1));
    (x, y)
}

pub fn line_number_width(line_count: usize, screen_width: usize) -> usize {
    if screen_width == 0 {
        return 0;
    }

    (line_count.max(1).to_string().len() + 1).min(screen_width)
}

fn build_ui(editor: &Editor, _width: u16, height: u16) -> UiNode {
    let lines = buffer_lines(editor);
    let text_height = text_area_height(height);
    let text_style = Style::new().fg(FG).bg(BG);
    let selection_style = Style::new().fg(FG).bg(VISUAL_BG);
    let status_style = Style::new().fg(STATUS_FG).bg(STATUS_BG);
    let status_left = status_left(editor);
    let status_right = status_right(editor);
    let command_text = command_line_text(editor);

    let mut tree = jsx! {
        <Column height={height}>
            <TextArea
                lines={lines}
                cursor_line={editor.cursor().row as u16}
                cursor_col={editor.cursor().col as u16}
                scroll_top={editor.scroll().row as u16}
                scroll_left={editor.scroll().col as u16}
                height={text_height}
                style={text_style}
                selection_style={selection_style}
                gutter
            />
            <StatusBar style={status_style}>
                <Left>
                    <Text bold bg={STATUS_BG} color={STATUS_FG}>{status_left.as_str()}</Text>
                </Left>
                <Right>
                    <Text bg={STATUS_BG} color={STATUS_FG}>{status_right.as_str()}</Text>
                </Right>
            </StatusBar>
            <Text style={text_style}>{command_text.as_str()}</Text>
        </Column>
    };

    if let UiNode::Column(ref mut col) = tree {
        if let UiNode::TextArea(ref mut text_area) = col.children[0] {
            text_area.selection = editor.selection().cloned();
        }
    }

    tree
}

fn buffer_lines(editor: &Editor) -> Vec<StyledLine> {
    (0..editor.buffer().line_count())
        .map(|idx| StyledLine::plain(editor.buffer().line(idx)))
        .collect()
}

fn text_area_height(height: u16) -> u16 {
    height.saturating_sub(2)
}

fn status_left(editor: &Editor) -> String {
    let modified = if editor.buffer().is_modified() {
        " [+]"
    } else {
        ""
    };
    format!(" \"{}\"{}", editor.buffer().display_name(), modified)
}

fn status_right(editor: &Editor) -> String {
    format!(
        "{}:{}/{} ",
        editor.cursor().row + 1,
        editor.cursor().col + 1,
        editor.buffer().line_count()
    )
}

fn command_line_text(editor: &Editor) -> String {
    match editor.mode() {
        Mode::Command => format!(":{}", editor.command_input()),
        Mode::Insert => "-- INSERT --".into(),
        Mode::Replace => "-- REPLACE --".into(),
        Mode::Visual => "-- VISUAL --".into(),
        Mode::VisualLine => "-- VISUAL LINE --".into(),
        Mode::Normal => editor.message().unwrap_or("").into(),
    }
}

fn paint_filler_lines(buf: &mut ScreenBuffer, editor: &Editor, text_height: u16) {
    for row in 0..text_height {
        let line_idx = editor.scroll().row + row as usize;
        if line_idx < editor.buffer().line_count() {
            continue;
        }
        buf.set_char(
            0, row, '~', DIM, BG, false, true, false, false, false, false,
        );
    }
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
        assert_eq!(buf.get(0, 22).bg, STATUS_BG);
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
        assert_eq!(screen.get(2, 0).ch, 'h');
    }

    #[test]
    fn test_render_filler_lines_like_vim() {
        let editor = Editor::new(Buffer::new());
        let screen = render(&editor, 80, 6);
        assert_eq!(screen.get(0, 1).ch, '~');
        assert_eq!(screen.get(0, 2).ch, '~');
    }

    #[test]
    fn test_render_command_mode() {
        let mut editor = Editor::new(Buffer::new());
        use display_protocol::{KeyCode, KeyEvent, KeyModifiers};
        editor.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::EMPTY));
        editor.handle_key(KeyEvent::char('w'));
        let screen = render(&editor, 80, 24);
        assert_eq!(screen.get(0, 23).ch, ':');
        assert_eq!(screen.get(1, 23).ch, 'w');
        assert_eq!(cursor_position(&editor, 80, 24), (2, 23));
    }

    #[test]
    fn test_render_insert_mode_message() {
        let mut editor = Editor::new(Buffer::new());
        editor.handle_key(display_protocol::KeyEvent::char('i'));
        let screen = render(&editor, 80, 24);
        let text: String = (0..12).map(|x| screen.get(x, 23).ch).collect();
        assert_eq!(text, "-- INSERT --");
    }

    #[test]
    fn test_mode_cursor_overlay() {
        let editor = Editor::new(Buffer::new());
        let mut screen = render(&editor, 80, 24);
        draw_mode_cursor(&mut screen, &editor, true);
        let (x, y) = cursor_position(&editor, 80, 24);
        assert_eq!(screen.get(x, y).bg, CURSOR_BG);
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
