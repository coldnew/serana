#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoraColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl MoraColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoraStyle {
    pub fg: Option<MoraColor>,
    pub bg: Option<MoraColor>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub dim: bool,
    pub reverse: bool,
    pub blink: bool,
}

impl Default for MoraStyle {
    fn default() -> Self {
        Self {
            fg: None,
            bg: None,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            dim: false,
            reverse: false,
            blink: false,
        }
    }
}

impl MoraStyle {
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            dim: false,
            reverse: false,
            blink: false,
        }
    }

    pub const fn fg(mut self, color: MoraColor) -> Self {
        self.fg = Some(color);
        self
    }

    pub const fn bg(mut self, color: MoraColor) -> Self {
        self.bg = Some(color);
        self
    }

    pub const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub const fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    pub const fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    pub const fn dim(mut self) -> Self {
        self.dim = true;
        self
    }

    pub const fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    pub const fn blink(mut self) -> Self {
        self.blink = true;
        self
    }
}

impl From<MoraColor> for ratatui::style::Color {
    fn from(c: MoraColor) -> Self {
        ratatui::style::Color::Rgb(c.r, c.g, c.b)
    }
}

impl From<ratatui::style::Color> for MoraColor {
    fn from(c: ratatui::style::Color) -> Self {
        match c {
            ratatui::style::Color::Rgb(r, g, b) => MoraColor::new(r, g, b),
            ratatui::style::Color::Black => MoraColor::new(0, 0, 0),
            ratatui::style::Color::Red => MoraColor::new(205, 49, 49),
            ratatui::style::Color::Green => MoraColor::new(13, 188, 121),
            ratatui::style::Color::Yellow => MoraColor::new(229, 229, 16),
            ratatui::style::Color::Blue => MoraColor::new(36, 114, 200),
            ratatui::style::Color::Magenta => MoraColor::new(188, 63, 188),
            ratatui::style::Color::Cyan => MoraColor::new(17, 168, 205),
            ratatui::style::Color::Gray => MoraColor::new(229, 229, 229),
            ratatui::style::Color::DarkGray => MoraColor::new(97, 97, 97),
            ratatui::style::Color::LightRed => MoraColor::new(241, 76, 76),
            ratatui::style::Color::LightGreen => MoraColor::new(35, 209, 139),
            ratatui::style::Color::LightYellow => MoraColor::new(245, 245, 67),
            ratatui::style::Color::LightBlue => MoraColor::new(72, 145, 215),
            ratatui::style::Color::LightMagenta => MoraColor::new(214, 112, 214),
            ratatui::style::Color::LightCyan => MoraColor::new(41, 184, 215),
            ratatui::style::Color::White => MoraColor::new(255, 255, 255),
            _ => MoraColor::new(229, 229, 229),
        }
    }
}

impl From<MoraStyle> for ratatui::style::Style {
    fn from(s: MoraStyle) -> Self {
        let mut style = ratatui::style::Style::default();
        if let Some(fg) = s.fg {
            style = style.fg(fg.into());
        }
        if let Some(bg) = s.bg {
            style = style.bg(bg.into());
        }
        let mut mods = ratatui::style::Modifier::empty();
        if s.bold {
            mods |= ratatui::style::Modifier::BOLD;
        }
        if s.italic {
            mods |= ratatui::style::Modifier::ITALIC;
        }
        if s.underline {
            mods |= ratatui::style::Modifier::UNDERLINED;
        }
        if s.dim {
            mods |= ratatui::style::Modifier::DIM;
        }
        if s.reverse {
            mods |= ratatui::style::Modifier::REVERSED;
        }
        if s.blink {
            mods |= ratatui::style::Modifier::SLOW_BLINK;
        }
        if s.strikethrough {
            mods |= ratatui::style::Modifier::CROSSED_OUT;
        }
        if !mods.is_empty() {
            style = style.add_modifier(mods);
        }
        style
    }
}

impl From<ratatui::style::Style> for MoraStyle {
    fn from(s: ratatui::style::Style) -> Self {
        MoraStyle {
            fg: s.fg.map(MoraColor::from),
            bg: s.bg.map(MoraColor::from),
            bold: s.add_modifier.contains(ratatui::style::Modifier::BOLD),
            italic: s.add_modifier.contains(ratatui::style::Modifier::ITALIC),
            underline: s.add_modifier.contains(ratatui::style::Modifier::UNDERLINED),
            strikethrough: s.add_modifier.contains(ratatui::style::Modifier::CROSSED_OUT),
            dim: s.add_modifier.contains(ratatui::style::Modifier::DIM),
            reverse: s.add_modifier.contains(ratatui::style::Modifier::REVERSED),
            blink: s.add_modifier.contains(ratatui::style::Modifier::SLOW_BLINK),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StyledSpan {
    pub text: String,
    pub style: MoraStyle,
}

impl StyledSpan {
    pub fn new(text: impl Into<String>, style: MoraStyle) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }

    pub fn width(&self) -> usize {
        self.text.chars().count()
    }
}

#[derive(Debug, Clone)]
pub struct StyledLine {
    pub spans: Vec<StyledSpan>,
}

impl StyledLine {
    pub fn new(spans: Vec<StyledSpan>) -> Self {
        Self { spans }
    }

    pub fn width(&self) -> usize {
        self.spans.iter().map(|s| s.width()).sum()
    }
}

impl From<StyledLine> for ratatui::text::Line<'static> {
    fn from(line: StyledLine) -> Self {
        let spans: Vec<ratatui::text::Span<'static>> = line
            .spans
            .into_iter()
            .map(|s| ratatui::text::Span::styled(s.text, ratatui::style::Style::from(s.style)))
            .collect();
        ratatui::text::Line::from(spans)
    }
}
