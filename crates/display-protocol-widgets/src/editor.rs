use display_protocol::{StyledLine, Style, Selection, TextAreaNode, UiNode};
use crate::palette;

/// A multi-line text editor / text area.
///
/// Works on both TUI and WGPU backends.
///
/// ```ignore
/// Editor::new(vec!["fn main() {", "    println!(\"hi\");", "}"])
///     .cursor(1, 4)
///     .gutter(true)
///     .build()
///
/// Editor::from_lines(lines)
///     .height(30)
///     .scroll(10, 0)
///     .build()
/// ```
#[derive(Debug, Clone)]
pub struct Editor {
    lines: Vec<String>,
    cursor_line: u16,
    cursor_col: u16,
    selection: Option<Selection>,
    scroll_top: u16,
    scroll_left: u16,
    height: u16,
    gutter: bool,
    focused: bool,
}

impl Editor {
    pub fn new(lines: Vec<impl Into<String>>) -> Self {
        Self {
            lines: lines.into_iter().map(|l| l.into()).collect(),
            cursor_line: 0,
            cursor_col: 0,
            selection: None,
            scroll_top: 0,
            scroll_left: 0,
            height: 24,
            gutter: true,
            focused: false,
        }
    }

    pub fn from_lines(lines: Vec<String>) -> Self {
        Self {
            lines,
            cursor_line: 0,
            cursor_col: 0,
            selection: None,
            scroll_top: 0,
            scroll_left: 0,
            height: 24,
            gutter: true,
            focused: false,
        }
    }

    pub fn cursor(mut self, line: u16, col: u16) -> Self {
        self.cursor_line = line;
        self.cursor_col = col;
        self
    }

    pub fn selection(mut self, sel: Selection) -> Self { self.selection = Some(sel); self }
    pub fn scroll(mut self, top: u16, left: u16) -> Self { self.scroll_top = top; self.scroll_left = left; self }
    pub fn height(mut self, h: u16) -> Self { self.height = h; self }
    pub fn gutter(mut self, show: bool) -> Self { self.gutter = show; self }
    pub fn focused(mut self, f: bool) -> Self { self.focused = f; self }

    pub fn build(self) -> UiNode {
        let styled_lines: Vec<StyledLine> = self.lines
            .into_iter()
            .map(StyledLine::plain)
            .collect();

        let cursor_style = if self.focused {
            Style::default().reverse()
        } else {
            Style::default()
        };

        UiNode::TextArea(TextAreaNode {
            lines: styled_lines,
            cursor_line: self.cursor_line,
            cursor_col: self.cursor_col,
            selection: self.selection,
            scroll_top: self.scroll_top,
            scroll_left: self.scroll_left,
            height: self.height,
            style: Style::default().fg(palette::LIGHT),
            cursor_style,
            selection_style: Style::default().bg(palette::Color::new(40, 60, 100)),
            gutter: self.gutter,
            focused: self.focused,
        })
    }
}

impl From<Editor> for UiNode {
    fn from(e: Editor) -> Self { e.build() }
}
