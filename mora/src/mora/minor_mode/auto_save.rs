use super::MinorMode;

#[derive(Debug)]
pub struct AutoSaveMode;

impl MinorMode for AutoSaveMode {
    fn name(&self) -> &'static str { "auto-save" }
    fn modeline_indicator(&self) -> &'static str { "AS" }
    fn priority(&self) -> i32 { -20 }
}
