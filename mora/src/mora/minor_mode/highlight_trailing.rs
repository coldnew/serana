use super::MinorMode;

#[derive(Debug)]
pub struct HighlightTrailingMode;

impl MinorMode for HighlightTrailingMode {
    fn name(&self) -> &'static str {
        "highlight-trailing"
    }
    fn modeline_indicator(&self) -> &'static str {
        "TW"
    }
    fn priority(&self) -> i32 {
        0
    }
}
