use display_protocol::buffer::ScreenBuffer;
use display_protocol::{Color, Style, StyledLine, UiNode};
use display_protocol_jsx::jsx;

use crate::editor::Editor;
use crate::mode::Mode;
use crate::syntax::SyntaxHighlighter;

const BG: Color = Color::BLACK;
const FG: Color = Color::new(218, 218, 218);
const DIM: Color = Color::new(102, 102, 102);
const STATUS_FG: Color = Color::BLACK;
const STATUS_BG: Color = Color::new(218, 218, 218);
const STATUS_MODE_FG: Color = Color::WHITE;
const NORMAL_MODE_BG: Color = Color::new(60, 60, 60);
const INSERT_MODE_BG: Color = Color::new(0, 128, 0);
const VISUAL_MODE_BG: Color = Color::new(100, 100, 200);
const COMMAND_MODE_BG: Color = Color::new(180, 140, 60);
const REPLACE_MODE_BG: Color = Color::new(180, 60, 60);
const VISUAL_BG: Color = Color::new(70, 80, 110);
const CURSOR_BG: Color = Color::new(204, 204, 204);
const REPLACE_CURSOR: Color = Color::new(255, 100, 100);

pub fn render(editor: &Editor, width: u16, height: u16) -> ScreenBuffer {
    let mut buf = ScreenBuffer::new(width, height);

    if width == 0 || height == 0 {
        return buf;
    }

    let tree = build_ui(editor, width, height);
    buf = display_protocol::collect_render_commands(&tree, width, height).into_buffer();
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

    // digit_count(line_count.max(1)) + 1 for the separator column
    let n = line_count.max(1);
    let digits = if n < 10 {
        1
    } else if n < 100 {
        2
    } else if n < 1_000 {
        3
    } else if n < 10_000 {
        4
    } else if n < 100_000 {
        5
    } else if n < 1_000_000 {
        6
    } else {
        n.ilog10() as usize + 1
    };
    (digits + 1).min(screen_width)
}

fn build_ui(editor: &Editor, _width: u16, height: u16) -> UiNode {
    let text_height = text_area_height(height);
    let lines = buffer_lines(editor);
    let text_style = Style::new().fg(FG).bg(BG);
    let selection_style = Style::new().fg(FG).bg(VISUAL_BG);
    let status_style = Style::new().fg(STATUS_FG).bg(STATUS_BG);
    let mode_label = editor.mode().mode_label();
    let mode_bg = mode_bg(editor.mode());
    let file_status = file_status(editor);
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
                    <Text bold bg={mode_bg} color={STATUS_MODE_FG}>{mode_label}</Text>
                    <Text bold bg={STATUS_BG} color={STATUS_FG}>{file_status.as_str()}</Text>
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
    let total = editor.buffer().line_count();
    if total == 0 {
        return Vec::new();
    }
    let lines: Vec<&str> = (0..total).map(|idx| editor.buffer().line(idx)).collect();
    SyntaxHighlighter::global().highlight_buffer(&lines, editor.buffer().path())
}

fn text_area_height(height: u16) -> u16 {
    height.saturating_sub(2)
}

fn mode_bg(mode: Mode) -> Color {
    match mode {
        Mode::Normal => NORMAL_MODE_BG,
        Mode::Insert => INSERT_MODE_BG,
        Mode::Visual | Mode::VisualLine => VISUAL_MODE_BG,
        Mode::Command => COMMAND_MODE_BG,
        Mode::Replace => REPLACE_MODE_BG,
    }
}

fn file_status(editor: &Editor) -> String {
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
        assert_eq!(buf.get(0, 22).bg, NORMAL_MODE_BG);
        assert_eq!(buf.get(8, 22).bg, STATUS_BG);
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
    fn test_render_highlights_path_backed_rust_buffer() {
        let path = std::env::temp_dir().join(format!(
            "vivi-highlight-{}-{}.rs",
            std::process::id(),
            "rust"
        ));
        std::fs::write(&path, "fn main() {}").unwrap();

        let editor = Editor::new(Buffer::from_file(&path).unwrap());
        let screen = render(&editor, 80, 24);

        assert_eq!(screen.get(2, 0).ch, 'f');
        assert_ne!(screen.get(2, 0).fg, FG);

        let _ = std::fs::remove_file(path);
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
    fn test_modeline_mode_color_changes_by_mode() {
        let mut editor = Editor::new(Buffer::new());

        let normal = render(&editor, 80, 24);
        assert_eq!(normal.get(0, 22).bg, NORMAL_MODE_BG);

        editor.handle_key(display_protocol::KeyEvent::char('i'));
        let insert = render(&editor, 80, 24);
        assert_eq!(insert.get(0, 22).bg, INSERT_MODE_BG);
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

    #[test]
    fn test_G_cursor_not_on_filler_line() {
        use display_protocol::KeyEvent;

        for n in 1..=50 {
            let mut buf = Buffer::new();
            buf.replace_range(0, 0, 0, "line0");
            for i in 1..n {
                buf.insert_line(i, format!("line{}", i));
            }
            let mut editor = Editor::new(buf);
            editor.set_term_size(80, 24);

            editor.handle_key(KeyEvent::char('G'));

            // Cursor must be on a valid line
            assert!(
                editor.cursor().row < editor.buffer().line_count(),
                "G on {}-line file: cursor.row={} >= line_count={}",
                n,
                editor.cursor().row,
                editor.buffer().line_count()
            );

            // Rendered cursor must be within the text area
            let (cx, cy) = cursor_position(&editor, 80, 24);
            let text_height = 22u16;
            assert!(
                cy < text_height,
                "G on {}-line file: cursor y={} >= text_height={}",
                n,
                cy,
                text_height
            );

            // The cell at cursor position must not be a filler '~'
            let screen = render(&editor, 80, 24);
            let ch = screen.get(cx, cy).ch;
            assert_ne!(
                ch, '~',
                "G on {}-line file: cursor on filler line at ({}, {}), cursor.row={}, scroll.row={}",
                n, cx, cy, editor.cursor().row, editor.scroll().row
            );
        }
    }
    #[test]
    fn test_G_cursor_various_term_sizes() {
        use display_protocol::KeyEvent;

        let term_sizes = [(80, 24), (80, 10), (40, 6), (80, 4)];
        let file_sizes = [1, 2, 3, 5, 10, 22, 50, 100, 200];

        for &(tw, th) in &term_sizes {
            for &n in &file_sizes {
                let mut buf = Buffer::new();
                buf.replace_range(0, 0, 0, "line0");
                for i in 1..n {
                    buf.insert_line(i, format!("line{}", i));
                }
                let mut editor = Editor::new(buf);
                editor.set_term_size(tw, th);

                editor.handle_key(KeyEvent::char('G'));

                assert!(
                    editor.cursor().row < editor.buffer().line_count(),
                    "G on {}-line file, {}x{} term: cursor.row={} >= line_count={}",
                    n,
                    tw,
                    th,
                    editor.cursor().row,
                    editor.buffer().line_count()
                );

                let (cx, cy) = cursor_position(&editor, tw, th);
                let text_height = th.saturating_sub(2);
                if text_height > 0 {
                    assert!(
                        cy < text_height,
                        "G on {}-line file, {}x{} term: cursor y={} >= text_height={}",
                        n,
                        tw,
                        th,
                        cy,
                        text_height
                    );

                    let screen = render(&editor, tw, th);
                    let ch = screen.get(cx, cy).ch;
                    assert_ne!(
                        ch, '~',
                        "G on {}-line file, {}x{} term: cursor on filler at ({}, {}), cursor.row={}, scroll.row={}",
                        n, tw, th, cx, cy, editor.cursor().row, editor.scroll().row
                    );
                }
            }
        }
    }

    #[test]
    fn test_g_integration_from_disk() {
        use display_protocol::KeyEvent;
        use std::fs;

        for n in [1, 2, 3, 5, 10, 50, 100] {
            let path =
                std::env::temp_dir().join(format!("vivi-g-integ-{}-{}.txt", std::process::id(), n));
            let content: String = (0..n).map(|i| format!("line{}\n", i)).collect();
            fs::write(&path, &content).unwrap();

            let buf = Buffer::from_file(&path).unwrap();
            let line_count = buf.line_count();
            let mut editor = Editor::new(buf);
            editor.set_term_size(80, 24);

            editor.handle_key(KeyEvent::char('G'));

            assert_eq!(
                editor.cursor().row,
                line_count - 1,
                "G on {}-line file: cursor.row={} expected {}",
                n,
                editor.cursor().row,
                line_count - 1
            );

            // Check every screen row
            let screen = render(&editor, 80, 24);
            let text_height: usize = 22;

            for row in 0..text_height {
                let ch = screen.get(0, row as u16).ch;
                let line_idx = editor.scroll().row + row;

                if line_idx < line_count {
                    assert_ne!(
                        ch, '~',
                        "G on {}-line file: row {} (line_idx={}) has '~' but should have content. scroll.row={}",
                        n, row, line_idx, editor.scroll().row
                    );
                }
            }

            // Cursor must map to a valid content row
            let (cx, cy) = cursor_position(&editor, 80, 24);
            let cursor_line_idx = editor.scroll().row + cy as usize;
            assert!(
                cursor_line_idx < line_count,
                "G on {}-line file: cursor y={} maps to line_idx={} >= line_count={}. scroll.row={}, cursor.row={}",
                n, cy, cursor_line_idx, line_count, editor.scroll().row, editor.cursor().row
            );

            let _ = fs::remove_file(&path);
        }
    }

    #[test]
    fn test_g_main_loop_simulation() {
        // Simulates the exact flow in main.rs:
        //   screen = render(&editor, w, h)
        //   draw_mode_cursor(&mut screen, &editor, true)
        use display_protocol::KeyEvent;

        for n in [1, 2, 3, 5, 10, 50] {
            let mut buf = Buffer::new();
            buf.replace_range(0, 0, 0, "line0");
            for i in 1..n {
                buf.insert_line(i, format!("line{}", i));
            }
            let mut editor = Editor::new(buf);
            editor.set_term_size(80, 24);

            // Simulate opening the file and pressing G
            editor.handle_key(KeyEvent::char('G'));

            // Simulate the main loop render cycle
            let mut screen = render(&editor, 80, 24);
            draw_mode_cursor(&mut screen, &editor, true);

            // Where is the cursor drawn?
            let (cx, cy) = cursor_position(&editor, 80, 24);
            assert!(cx < 80 && cy < 22, "cursor out of bounds: ({}, {})", cx, cy);

            // What character is UNDER the cursor?
            let ch = screen.get(cx, cy).ch;
            assert_ne!(
                ch,
                '~',
                "G on {}-line file: cursor drawn on '~' at screen ({}, {}). \
                cursor.row={}, scroll.row={}, line_count={}",
                n,
                cx,
                cy,
                editor.cursor().row,
                editor.scroll().row,
                editor.buffer().line_count()
            );

            // Also verify the cursor bg is set (draw_mode_cursor applied)
            assert_eq!(
                screen.get(cx, cy).bg,
                CURSOR_BG,
                "G on {}-line file: cursor bg not applied at ({}, {})",
                n,
                cx,
                cy
            );
        }
    }
}
