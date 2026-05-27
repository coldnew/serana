use super::MinorMode;
use crate::mora::keymap::KeyAction;

#[derive(Debug)]
pub struct AggressiveIndentMode;

impl MinorMode for AggressiveIndentMode {
    fn name(&self) -> &'static str {
        "aggressive-indent"
    }
    fn modeline_indicator(&self) -> &'static str {
        "AI"
    }
    fn priority(&self) -> i32 {
        -15
    }

    fn on_insert_newline(&self) -> Option<KeyAction> {
        Some(KeyAction::IndentLine)
    }
}
