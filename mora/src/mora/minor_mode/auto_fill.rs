use super::MinorMode;

#[derive(Debug)]
pub struct AutoFillMode;

impl MinorMode for AutoFillMode {
    fn name(&self) -> &'static str { "auto-fill" }
    fn modeline_indicator(&self) -> &'static str { "Fill" }
    fn priority(&self) -> i32 { -10 }
}
