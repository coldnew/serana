use crate::mora::display::style::{MoraColor, MoraStyle};
use crate::mora::editor::MoraEditor;
use crate::mora::ui::EditorWidget;
use crate::mora::ui_node;
use crate::mora::display::backend::{InputEvent, MouseKind};
use crate::mora::display::event::{MoraKeyEvent, MoraKeyCode};
use display_protocol::{
    Cell, Color, CursorState, CursorStyle, FrameUpdate, Grid, StatusLine, Style,
    DisplayCmd, InputEvent as ProtoInputEvent, KeyEvent, KeyCode,
    compute_layout, paint,
};
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use std::path::Path;

/// Headless editor engine. Produces FrameUpdate protocol messages
/// and consumes InputEvent protocol messages. No display dependency.
pub struct MoraCore {
    pub editor: MoraEditor,
    width: u16,
    height: u16,
}

impl MoraCore {
    pub fn new(width: u16, height: u16) -> Self {
        let editor_height = height.saturating_sub(3) as usize;
        Self {
            editor: MoraEditor::new(editor_height),
            width,
            height,
        }
    }

    pub fn open(path: &Path, width: u16, height: u16) -> Result<Self, String> {
        let editor_height = height.saturating_sub(3) as usize;
        let editor = MoraEditor::open(path, editor_height)
            .map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
        Ok(Self {
            editor,
            width,
            height,
        })
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        let editor_height = height.saturating_sub(3) as usize;
        self.editor.set_height(editor_height);
    }

    /// Render the current editor state as a FrameUpdate.
    pub fn render_frame(&self) -> FrameUpdate {
        let area = Rect::new(0, 0, self.width, self.height);
        let mut ratatui_buf = ratatui::buffer::Buffer::empty(area);
        let widget = EditorWidget::new(&self.editor);
        widget.render(area, &mut ratatui_buf);

        let mut grid = Grid::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let cell = &ratatui_buf[(x, y)];
                let fg: Color = display_tui::conversions::color_from_ratatui(cell.fg);
                let bg: Color = display_tui::conversions::color_from_ratatui(cell.bg);
                let style = Style {
                    fg: Some(fg),
                    bg: Some(bg),
                    ..Style::default()
                };
                let ch = cell.symbol().chars().next().unwrap_or(' ');
                grid.set(x, y, Cell { ch, style });
            }
        }

        let cursor_visible = !matches!(
            self.editor.mode(),
            crate::mora::keymap::EditorMode::Insert
                | crate::mora::keymap::EditorMode::Normal
        ) || true; // always visible in most modes

        let cursor_style = match self.editor.mode() {
            crate::mora::keymap::EditorMode::Insert => CursorStyle::Bar,
            crate::mora::keymap::EditorMode::Normal => CursorStyle::Block,
            _ => CursorStyle::Block,
        };

        FrameUpdate {
            grid,
            cursor: CursorState {
                x: self.editor.buffer.cursor.col as u16,
                y: self.editor.buffer.cursor.row as u16,
                visible: cursor_visible,
                style: cursor_style,
            },
            status_line: StatusLine::default(),
            command_line: None,
            help_bar: None,
            full_redraw: true,
        }
    }

    /// Render editor state using the declarative UiNode pipeline.
    /// Builds a UiNode tree → layout → paint → Grid.
    pub fn render_ui_frame(&self) -> FrameUpdate {
        let ui = ui_node::build_ui(&self.editor, self.width, self.height);
        let layout = compute_layout(&ui, self.width, self.height);
        let buf = paint(&ui, self.width, self.height);

        let mut grid = Grid::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let cell = buf.get(x, y);
                let style = Style {
                    fg: Some(Color::new(cell.fg.r, cell.fg.g, cell.fg.b)),
                    bg: Some(Color::new(cell.bg.r, cell.bg.g, cell.bg.b)),
                    bold: cell.bold,
                    italic: cell.italic,
                    underline: cell.underline,
                    strikethrough: cell.strikethrough,
                    dim: cell.dim,
                    reverse: cell.reverse,
                    ..Style::default()
                };
                grid.set(x, y, Cell { ch: cell.ch, style });
            }
        }

        let cursor_visible = true;
        let cursor_style = match self.editor.mode() {
            crate::mora::keymap::EditorMode::Insert => CursorStyle::Bar,
            _ => CursorStyle::Block,
        };

        FrameUpdate {
            grid,
            cursor: CursorState {
                x: self.editor.buffer.cursor.col as u16,
                y: self.editor.buffer.cursor.row as u16,
                visible: cursor_visible,
                style: cursor_style,
            },
            status_line: StatusLine::default(),
            command_line: None,
            help_bar: None,
            full_redraw: true,
        }
    }

    /// Handle a protocol InputEvent. Returns any DisplayCmds produced.
    pub fn handle_input(&mut self, event: ProtoInputEvent) -> Vec<DisplayCmd> {
        match event {
            ProtoInputEvent::Key(key) => {
                let mora_key = proto_key_to_mora(key);
                if mora_key.modifiers.ctrl && mora_key.code == MoraKeyCode::Char('c') {
                    return vec![DisplayCmd::Quit];
                }
                self.editor.handle_key(mora_key);
            }
            ProtoInputEvent::Resize { width, height } => {
                self.resize(width, height);
            }
            ProtoInputEvent::Mouse { .. } => {
                // TODO: mouse support
            }
            ProtoInputEvent::FocusGained | ProtoInputEvent::FocusLost => {}
            ProtoInputEvent::Paste(text) => {
                for ch in text.chars() {
                    self.editor.handle_key(MoraKeyEvent::new(
                        MoraKeyCode::Char(ch),
                        Default::default(),
                    ));
                }
            }
        }
        Vec::new()
    }

    /// Handle a mora InputEvent (from backends). Returns true if should quit.
    pub fn handle_mora_input(&mut self, event: InputEvent) -> bool {
        match event {
            InputEvent::Key(key) => {
                if key.modifiers.ctrl && key.code == MoraKeyCode::Char('c') {
                    return true;
                }
                self.editor.handle_key(key);
            }
            InputEvent::Resize(_w, h) => {
                let editor_height = h.saturating_sub(3) as usize;
                self.editor.set_height(editor_height);
            }
            _ => {}
        }
        self.editor.quit_requested()
    }

    pub fn quit_requested(&self) -> bool {
        self.editor.quit_requested()
    }
}

fn proto_key_to_mora(key: KeyEvent) -> MoraKeyEvent {
    let code = match key.code {
        KeyCode::Char(c) => MoraKeyCode::Char(c),
        KeyCode::Enter => MoraKeyCode::Enter,
        KeyCode::Tab => MoraKeyCode::Tab,
        KeyCode::Backspace => MoraKeyCode::Backspace,
        KeyCode::Delete => MoraKeyCode::Delete,
        KeyCode::Esc => MoraKeyCode::Esc,
        KeyCode::Left => MoraKeyCode::Left,
        KeyCode::Right => MoraKeyCode::Right,
        KeyCode::Up => MoraKeyCode::Up,
        KeyCode::Down => MoraKeyCode::Down,
        KeyCode::Home => MoraKeyCode::Home,
        KeyCode::End => MoraKeyCode::End,
        KeyCode::PageUp => MoraKeyCode::PageUp,
        KeyCode::PageDown => MoraKeyCode::PageDown,
        KeyCode::F(n) => MoraKeyCode::F(n),
        KeyCode::Insert => MoraKeyCode::Insert,
        KeyCode::BackTab => MoraKeyCode::BackTab,
    };
    MoraKeyEvent::new(code, Default::default())
}
