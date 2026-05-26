use super::{CommentStyle, MajorMode, MajorModeKind};

#[derive(Debug)]
pub struct ShellMode;

impl MajorMode for ShellMode {
    fn kind(&self) -> MajorModeKind { MajorModeKind::Shell }
    fn name(&self) -> &'static str { "Shell" }

    fn comment_style(&self) -> CommentStyle {
        CommentStyle {
            line_comment: "#",
            block_comment_start: None,
            block_comment_end: None,
        }
    }

    fn indent_width(&self) -> usize { 2 }
}
