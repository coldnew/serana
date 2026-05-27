use super::{BracketPair, CommentStyle, MajorMode, MajorModeKind};

#[derive(Debug)]
pub struct RustMode;

impl MajorMode for RustMode {
    fn kind(&self) -> MajorModeKind {
        MajorModeKind::Rust
    }
    fn name(&self) -> &'static str {
        "Rust"
    }

    fn comment_style(&self) -> CommentStyle {
        CommentStyle {
            line_comment: "//",
            block_comment_start: Some("/*"),
            block_comment_end: Some("*/"),
        }
    }

    fn indent_width(&self) -> usize {
        4
    }

    fn bracket_pairs(&self) -> Vec<BracketPair> {
        vec![
            BracketPair {
                open: '(',
                close: ')',
            },
            BracketPair {
                open: '{',
                close: '}',
            },
            BracketPair {
                open: '[',
                close: ']',
            },
            BracketPair {
                open: '<',
                close: '>',
            },
        ]
    }

    fn auto_indent(&self, prev_line: &str) -> String {
        let trimmed = prev_line.trim_end();
        let base: String = prev_line
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();
        if trimmed.ends_with('{') || trimmed.ends_with('(') || trimmed.ends_with('[') {
            format!("{}    ", base)
        } else {
            base
        }
    }
}
