use display_protocol::{InputNode, Style, UiNode};
use crate::palette;

/// A single-line text input field.
///
/// Works on both TUI and WGPU backends.
///
/// ```ignore
/// TextInput::new("hello").build()
/// TextInput::new("").placeholder("Type here...").focused(true).build()
/// TextInput::new("search").width(40).build()
/// ```
#[derive(Debug, Clone)]
pub struct TextInput {
    value: String,
    placeholder: String,
    cursor: u16,
    width: Option<u16>,
    focused: bool,
}

impl TextInput {
    pub fn new(value: impl Into<String>) -> Self {
        let v = value.into();
        let cursor = v.len() as u16;
        Self {
            value: v,
            placeholder: String::new(),
            cursor,
            width: None,
            focused: false,
        }
    }

    pub fn placeholder(mut self, p: impl Into<String>) -> Self { self.placeholder = p.into(); self }
    pub fn cursor_pos(mut self, pos: u16) -> Self { self.cursor = pos; self }
    pub fn width(mut self, w: u16) -> Self { self.width = Some(w); self }
    pub fn focused(mut self, f: bool) -> Self { self.focused = f; self }

    pub fn build(self) -> UiNode {
        let cursor_style = if self.focused {
            Style::default().reverse()
        } else {
            Style::default()
        };

        UiNode::Input(InputNode {
            value: self.value,
            placeholder: self.placeholder,
            cursor: self.cursor,
            style: Style::default().fg(palette::LIGHT),
            cursor_style,
            width: self.width,
            focused: self.focused,
        })
    }
}

impl From<TextInput> for UiNode {
    fn from(t: TextInput) -> Self { t.build() }
}
