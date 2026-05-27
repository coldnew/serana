use super::MinorMode;

#[derive(Debug)]
pub struct LineNumbersMode;

impl MinorMode for LineNumbersMode {
    fn name(&self) -> &'static str {
        "line-numbers"
    }
    fn modeline_indicator(&self) -> &'static str {
        "LN"
    }
    fn priority(&self) -> i32 {
        0
    }
}
