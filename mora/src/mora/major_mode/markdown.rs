use super::{BracketPair, MajorMode, MajorModeKind};

#[derive(Debug)]
pub struct MarkdownMode;

impl MajorMode for MarkdownMode {
    fn kind(&self) -> MajorModeKind { MajorModeKind::Markdown }
    fn name(&self) -> &'static str { "Markdown" }

    fn bracket_pairs(&self) -> Vec<BracketPair> {
        vec![
            BracketPair { open: '(', close: ')' },
            BracketPair { open: '{', close: '}' },
            BracketPair { open: '[', close: ']' },
        ]
    }

    fn quote_pairs(&self) -> Vec<char> {
        vec!['"', '`']
    }
}
