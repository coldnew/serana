use crate::buffer::{ScreenBuffer, ScreenCell};
use crate::types::{Color, Style, StyledLine};

/// A renderer-agnostic drawing instruction produced by the layout pipeline.
#[derive(Debug, Clone)]
pub enum RenderCommand {
    SetCell {
        x: u16,
        y: u16,
        cell: ScreenCell,
    },
    FillRect {
        x: u16,
        y: u16,
        w: u16,
        h: u16,
        cell: ScreenCell,
    },
    WriteStr {
        x: u16,
        y: u16,
        text: String,
        fg: Color,
        bg: Color,
        bold: bool,
        dim: bool,
    },
    WriteStyled {
        x: u16,
        y: u16,
        text: String,
        style: Style,
    },
    WriteStyledLine {
        x: u16,
        y: u16,
        line: StyledLine,
        default_fg: Color,
        default_bg: Color,
    },
    HLine {
        x: u16,
        y: u16,
        width: u16,
        ch: char,
        fg: Color,
        bg: Color,
    },
    DrawBorder {
        x: u16,
        y: u16,
        w: u16,
        h: u16,
        fg: Color,
        bg: Color,
        title: Option<String>,
    },
}

/// Render target interface shared by buffer-backed and command-backed paths.
pub trait RenderTarget {
    fn width(&self) -> u16;
    fn height(&self) -> u16;
    fn get(&self, x: u16, y: u16) -> ScreenCell;
    fn set(&mut self, x: u16, y: u16, cell: ScreenCell);
    fn set_char(
        &mut self,
        x: u16,
        y: u16,
        ch: char,
        fg: Color,
        bg: Color,
        bold: bool,
        dim: bool,
        underline: bool,
        strikethrough: bool,
        italic: bool,
        reverse: bool,
    );
    fn write_str(&mut self, x: u16, y: u16, s: &str, fg: Color, bg: Color, bold: bool, dim: bool);
    fn write_styled(&mut self, x: u16, y: u16, s: &str, style: &Style);
    fn fill_char(&mut self, x: u16, y: u16, w: u16, h: u16, ch: char, fg: Color, bg: Color);
    fn hline(&mut self, x: u16, y: u16, width: u16, ch: char, fg: Color, bg: Color);
    fn draw_border(
        &mut self,
        x: u16,
        y: u16,
        w: u16,
        h: u16,
        fg: Color,
        bg: Color,
        title: Option<&str>,
    );
    fn write_styled_line(
        &mut self,
        x: u16,
        y: u16,
        line: &StyledLine,
        default_fg: Color,
        default_bg: Color,
    );
}

/// A render command stream plus a shadow buffer for read-after-write layout logic.
#[derive(Debug, Clone)]
pub struct RenderCommandArray {
    pub width: u16,
    pub height: u16,
    commands: Vec<RenderCommand>,
    shadow: ScreenBuffer,
}

impl RenderCommandArray {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            commands: Vec::new(),
            shadow: ScreenBuffer::new(width, height),
        }
    }

    pub fn commands(&self) -> &[RenderCommand] {
        &self.commands
    }

    pub fn into_commands(self) -> Vec<RenderCommand> {
        self.commands
    }

    pub fn into_buffer(self) -> ScreenBuffer {
        self.shadow
    }

    fn push(&mut self, command: RenderCommand) {
        apply_render_command(&mut self.shadow, &command);
        self.commands.push(command);
    }
}

impl RenderTarget for RenderCommandArray {
    fn width(&self) -> u16 {
        self.width
    }

    fn height(&self) -> u16 {
        self.height
    }

    fn get(&self, x: u16, y: u16) -> ScreenCell {
        self.shadow.get(x, y)
    }

    fn set(&mut self, x: u16, y: u16, cell: ScreenCell) {
        self.push(RenderCommand::SetCell { x, y, cell });
    }

    fn set_char(
        &mut self,
        x: u16,
        y: u16,
        ch: char,
        fg: Color,
        bg: Color,
        bold: bool,
        dim: bool,
        underline: bool,
        strikethrough: bool,
        italic: bool,
        reverse: bool,
    ) {
        self.push(RenderCommand::SetCell {
            x,
            y,
            cell: ScreenCell {
                ch,
                fg,
                bg,
                bold,
                italic,
                underline,
                strikethrough,
                dim,
                reverse,
                blink: false,
                underline_color: None,
                hyperlink: None,
            },
        });
    }

    fn write_str(&mut self, x: u16, y: u16, s: &str, fg: Color, bg: Color, bold: bool, dim: bool) {
        self.push(RenderCommand::WriteStr {
            x,
            y,
            text: s.to_string(),
            fg,
            bg,
            bold,
            dim,
        });
    }

    fn write_styled(&mut self, x: u16, y: u16, s: &str, style: &Style) {
        self.push(RenderCommand::WriteStyled {
            x,
            y,
            text: s.to_string(),
            style: *style,
        });
    }

    fn fill_char(&mut self, x: u16, y: u16, w: u16, h: u16, ch: char, fg: Color, bg: Color) {
        self.push(RenderCommand::FillRect {
            x,
            y,
            w,
            h,
            cell: ScreenCell::new(ch, fg, bg),
        });
    }

    fn hline(&mut self, x: u16, y: u16, width: u16, ch: char, fg: Color, bg: Color) {
        self.push(RenderCommand::HLine {
            x,
            y,
            width,
            ch,
            fg,
            bg,
        });
    }

    fn draw_border(
        &mut self,
        x: u16,
        y: u16,
        w: u16,
        h: u16,
        fg: Color,
        bg: Color,
        title: Option<&str>,
    ) {
        self.push(RenderCommand::DrawBorder {
            x,
            y,
            w,
            h,
            fg,
            bg,
            title: title.map(|t| t.to_string()),
        });
    }

    fn write_styled_line(
        &mut self,
        x: u16,
        y: u16,
        line: &StyledLine,
        default_fg: Color,
        default_bg: Color,
    ) {
        self.push(RenderCommand::WriteStyledLine {
            x,
            y,
            line: line.clone(),
            default_fg,
            default_bg,
        });
    }
}

pub fn apply_render_command(buffer: &mut ScreenBuffer, command: &RenderCommand) {
    match command {
        RenderCommand::SetCell { x, y, cell } => buffer.set(*x, *y, *cell),
        RenderCommand::FillRect { x, y, w, h, cell } => buffer.fill_rect(*x, *y, *w, *h, *cell),
        RenderCommand::WriteStr {
            x,
            y,
            text,
            fg,
            bg,
            bold,
            dim,
        } => buffer.write_str(*x, *y, text, *fg, *bg, *bold, *dim),
        RenderCommand::WriteStyled { x, y, text, style } => {
            buffer.write_styled(*x, *y, text, style)
        }
        RenderCommand::WriteStyledLine {
            x,
            y,
            line,
            default_fg,
            default_bg,
        } => buffer.write_styled_line(*x, *y, line, *default_fg, *default_bg),
        RenderCommand::HLine {
            x,
            y,
            width,
            ch,
            fg,
            bg,
        } => buffer.hline(*x, *y, *width, *ch, *fg, *bg),
        RenderCommand::DrawBorder {
            x,
            y,
            w,
            h,
            fg,
            bg,
            title,
        } => buffer.draw_border(*x, *y, *w, *h, *fg, *bg, title.as_deref()),
    }
}

pub fn render_commands_to_buffer(
    commands: &[RenderCommand],
    width: u16,
    height: u16,
) -> ScreenBuffer {
    let mut buffer = ScreenBuffer::new(width, height);
    for command in commands {
        apply_render_command(&mut buffer, command);
    }
    buffer
}
