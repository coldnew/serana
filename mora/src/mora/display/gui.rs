use super::backend::{DisplayBackend, InputEvent, MoraRect, CellBuffer};
use super::event::MoraKeyEvent;
use super::style::MoraStyle;

pub struct GuiBackend {
    width: u16,
    height: u16,
    cells: Vec<Vec<(char, MoraStyle)>>,
}

impl GuiBackend {
    pub fn new(width: u16, height: u16) -> Self {
        let style = MoraStyle::default();
        let cells = vec![vec![(' ', style); width as usize]; height as usize];
        Self {
            width,
            height,
            cells,
        }
    }
}

impl DisplayBackend for GuiBackend {
    fn init(&mut self) -> Result<(), String> {
        self.cells = vec![
            vec![(' ', MoraStyle::default()); self.width as usize];
            self.height as usize
        ];
        Ok(())
    }

    fn cleanup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    fn clear(&mut self) {
        self.cells = vec![
            vec![(' ', MoraStyle::default()); self.width as usize];
            self.height as usize
        ];
    }

    fn flush(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn set_cell(&mut self, x: u16, y: u16, ch: char, style: MoraStyle) {
        if (x as usize) < self.width as usize && (y as usize) < self.height as usize {
            self.cells[y as usize][x as usize] = (ch, style);
        }
    }

    fn set_line(&mut self, x: u16, y: u16, text: &str, style: MoraStyle) {
        for (i, ch) in text.chars().enumerate() {
            let cx = x + i as u16;
            if cx >= self.width {
                break;
            }
            self.set_cell(cx, y, ch, style);
        }
    }

    fn poll_event(&mut self, _timeout_ms: u64) -> Option<InputEvent> {
        None
    }

    fn hide_cursor(&mut self) {}
    fn show_cursor(&mut self) {}
    fn set_cursor(&mut self, _x: u16, _y: u16) {}

    fn render_buffer(&mut self, buf: &CellBuffer) -> Result<(), String> {
        self.width = buf.width;
        self.height = buf.height;
        self.cells = (0..buf.height)
            .map(|y| {
                (0..buf.width)
                    .map(|x| {
                        let cell = buf.get(x, y);
                        (cell.ch, cell.style)
                    })
                    .collect()
            })
            .collect();
        Ok(())
    }
}
