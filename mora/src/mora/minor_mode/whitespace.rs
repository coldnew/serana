use super::MinorMode;

#[derive(Debug)]
pub struct WhitespaceMode;

impl MinorMode for WhitespaceMode {
    fn name(&self) -> &'static str {
        "whitespace"
    }
    fn modeline_indicator(&self) -> &'static str {
        "WS"
    }
    fn priority(&self) -> i32 {
        0
    }
}
