use crate::buffer::ScreenBuffer;

/// Backend-agnostic renderer trait.
///
/// Different backends (TUI/ANSI, WGPU, Web) implement this trait
/// to render a ScreenBuffer to their output medium.
pub trait Renderer {
    type Error;

    /// Initialize the renderer.
    fn init(&mut self) -> Result<(), Self::Error>;

    /// Cleanup / restore terminal state.
    fn cleanup(&mut self) -> Result<(), Self::Error>;

    /// Get the current size (width, height).
    fn size(&self) -> (u16, u16);

    /// Render a ScreenBuffer to the output. Uses diffing against previous frame.
    fn render(&mut self, buffer: &ScreenBuffer) -> Result<(), Self::Error>;

    /// Flush any pending output.
    fn flush(&mut self) -> Result<(), Self::Error>;

    /// Set the cursor position.
    fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), Self::Error>;

    /// Hide the cursor.
    fn hide_cursor(&mut self) -> Result<(), Self::Error>;

    /// Show the cursor.
    fn show_cursor(&mut self) -> Result<(), Self::Error>;
}

/// ANSI terminal renderer — converts ScreenBuffer diff to ANSI escape sequences.
///
/// Like storm-rs's DiffRenderer, this maintains a previous buffer and only
/// emits ANSI sequences for changed cells.
pub struct AnsiRenderer {
    prev_buffer: Option<ScreenBuffer>,
    width: u16,
    height: u16,
    output: String,
}

impl AnsiRenderer {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            prev_buffer: None,
            width,
            height,
            output: String::new(),
        }
    }

    /// Build the ANSI output string for the diff between prev and current buffer.
    fn build_diff(&mut self, current: &ScreenBuffer) -> String {
        let mut out = String::new();

        match &self.prev_buffer {
            Some(prev) => {
                let changes = current.diff(prev);
                for (x, y, cell) in changes {
                    // Move cursor
                    out.push_str(&format!("\x1b[{};{}H", y + 1, x + 1));
                    // SGR
                    out.push_str(&cell_to_sgr(&cell));
                    // Character
                    out.push(cell.ch);
                }
            }
            None => {
                // Full redraw
                out.push_str("\x1b[2J\x1b[H"); // Clear screen, home cursor
                for y in 0..current.height {
                    for x in 0..current.width {
                        let cell = current.get(x, y);
                        if x == 0 && y > 0 {
                            out.push_str("\r\n");
                        }
                        out.push_str(&cell_to_sgr(&cell));
                        out.push(cell.ch);
                    }
                }
            }
        }

        out.push_str("\x1b[0m"); // Reset
        out
    }
}

impl Renderer for AnsiRenderer {
    type Error = String;

    fn init(&mut self) -> Result<(), String> {
        self.output.push_str("\x1b[?1049h"); // Enter alternate screen
        self.output.push_str("\x1b[?25l"); // Hide cursor
        Ok(())
    }

    fn cleanup(&mut self) -> Result<(), String> {
        self.output.push_str("\x1b[?25h"); // Show cursor
        self.output.push_str("\x1b[?1049l"); // Leave alternate screen
        Ok(())
    }

    fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    fn render(&mut self, buffer: &ScreenBuffer) -> Result<(), String> {
        let diff_output = self.build_diff(buffer);
        self.output.push_str(&diff_output);
        self.prev_buffer = Some(buffer.clone());
        Ok(())
    }

    fn flush(&mut self) -> Result<(), String> {
        // Output is accumulated in self.output, caller reads it
        Ok(())
    }

    fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), String> {
        self.output.push_str(&format!("\x1b[{};{}H", y + 1, x + 1));
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), String> {
        self.output.push_str("\x1b[?25l");
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<(), String> {
        self.output.push_str("\x1b[?25h");
        Ok(())
    }
}

impl AnsiRenderer {
    /// Take the accumulated output string.
    pub fn take_output(&mut self) -> String {
        std::mem::take(&mut self.output)
    }

    /// Resize the renderer.
    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.prev_buffer = None; // Force full redraw
    }
}

/// Convert a ScreenCell to ANSI SGR escape sequence.
fn cell_to_sgr(cell: &crate::buffer::ScreenCell) -> String {
    let mut sgr = String::from("\x1b[0");

    if cell.bold {
        sgr.push_str(";1");
    }
    if cell.dim {
        sgr.push_str(";2");
    }
    if cell.italic {
        sgr.push_str(";3");
    }
    if cell.underline {
        sgr.push_str(";4");
    }
    if cell.blink {
        sgr.push_str(";5");
    }
    if cell.reverse {
        sgr.push_str(";7");
    }
    if cell.strikethrough {
        sgr.push_str(";9");
    }

    // Foreground
    sgr.push_str(&format!(";38;2;{};{};{}", cell.fg.r, cell.fg.g, cell.fg.b));
    // Background
    sgr.push_str(&format!(";48;2;{};{};{}", cell.bg.r, cell.bg.g, cell.bg.b));

    // Underline color (OSC 58)
    if let Some(uc) = cell.underline_color {
        sgr.push_str(&format!(";58;2;{};{};{}", uc.r, uc.g, uc.b));
    }

    sgr.push('m');
    sgr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ansi_renderer_full_redraw() {
        let mut renderer = AnsiRenderer::new(10, 3);
        let mut buf = ScreenBuffer::new(10, 3);
        buf.set_char(
            0,
            0,
            'H',
            Color::WHITE,
            Color::BLACK,
            false,
            false,
            false,
            false,
            false,
            false,
        );
        renderer.render(&buf).unwrap();
        let output = renderer.take_output();
        assert!(output.contains("\x1b[2J")); // Clear screen
        assert!(output.contains('H'));
    }

    #[test]
    fn test_ansi_renderer_diff() {
        let mut renderer = AnsiRenderer::new(10, 3);
        let buf1 = ScreenBuffer::new(10, 3);
        renderer.render(&buf1).unwrap();
        renderer.take_output(); // Clear first frame

        let mut buf2 = ScreenBuffer::new(10, 3);
        buf2.set_char(
            5,
            2,
            'X',
            Color::RED,
            Color::BLACK,
            false,
            false,
            false,
            false,
            false,
            false,
        );
        renderer.render(&buf2).unwrap();
        let output = renderer.take_output();
        assert!(output.contains('X'));
        assert!(!output.contains("\x1b[2J")); // Should NOT be a full redraw
    }

    #[test]
    fn test_cell_to_sgr() {
        let cell = crate::buffer::ScreenCell::new('A', Color::RED, Color::BLUE);
        let sgr = cell_to_sgr(&cell);
        assert!(sgr.contains("38;2;255;0;0"));
        assert!(sgr.contains("48;2;0;0;255"));
    }
}
