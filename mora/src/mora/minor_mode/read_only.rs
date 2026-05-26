use crossterm::event::KeyEvent;
use super::MinorMode;
use crate::mora::keymap::KeyAction;

#[derive(Debug)]
pub struct ReadOnlyMode;

impl MinorMode for ReadOnlyMode {
    fn name(&self) -> &'static str { "read-only" }
    fn modeline_indicator(&self) -> &'static str { "RO" }
    fn priority(&self) -> i32 { 100 }

    fn on_key(&self, key: KeyEvent) -> Option<KeyAction> {
        use crossterm::event::{KeyCode, KeyModifiers};
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c'))
            | (KeyModifiers::CONTROL, KeyCode::Char('g'))
            | (_, KeyCode::Esc) => None,
            (_, KeyCode::Char(_)) | (_, KeyCode::Enter) | (_, KeyCode::Backspace) | (_, KeyCode::Tab) => {
                Some(KeyAction::None)
            }
            _ => None,
        }
    }
}
