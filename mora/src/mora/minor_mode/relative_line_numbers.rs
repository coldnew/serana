use super::MinorMode;

#[derive(Debug)]
pub struct RelativeLineNumbersMode;

impl MinorMode for RelativeLineNumbersMode {
    fn name(&self) -> &'static str {
        "relative-line-numbers"
    }
    fn modeline_indicator(&self) -> &'static str {
        "RLN"
    }
    fn priority(&self) -> i32 {
        0
    }
}
