use super::MinorMode;
use crate::mora::keymap::KeyAction;

#[derive(Debug)]
pub struct ElectricPairMode;

impl MinorMode for ElectricPairMode {
    fn name(&self) -> &'static str { "electric-pair" }
    fn modeline_indicator(&self) -> &'static str { "EP" }
    fn priority(&self) -> i32 { -5 }

    fn on_insert_char(&self, ch: char) -> Option<KeyAction> {
        match ch {
            '(' => Some(KeyAction::InsertChar(')')),
            '[' => Some(KeyAction::InsertChar(']')),
            '{' => Some(KeyAction::InsertChar('}')),
            '"' => Some(KeyAction::InsertChar('"')),
            '\'' => Some(KeyAction::InsertChar('\'')),
            '`' => Some(KeyAction::InsertChar('`')),
            _ => None,
        }
    }
}
