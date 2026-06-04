use crate::palette;
use display_protocol::{Border, BoxNode, Padding, Style, UiNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    Primary,
    Secondary,
    Outline,
    Ghost,
    Danger,
}

#[derive(Debug, Clone)]
pub struct Button {
    label: String,
    style: ButtonStyle,
    disabled: bool,
    pressed: bool,
    focused: bool,
    width: Option<u16>,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            style: ButtonStyle::Primary,
            disabled: false,
            pressed: false,
            focused: false,
            width: None,
        }
    }

    pub fn style(mut self, s: ButtonStyle) -> Self {
        self.style = s;
        self
    }
    pub fn primary(self) -> Self {
        self.style(ButtonStyle::Primary)
    }
    pub fn secondary(self) -> Self {
        self.style(ButtonStyle::Secondary)
    }
    pub fn outline(self) -> Self {
        self.style(ButtonStyle::Outline)
    }
    pub fn ghost(self) -> Self {
        self.style(ButtonStyle::Ghost)
    }
    pub fn danger(self) -> Self {
        self.style(ButtonStyle::Danger)
    }
    pub fn disabled(mut self, v: bool) -> Self {
        self.disabled = v;
        self
    }
    pub fn pressed(mut self, v: bool) -> Self {
        self.pressed = v;
        self
    }
    pub fn focused(mut self, v: bool) -> Self {
        self.focused = v;
        self
    }
    pub fn width(mut self, w: u16) -> Self {
        self.width = Some(w);
        self
    }

    pub fn build(self) -> UiNode {
        let (fg, bg, border_opt) = match self.style {
            ButtonStyle::Primary => {
                if self.disabled {
                    (palette::MUTED, palette::DARK, None)
                } else if self.pressed {
                    (palette::WHITE, palette::Color::new(37, 99, 235), None)
                } else {
                    (palette::WHITE, palette::PRIMARY, None)
                }
            }
            ButtonStyle::Secondary => {
                if self.disabled {
                    (palette::MUTED, palette::DARK, None)
                } else {
                    (palette::LIGHT, palette::Color::new(75, 85, 99), None)
                }
            }
            ButtonStyle::Outline => {
                if self.disabled {
                    (palette::MUTED, palette::BLACK, Some(palette::MUTED))
                } else if self.pressed {
                    (
                        palette::PRIMARY,
                        palette::Color::new(30, 66, 159),
                        Some(palette::PRIMARY),
                    )
                } else {
                    (palette::LIGHT, palette::BLACK, Some(palette::MUTED))
                }
            }
            ButtonStyle::Ghost => {
                if self.disabled {
                    (palette::MUTED, palette::BLACK, None)
                } else if self.pressed {
                    (palette::PRIMARY, palette::Color::new(30, 30, 40), None)
                } else {
                    (palette::LIGHT, palette::BLACK, None)
                }
            }
            ButtonStyle::Danger => {
                if self.disabled {
                    (palette::MUTED, palette::DARK, None)
                } else if self.pressed {
                    (palette::WHITE, palette::Color::new(185, 28, 28), None)
                } else {
                    (palette::WHITE, palette::DANGER, None)
                }
            }
        };

        let mut style = Style::default().fg(fg).bg(bg);
        if self.focused {
            style = style.reverse();
        }
        if self.disabled {
            style = style.dim();
        }

        let border = match border_opt {
            Some(c) => Border::all(Some(Style::default().fg(c))),
            None => Border::NONE,
        };

        UiNode::Box(BoxNode {
            children: vec![UiNode::text(&self.label).color(fg).bg(bg)],
            style,
            padding: Padding::new(0, 2, 0, 2),
            border,
            title: None,
            width: self.width,
            height: None,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
        })
    }
}

impl From<Button> for UiNode {
    fn from(btn: Button) -> Self {
        btn.build()
    }
}
