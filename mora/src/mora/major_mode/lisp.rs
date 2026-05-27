use super::{BracketPair, CommentStyle, MajorMode, MajorModeKind};

#[derive(Debug)]
pub struct LispMode;

impl MajorMode for LispMode {
    fn kind(&self) -> MajorModeKind {
        MajorModeKind::Lisp
    }
    fn name(&self) -> &'static str {
        "Lisp"
    }

    fn comment_style(&self) -> CommentStyle {
        CommentStyle {
            line_comment: ";",
            block_comment_start: Some("#|"),
            block_comment_end: Some("|#"),
        }
    }

    fn indent_width(&self) -> usize {
        2
    }

    fn bracket_pairs(&self) -> Vec<BracketPair> {
        vec![
            BracketPair {
                open: '(',
                close: ')',
            },
            BracketPair {
                open: '[',
                close: ']',
            },
        ]
    }

    fn quote_pairs(&self) -> Vec<char> {
        vec!['"']
    }
}
