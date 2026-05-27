use super::{MajorMode, MajorModeKind};

#[derive(Debug)]
pub struct FundamentalMode;

impl MajorMode for FundamentalMode {
    fn kind(&self) -> MajorModeKind {
        MajorModeKind::Fundamental
    }
    fn name(&self) -> &'static str {
        "Fundamental"
    }
}
