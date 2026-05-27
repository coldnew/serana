use super::MinorMode;
use crate::mora::keymap::KeyAction;

#[derive(Debug)]
pub struct AutoIndentMode;

impl MinorMode for AutoIndentMode {
    fn name(&self) -> &'static str {
        "auto-indent"
    }
    fn modeline_indicator(&self) -> &'static str {
        "Ind"
    }
    fn priority(&self) -> i32 {
        -15
    }

    fn on_insert_newline(&self) -> Option<KeyAction> {
        Some(KeyAction::IndentLine)
    }
}
