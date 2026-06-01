use crate::mora::display::backend::InputEvent;
use crate::mora::display::event::{MoraKeyCode, MoraKeyEvent};
use crate::mora::editor::MoraEditor;
use crate::mora::ui_node;
use display_protocol::{
    collect_render_commands, render_commands_to_buffer, Cell, Color, CursorState, CursorStyle,
    DisplayCmd, FrameUpdate, Grid, InputEvent as ProtoInputEvent, KeyCode, KeyEvent,
    MouseEventKind, Style, UiNode,
};
use std::path::Path;

/// Headless editor engine. Produces FrameUpdate protocol messages
/// and consumes InputEvent protocol messages. No display dependency.
pub struct MoraCore {
    pub editor: MoraEditor,
    pub show_menu_bar: bool,
    width: u16,
    height: u16,
    cell_width: f32,
    cell_height: f32,
}

impl MoraCore {
    pub fn new(width: u16, height: u16) -> Self {
        let editor_height = height.saturating_sub(3) as usize;
        let mut editor = MoraEditor::new(editor_height);
        editor.init_scratch_buffer();
        Self {
            editor,
            show_menu_bar: true,
            width,
            height,
            cell_width: 8.0,
            cell_height: 16.0,
        }
    }

    pub fn open(path: &Path, width: u16, height: u16) -> Result<Self, String> {
        let editor_height = height.saturating_sub(3) as usize;
        let editor = MoraEditor::open(path, editor_height)
            .map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
        Ok(Self {
            editor,
            show_menu_bar: true,
            width,
            height,
            cell_width: 8.0,
            cell_height: 16.0,
        })
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        let editor_height = height.saturating_sub(3) as usize;
        self.editor.set_height(editor_height);
    }

    pub fn width(&self) -> u16 {
        self.width
    }
    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn set_cell_size(&mut self, cell_width: f32, cell_height: f32) {
        self.cell_width = cell_width;
        self.cell_height = cell_height;
    }

    /// Render editor state using the declarative UiNode pipeline.
    /// Builds a UiNode tree → render commands → ScreenBuffer → Grid.
    pub fn render_ui_frame(&mut self) -> FrameUpdate {
        let ui = ui_node::build_ui(
            &mut self.editor,
            self.width,
            self.height,
            self.show_menu_bar,
        );
        let commands = collect_render_commands(&ui, self.width, self.height);
        let buf = render_commands_to_buffer(commands.commands(), self.width, self.height);

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
            status_line: Default::default(),
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
            ProtoInputEvent::Mouse { x, y, kind, .. } => {
                if kind == MouseEventKind::Press {
                    let grid_x = (x as f32 / self.cell_width) as u16;
                    let grid_y = (y as f32 / self.cell_height) as u16;
                    self.handle_mouse_press(grid_x, grid_y);
                }
            }
            ProtoInputEvent::FocusGained | ProtoInputEvent::FocusLost => {}
            ProtoInputEvent::Paste(text) => {
                for ch in text.chars() {
                    self.editor
                        .handle_key(MoraKeyEvent::new(MoraKeyCode::Char(ch), Default::default()));
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

    /// Handle a mouse press at grid coordinates.
    fn handle_mouse_press(&mut self, grid_x: u16, grid_y: u16) {
        // Menu bar occupies row 0 when enabled.
        if self.show_menu_bar && grid_y == 0 {
            // Menu item column ranges: (name, start_col, end_col) inclusive.
            const ITEMS: &[(&str, u16, u16)] = &[
                ("File", 1, 4),
                ("Edit", 7, 10),
                ("Options", 13, 19),
                ("Buffers", 22, 28),
                ("Tools", 31, 35),
                ("Help", 38, 41),
            ];
            for (i, &(_name, start, end)) in ITEMS.iter().enumerate() {
                if grid_x >= start && grid_x <= end {
                    self.editor.menu_bar_selection = Some(i);
                    return;
                }
            }
            // Clicked on menu bar but not on an item — deselect.
            self.editor.menu_bar_selection = None;
            return;
        }
        // Click outside menu bar clears selection.
        self.editor.menu_bar_selection = None;
    }

    pub fn quit_requested(&self) -> bool {
        self.editor.quit_requested()
    }

    /// Build a UiNode tree directly for GPU rendering.
    /// This bypasses the FrameUpdate→Grid conversion used by the TUI path.
    pub fn build_ui_node(&mut self, width: u16, height: u16) -> UiNode {
        ui_node::build_ui(&mut self.editor, width, height, self.show_menu_bar)
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
        _ => MoraKeyCode::Char('\0'),
    };
    let modifiers = crate::mora::display::event::MoraKeyModifiers {
        ctrl: key.modifiers.ctrl,
        alt: key.modifiers.alt,
        shift: key.modifiers.shift,
        super_key: key.modifiers.super_key,
    };
    MoraKeyEvent::new(code, modifiers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn grid_row_text(grid: &Grid, row: u16) -> String {
        let mut out = String::new();
        for x in 0..grid.width {
            let ch = grid.get(x, row).ch;
            if ch == '\0' {
                break;
            }
            out.push(ch);
        }
        out.trim_end().to_string()
    }

    #[test]
    fn menu_bar_is_rendered_only_when_enabled() {
        let path = Path::new("mora/src/mora/editor.rs");

        let mut gui_core = MoraCore::open(path, 80, 24).expect("open editor");
        gui_core.show_menu_bar = true;
        let gui_frame = gui_core.render_ui_frame();
        assert_eq!(
            grid_row_text(&gui_frame.grid, 0),
            " File  Edit  Options  Buffers  Tools  Help"
        );

        let mut tui_core = MoraCore::open(path, 80, 24).expect("open editor");
        tui_core.show_menu_bar = false;
        let tui_frame = tui_core.render_ui_frame();
        let row0 = grid_row_text(&tui_frame.grid, 0);
        assert_ne!(row0, " File  Edit  Options  Buffers  Tools  Help");
    }
}
