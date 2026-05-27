use super::display::style::{MoraColor, MoraStyle};

use serana_tree_sitter::highlight::highlight_document;
pub use serana_tree_sitter::highlight::{HighlightKind, HighlightToken};
use serana_tree_sitter::LanguageId;

pub fn style_for_kind(kind: HighlightKind) -> MoraStyle {
    match kind {
        HighlightKind::Normal => MoraStyle::new().fg(MoraColor::new(232, 236, 244)),
        HighlightKind::Keyword => MoraStyle::new()
            .fg(MoraColor::new(255, 100, 150))
            .bold(),
        HighlightKind::String => MoraStyle::new().fg(MoraColor::new(150, 220, 100)),
        HighlightKind::Comment => MoraStyle::new().fg(MoraColor::new(107, 114, 128)),
        HighlightKind::Number => MoraStyle::new().fg(MoraColor::new(255, 179, 71)),
        HighlightKind::Function => MoraStyle::new().fg(MoraColor::new(100, 200, 255)),
        HighlightKind::Type => MoraStyle::new().fg(MoraColor::new(200, 150, 255)),
        HighlightKind::Operator => MoraStyle::new().fg(MoraColor::new(232, 236, 244)),
        HighlightKind::Bracket => MoraStyle::new().fg(MoraColor::new(200, 200, 200)),
        HighlightKind::Property => MoraStyle::new().fg(MoraColor::new(232, 180, 100)),
        HighlightKind::Variable => MoraStyle::new().fg(MoraColor::new(232, 236, 244)),
        HighlightKind::Constant => MoraStyle::new().fg(MoraColor::new(255, 179, 71)),
        HighlightKind::Heading => MoraStyle::new()
            .fg(MoraColor::new(255, 200, 50))
            .bold(),
        HighlightKind::Bold => MoraStyle::new()
            .fg(MoraColor::new(232, 236, 244))
            .bold(),
        HighlightKind::Italic => MoraStyle::new()
            .fg(MoraColor::new(232, 236, 244))
            .italic(),
        HighlightKind::Link => MoraStyle::new()
            .fg(MoraColor::new(100, 200, 255))
            .underline(),
        HighlightKind::Code => MoraStyle::new()
            .fg(MoraColor::new(150, 220, 100))
            .bg(MoraColor::new(40, 44, 52)),
        HighlightKind::ListMarker => MoraStyle::new().fg(MoraColor::new(255, 100, 150)),
        HighlightKind::Blockquote => MoraStyle::new()
            .fg(MoraColor::new(107, 114, 128))
            .italic(),
        HighlightKind::HorizontalRule => MoraStyle::new().fg(MoraColor::new(107, 114, 128)),
        HighlightKind::Tag => MoraStyle::new().fg(MoraColor::new(255, 100, 150)),
        HighlightKind::Attribute => MoraStyle::new().fg(MoraColor::new(255, 179, 71)),
    }
}

pub trait SyntaxHighlighter: std::fmt::Debug {
    fn highlight_line(&self, line: &str, line_idx: usize) -> Vec<HighlightToken>;
}

struct TreeSitterHighlighter {
    line_tokens: Vec<Vec<HighlightToken>>,
}

impl std::fmt::Debug for TreeSitterHighlighter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TreeSitterHighlighter({} lines)", self.line_tokens.len())
    }
}

impl SyntaxHighlighter for TreeSitterHighlighter {
    fn highlight_line(&self, _line: &str, line_idx: usize) -> Vec<HighlightToken> {
        self.line_tokens.get(line_idx).cloned().unwrap_or_default()
    }
}

#[derive(Debug)]
struct RegexHighlighter {
    mode: String,
}

impl SyntaxHighlighter for RegexHighlighter {
    fn highlight_line(&self, line: &str, _line_idx: usize) -> Vec<HighlightToken> {
        match self.mode.as_str() {
            "Markdown" => highlight_markdown_regex(line),
            "Lisp" => highlight_lisp_regex(line),
            "Fundamental" | "Text" => highlight_generic_regex(line),
            _ => highlight_generic_regex(line),
        }
    }
}

fn highlight_markdown_regex(line: &str) -> Vec<HighlightToken> {
    let mut tokens = Vec::new();
    let trimmed = line.trim_start();

    if trimmed.starts_with("# ")
        || trimmed.starts_with("## ")
        || trimmed.starts_with("### ")
        || trimmed.starts_with("#### ")
        || trimmed.starts_with("##### ")
        || trimmed.starts_with("###### ")
    {
        tokens.push(HighlightToken {
            start: 0,
            end: line.len(),
            kind: HighlightKind::Heading,
        });
        return tokens;
    }
    if trimmed
        .chars()
        .all(|c| c == '-' || c == '*' || c == '_' || c == ' ')
        && trimmed.len() >= 3
    {
        tokens.push(HighlightToken {
            start: 0,
            end: line.len(),
            kind: HighlightKind::HorizontalRule,
        });
        return tokens;
    }
    if trimmed.starts_with("> ") {
        tokens.push(HighlightToken {
            start: 0,
            end: line.len(),
            kind: HighlightKind::Blockquote,
        });
        return tokens;
    }
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        let offset = line.len() - trimmed.len();
        tokens.push(HighlightToken {
            start: offset,
            end: offset + 2,
            kind: HighlightKind::ListMarker,
        });
    }

    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '`' {
            if let Some(end) = find_closing(&chars, i + 1, '`') {
                tokens.push(HighlightToken {
                    start: i,
                    end: end + 1,
                    kind: HighlightKind::Code,
                });
                i = end + 1;
                continue;
            }
        }
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(end) = find_double_closing(&chars, i + 2, '*') {
                tokens.push(HighlightToken {
                    start: i,
                    end: end + 2,
                    kind: HighlightKind::Bold,
                });
                i = end + 2;
                continue;
            }
        }
        if chars[i] == '*' && (i == 0 || chars[i - 1] != '*') {
            if let Some(end) = find_closing(&chars, i + 1, '*') {
                tokens.push(HighlightToken {
                    start: i,
                    end: end + 1,
                    kind: HighlightKind::Italic,
                });
                i = end + 1;
                continue;
            }
        }
        if chars[i] == '[' {
            if let Some(bracket_end) = find_closing(&chars, i + 1, ']') {
                if bracket_end + 1 < chars.len() && chars[bracket_end + 1] == '(' {
                    if let Some(paren_end) = find_closing(&chars, bracket_end + 2, ')') {
                        tokens.push(HighlightToken {
                            start: i,
                            end: paren_end + 1,
                            kind: HighlightKind::Link,
                        });
                        i = paren_end + 1;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    tokens
}

fn highlight_lisp_regex(line: &str) -> Vec<HighlightToken> {
    let mut tokens = Vec::new();
    let trimmed = line.trim_start();

    if trimmed.starts_with(';') {
        tokens.push(HighlightToken {
            start: 0,
            end: line.len(),
            kind: HighlightKind::Comment,
        });
        return tokens;
    }

    let keywords = [
        "defun",
        "defvar",
        "defmacro",
        "setq",
        "let",
        "if",
        "when",
        "unless",
        "cond",
        "lambda",
        "progn",
        "quote",
        "backquote",
        "require",
        "provide",
        "interactive",
    ];

    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '"' {
            if let Some(end) = find_closing(&chars, i + 1, '"') {
                tokens.push(HighlightToken {
                    start: i,
                    end: end + 1,
                    kind: HighlightKind::String,
                });
                i = end + 1;
                continue;
            }
        }
        if chars[i] == '(' || chars[i] == ')' {
            tokens.push(HighlightToken {
                start: i,
                end: i + 1,
                kind: HighlightKind::Bracket,
            });
            i += 1;
            continue;
        }
        if chars[i].is_alphabetic() || chars[i] == '-' || chars[i] == '_' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_alphanumeric() || chars[i] == '-' || chars[i] == '_')
            {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            if keywords.contains(&word.as_str()) {
                tokens.push(HighlightToken {
                    start,
                    end: i,
                    kind: HighlightKind::Keyword,
                });
            }
            continue;
        }
        i += 1;
    }
    tokens
}

fn highlight_generic_regex(line: &str) -> Vec<HighlightToken> {
    let mut tokens = Vec::new();
    let trimmed = line.trim_start();

    if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with(';') {
        tokens.push(HighlightToken {
            start: 0,
            end: line.len(),
            kind: HighlightKind::Comment,
        });
        return tokens;
    }

    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '"' || chars[i] == '\'' {
            let quote = chars[i];
            if let Some(end) = find_closing(&chars, i + 1, quote) {
                tokens.push(HighlightToken {
                    start: i,
                    end: end + 1,
                    kind: HighlightKind::String,
                });
                i = end + 1;
                continue;
            }
        }
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len()
                && (chars[i].is_ascii_digit() || chars[i] == '_' || chars[i] == '.')
            {
                i += 1;
            }
            tokens.push(HighlightToken {
                start,
                end: i,
                kind: HighlightKind::Number,
            });
            continue;
        }
        i += 1;
    }
    tokens
}

fn find_closing(chars: &[char], start: usize, target: char) -> Option<usize> {
    let mut i = start;
    while i < chars.len() {
        if chars[i] == target {
            return Some(i);
        }
        if chars[i] == '\\' {
            i += 1;
        }
        i += 1;
    }
    None
}

fn find_double_closing(chars: &[char], start: usize, target: char) -> Option<usize> {
    let mut i = start;
    while i + 1 < chars.len() {
        if chars[i] == target && chars[i + 1] == target {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn mode_to_lang_id(mode: &str) -> Option<LanguageId> {
    match mode {
        "Rust" => Some(LanguageId::Rust),
        "Python" => Some(LanguageId::Python),
        "JavaScript" | "TypeScript" => Some(LanguageId::JavaScript),
        "Go" => Some(LanguageId::Go),
        "Shell" => Some(LanguageId::Bash),
        "C" => Some(LanguageId::C),
        _ => None,
    }
}

pub fn create_highlighter(mode: &str, content: &str) -> Box<dyn SyntaxHighlighter> {
    if let Some(lang_id) = mode_to_lang_id(mode) {
        let line_tokens = highlight_document(lang_id, content);
        Box::new(TreeSitterHighlighter { line_tokens })
    } else {
        Box::new(RegexHighlighter {
            mode: mode.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_sitter_rust() {
        let h = create_highlighter("Rust", "fn main() {\n    // hello\n    let x = 42;\n}\n");
        let tokens = h.highlight_line("fn main() {", 0);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::Keyword));
        let tokens = h.highlight_line("    // hello", 1);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::Comment));
        let tokens = h.highlight_line("    let x = 42;", 2);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::Number));
    }

    #[test]
    fn test_tree_sitter_python() {
        let h = create_highlighter("Python", "def foo():\n    return 42\n");
        let tokens = h.highlight_line("def foo():", 0);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::Keyword));
    }

    #[test]
    fn test_tree_sitter_markdown() {
        let h = create_highlighter("Markdown", "# Hello\n\nSome text.\n");
        let tokens = h.highlight_line("# Hello", 0);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::Heading));
    }

    #[test]
    fn test_regex_lisp() {
        let h = create_highlighter("Lisp", "(defun foo ()\n  ; comment\n  42)\n");
        let tokens = h.highlight_line("(defun foo ()", 0);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::Keyword));
        let tokens = h.highlight_line("  ; comment", 1);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::Comment));
    }

    #[test]
    fn test_regex_fundamental() {
        let h = create_highlighter("Fundamental", "// comment\n\"string\"\n");
        let tokens = h.highlight_line("// comment", 0);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::Comment));
        let tokens = h.highlight_line("\"string\"", 1);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::String));
    }

    #[test]
    fn test_style_for_kind() {
        let style = style_for_kind(HighlightKind::Keyword);
        assert_eq!(style.fg, Some(MoraColor::new(255, 100, 150)));
    }
}
