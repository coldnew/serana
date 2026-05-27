use super::MinorMode;
use crate::mora::display::event::{MoraKeyCode, MoraKeyEvent, MoraKeyModifiers};
use crate::mora::keymap::KeyAction;

#[derive(Debug)]
pub struct ReadOnlyMode;

impl MinorMode for ReadOnlyMode {
    fn name(&self) -> &'static str {
        "read-only"
    }
    fn modeline_indicator(&self) -> &'static str {
        "RO"
    }
    fn priority(&self) -> i32 {
        100
    }

    fn on_key(&self, key: MoraKeyEvent) -> Option<KeyAction> {
        match (key.modifiers, key.code) {
            (MoraKeyModifiers::CTRL, MoraKeyCode::Char('c'))
            | (MoraKeyModifiers::CTRL, MoraKeyCode::Char('g'))
            | (_, MoraKeyCode::Esc) => None,
            (_, MoraKeyCode::Char(_))
            | (_, MoraKeyCode::Enter)
            | (_, MoraKeyCode::Backspace)
            | (_, MoraKeyCode::Tab) => Some(KeyAction::None),
            _ => None,
        }
    }
}
