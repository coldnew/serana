use super::{MajorMode, MajorModeKind};

#[derive(Debug)]
pub struct TextMode;

impl MajorMode for TextMode {
    fn kind(&self) -> MajorModeKind {
        MajorModeKind::Text
    }
    fn name(&self) -> &'static str {
        "Text"
    }
}
