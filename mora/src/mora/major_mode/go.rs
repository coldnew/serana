use super::{CommentStyle, MajorMode, MajorModeKind};

#[derive(Debug)]
pub struct GoMode;

impl MajorMode for GoMode {
    fn kind(&self) -> MajorModeKind {
        MajorModeKind::Go
    }
    fn name(&self) -> &'static str {
        "Go"
    }

    fn comment_style(&self) -> CommentStyle {
        CommentStyle {
            line_comment: "//",
            block_comment_start: Some("/*"),
            block_comment_end: Some("*/"),
        }
    }

    fn indent_width(&self) -> usize {
        1
    }
    fn use_tabs(&self) -> bool {
        true
    }

    fn auto_indent(&self, prev_line: &str) -> String {
        let trimmed = prev_line.trim_end();
        let base: String = prev_line
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();
        if trimmed.ends_with('{') || trimmed.ends_with('(') {
            format!("{}\t", base)
        } else {
            base
        }
    }
}
