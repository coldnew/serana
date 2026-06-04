use crate::{Editor, TextInput};

pub fn input(value: impl Into<String>) -> TextInput {
    TextInput::new(value)
}

pub fn editor(lines: Vec<String>) -> Editor {
    Editor::from_lines(lines)
}
