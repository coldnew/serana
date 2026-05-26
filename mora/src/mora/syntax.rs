use ratatui::style::{Color, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightKind {
    Normal,
    Keyword,
    String,
    Comment,
    Number,
    Function,
    Type,
    Operator,
    Bracket,
    Heading,
    Bold,
    Italic,
    Link,
    Code,
    ListMarker,
    Blockquote,
    HorizontalRule,
}

impl HighlightKind {
    pub fn to_style(self) -> Style {
        match self {
            Self::Normal => Style::new().fg(Color::Rgb(232, 236, 244)),
            Self::Keyword => Style::new().fg(Color::Rgb(255, 100, 150)).add_modifier(ratatui::style::Modifier::BOLD),
            Self::String => Style::new().fg(Color::Rgb(150, 220, 100)),
            Self::Comment => Style::new().fg(Color::Rgb(107, 114, 128)),
            Self::Number => Style::new().fg(Color::Rgb(255, 179, 71)),
            Self::Function => Style::new().fg(Color::Rgb(100, 200, 255)),
            Self::Type => Style::new().fg(Color::Rgb(200, 150, 255)),
            Self::Operator => Style::new().fg(Color::Rgb(232, 236, 244)),
            Self::Bracket => Style::new().fg(Color::Rgb(200, 200, 200)),
            Self::Heading => Style::new().fg(Color::Rgb(255, 200, 50)).add_modifier(ratatui::style::Modifier::BOLD),
            Self::Bold => Style::new().fg(Color::Rgb(232, 236, 244)).add_modifier(ratatui::style::Modifier::BOLD),
            Self::Italic => Style::new().fg(Color::Rgb(232, 236, 244)).add_modifier(ratatui::style::Modifier::ITALIC),
            Self::Link => Style::new().fg(Color::Rgb(100, 200, 255)).add_modifier(ratatui::style::Modifier::UNDERLINED),
            Self::Code => Style::new().fg(Color::Rgb(150, 220, 100)).bg(Color::Rgb(40, 44, 52)),
            Self::ListMarker => Style::new().fg(Color::Rgb(255, 100, 150)),
            Self::Blockquote => Style::new().fg(Color::Rgb(107, 114, 128)).add_modifier(ratatui::style::Modifier::ITALIC),
            Self::HorizontalRule => Style::new().fg(Color::Rgb(107, 114, 128)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HighlightToken {
    pub start: usize,
    pub end: usize,
    pub kind: HighlightKind,
}

pub trait SyntaxHighlighter: std::fmt::Debug {
    fn highlight_line(&self, line: &str, line_idx: usize) -> Vec<HighlightToken>;
}

#[derive(Debug)]
pub struct MarkdownHighlighter;

impl SyntaxHighlighter for MarkdownHighlighter {
    fn highlight_line(&self, line: &str, _line_idx: usize) -> Vec<HighlightToken> {
        let mut tokens = Vec::new();
        let trimmed = line.trim_start();

        // Headings
        if trimmed.starts_with("# ") {
            tokens.push(HighlightToken { start: 0, end: line.len(), kind: HighlightKind::Heading });
            return tokens;
        }
        if trimmed.starts_with("## ") {
            tokens.push(HighlightToken { start: 0, end: line.len(), kind: HighlightKind::Heading });
            return tokens;
        }
        if trimmed.starts_with("### ") {
            tokens.push(HighlightToken { start: 0, end: line.len(), kind: HighlightKind::Heading });
            return tokens;
        }
        if trimmed.starts_with("#### ") {
            tokens.push(HighlightToken { start: 0, end: line.len(), kind: HighlightKind::Heading });
            return tokens;
        }

        // Horizontal rule
        if trimmed.chars().all(|c| c == '-' || c == '*' || c == '_' || c == ' ') && trimmed.len() >= 3 {
            tokens.push(HighlightToken { start: 0, end: line.len(), kind: HighlightKind::HorizontalRule });
            return tokens;
        }

        // Blockquote
        if trimmed.starts_with("> ") {
            tokens.push(HighlightToken { start: 0, end: line.len(), kind: HighlightKind::Blockquote });
            return tokens;
        }

        // List markers
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
            let offset = line.len() - trimmed.len();
            tokens.push(HighlightToken { start: offset, end: offset + 2, kind: HighlightKind::ListMarker });
        }

        // Numbered list
        if let Some(pos) = trimmed.find(|c: char| c == '.' || c == ')') {
            if pos > 0 && trimmed[..pos].chars().all(|c| c.is_ascii_digit()) {
                let offset = line.len() - trimmed.len();
                tokens.push(HighlightToken { start: offset, end: offset + pos + 1, kind: HighlightKind::ListMarker });
            }
        }

        // Inline formatting
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            // Inline code
            if chars[i] == '`' {
                if let Some(end) = find_closing(&chars, i + 1, '`') {
                    tokens.push(HighlightToken { start: i, end: end + 1, kind: HighlightKind::Code });
                    i = end + 1;
                    continue;
                }
            }

            // Bold **text**
            if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
                if let Some(end) = find_double_closing(&chars, i + 2, '*') {
                    tokens.push(HighlightToken { start: i, end: end + 2, kind: HighlightKind::Bold });
                    i = end + 2;
                    continue;
                }
            }

            // Italic *text*
            if chars[i] == '*' && (i == 0 || chars[i - 1] != '*') {
                if let Some(end) = find_closing(&chars, i + 1, '*') {
                    tokens.push(HighlightToken { start: i, end: end + 1, kind: HighlightKind::Italic });
                    i = end + 1;
                    continue;
                }
            }

            // Link [text](url)
            if chars[i] == '[' {
                if let Some(bracket_end) = find_closing(&chars, i + 1, ']') {
                    if bracket_end + 1 < chars.len() && chars[bracket_end + 1] == '(' {
                        if let Some(paren_end) = find_closing(&chars, bracket_end + 2, ')') {
                            tokens.push(HighlightToken { start: i, end: paren_end + 1, kind: HighlightKind::Link });
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
}

#[derive(Debug)]
pub struct RustHighlighter;

impl SyntaxHighlighter for RustHighlighter {
    fn highlight_line(&self, line: &str, _line_idx: usize) -> Vec<HighlightToken> {
        let mut tokens = Vec::new();
        let trimmed = line.trim_start();
        let offset = line.len() - trimmed.len();

        // Line comment
        if let Some(pos) = trimmed.find("//") {
            tokens.push(HighlightToken {
                start: offset + pos,
                end: line.len(),
                kind: HighlightKind::Comment,
            });
            return tokens;
        }

        // Keywords
        let keywords = ["fn", "let", "mut", "const", "static", "struct", "enum", "impl",
            "trait", "pub", "use", "mod", "crate", "self", "super", "where", "for", "in",
            "loop", "while", "if", "else", "match", "return", "break", "continue", "async",
            "await", "move", "ref", "type", "as", "unsafe", "extern"];

        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            // String literals
            if chars[i] == '"' {
                if let Some(end) = find_closing(&chars, i + 1, '"') {
                    tokens.push(HighlightToken { start: i, end: end + 1, kind: HighlightKind::String });
                    i = end + 1;
                    continue;
                }
            }

            // Numbers
            if chars[i].is_ascii_digit() {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '_' || chars[i] == '.' || chars[i] == 'x' || chars[i] == 'b') {
                    i += 1;
                }
                tokens.push(HighlightToken { start, end: i, kind: HighlightKind::Number });
                continue;
            }

            // Words
            if chars[i].is_alphabetic() || chars[i] == '_' {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                let kind = if keywords.contains(&word.as_str()) {
                    HighlightKind::Keyword
                } else if i < chars.len() && chars[i] == '(' {
                    HighlightKind::Function
                } else if word.chars().next().map_or(false, |c| c.is_uppercase()) {
                    HighlightKind::Type
                } else {
                    HighlightKind::Normal
                };
                if kind != HighlightKind::Normal {
                    tokens.push(HighlightToken { start, end: i, kind });
                }
                continue;
            }

            // Brackets
            if "(){}[]".contains(chars[i]) {
                tokens.push(HighlightToken { start: i, end: i + 1, kind: HighlightKind::Bracket });
            }

            i += 1;
        }

        tokens
    }
}

#[derive(Debug)]
pub struct PythonHighlighter;

impl SyntaxHighlighter for PythonHighlighter {
    fn highlight_line(&self, line: &str, _line_idx: usize) -> Vec<HighlightToken> {
        let mut tokens = Vec::new();
        let trimmed = line.trim_start();
        let offset = line.len() - trimmed.len();

        // Line comment
        if let Some(pos) = trimmed.find('#') {
            tokens.push(HighlightToken {
                start: offset + pos,
                end: line.len(),
                kind: HighlightKind::Comment,
            });
            return tokens;
        }

        let keywords = ["def", "class", "if", "elif", "else", "for", "while", "return",
            "import", "from", "as", "try", "except", "finally", "raise", "with", "yield",
            "lambda", "pass", "break", "continue", "and", "or", "not", "in", "is", "None",
            "True", "False", "self", "async", "await"];

        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            // String literals
            if chars[i] == '"' || chars[i] == '\'' {
                let quote = chars[i];
                if let Some(end) = find_closing(&chars, i + 1, quote) {
                    tokens.push(HighlightToken { start: i, end: end + 1, kind: HighlightKind::String });
                    i = end + 1;
                    continue;
                }
            }

            // Numbers
            if chars[i].is_ascii_digit() {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '_' || chars[i] == '.') {
                    i += 1;
                }
                tokens.push(HighlightToken { start, end: i, kind: HighlightKind::Number });
                continue;
            }

            // Words
            if chars[i].is_alphabetic() || chars[i] == '_' {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                let kind = if keywords.contains(&word.as_str()) {
                    HighlightKind::Keyword
                } else if i < chars.len() && chars[i] == '(' {
                    HighlightKind::Function
                } else {
                    HighlightKind::Normal
                };
                if kind != HighlightKind::Normal {
                    tokens.push(HighlightToken { start, end: i, kind });
                }
                continue;
            }

            i += 1;
        }

        tokens
    }
}

#[derive(Debug)]
pub struct ShellHighlighter;

impl SyntaxHighlighter for ShellHighlighter {
    fn highlight_line(&self, line: &str, _line_idx: usize) -> Vec<HighlightToken> {
        let mut tokens = Vec::new();
        let trimmed = line.trim_start();
        let _offset = line.len() - trimmed.len();

        // Line comment
        if trimmed.starts_with('#') {
            tokens.push(HighlightToken { start: 0, end: line.len(), kind: HighlightKind::Comment });
            return tokens;
        }

        // Inline comment
        if let Some(pos) = find_unquoted_hash(line) {
            tokens.push(HighlightToken { start: pos, end: line.len(), kind: HighlightKind::Comment });
            return tokens;
        }

        let keywords = ["if", "then", "else", "elif", "fi", "for", "while", "do", "done",
            "case", "esac", "function", "return", "local", "export", "source", "alias",
            "unset", "shift", "exit", "echo", "printf"];

        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            // String literals
            if chars[i] == '"' || chars[i] == '\'' {
                let quote = chars[i];
                if let Some(end) = find_closing(&chars, i + 1, quote) {
                    tokens.push(HighlightToken { start: i, end: end + 1, kind: HighlightKind::String });
                    i = end + 1;
                    continue;
                }
            }

            // Variables $VAR
            if chars[i] == '$' {
                let start = i;
                i += 1;
                if i < chars.len() && chars[i] == '{' {
                    while i < chars.len() && chars[i] != '}' {
                        i += 1;
                    }
                    if i < chars.len() { i += 1; }
                } else {
                    while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                        i += 1;
                    }
                }
                tokens.push(HighlightToken { start, end: i, kind: HighlightKind::Type });
                continue;
            }

            // Words
            if chars[i].is_alphabetic() || chars[i] == '_' {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                let kind = if keywords.contains(&word.as_str()) {
                    HighlightKind::Keyword
                } else if i < chars.len() && chars[i] == '(' {
                    HighlightKind::Function
                } else {
                    HighlightKind::Normal
                };
                if kind != HighlightKind::Normal {
                    tokens.push(HighlightToken { start, end: i, kind });
                }
                continue;
            }

            i += 1;
        }

        tokens
    }
}

#[derive(Debug)]
pub struct GenericHighlighter;

impl SyntaxHighlighter for GenericHighlighter {
    fn highlight_line(&self, line: &str, _line_idx: usize) -> Vec<HighlightToken> {
        let mut tokens = Vec::new();
        let trimmed = line.trim_start();
        let _offset = line.len() - trimmed.len();

        // Try common comment styles
        if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with(';') {
            tokens.push(HighlightToken { start: 0, end: line.len(), kind: HighlightKind::Comment });
            return tokens;
        }

        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            // String literals
            if chars[i] == '"' || chars[i] == '\'' {
                let quote = chars[i];
                if let Some(end) = find_closing(&chars, i + 1, quote) {
                    tokens.push(HighlightToken { start: i, end: end + 1, kind: HighlightKind::String });
                    i = end + 1;
                    continue;
                }
            }

            // Numbers
            if chars[i].is_ascii_digit() {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '_' || chars[i] == '.') {
                    i += 1;
                }
                tokens.push(HighlightToken { start, end: i, kind: HighlightKind::Number });
                continue;
            }

            i += 1;
        }

        tokens
    }
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

fn find_unquoted_hash(line: &str) -> Option<usize> {
    let chars: Vec<char> = line.chars().collect();
    let mut in_single = false;
    let mut in_double = false;
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '#' if !in_single && !in_double => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

pub fn create_highlighter(mode: &str) -> Box<dyn SyntaxHighlighter> {
    match mode {
        "Markdown" => Box::new(MarkdownHighlighter),
        "Rust" => Box::new(RustHighlighter),
        "Python" => Box::new(PythonHighlighter),
        "Shell" => Box::new(ShellHighlighter),
        _ => Box::new(GenericHighlighter),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_heading() {
        let h = MarkdownHighlighter;
        let tokens = h.highlight_line("# Hello", 0);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, HighlightKind::Heading);
    }

    #[test]
    fn test_markdown_bold() {
        let h = MarkdownHighlighter;
        let tokens = h.highlight_line("some **bold** text", 0);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::Bold));
    }

    #[test]
    fn test_markdown_code() {
        let h = MarkdownHighlighter;
        let tokens = h.highlight_line("use `code` here", 0);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::Code));
    }

    #[test]
    fn test_markdown_link() {
        let h = MarkdownHighlighter;
        let tokens = h.highlight_line("[link](url)", 0);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::Link));
    }

    #[test]
    fn test_markdown_list() {
        let h = MarkdownHighlighter;
        let tokens = h.highlight_line("- item", 0);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::ListMarker));
    }

    #[test]
    fn test_markdown_blockquote() {
        let h = MarkdownHighlighter;
        let tokens = h.highlight_line("> quote", 0);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::Blockquote));
    }

    #[test]
    fn test_markdown_hr() {
        let h = MarkdownHighlighter;
        let tokens = h.highlight_line("---", 0);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::HorizontalRule));
    }

    #[test]
    fn test_rust_comment() {
        let h = RustHighlighter;
        let tokens = h.highlight_line("// comment", 0);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, HighlightKind::Comment);
    }

    #[test]
    fn test_rust_keyword() {
        let h = RustHighlighter;
        let tokens = h.highlight_line("fn main() {", 0);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::Keyword));
    }

    #[test]
    fn test_rust_string() {
        let h = RustHighlighter;
        let tokens = h.highlight_line("let s = \"hello\";", 0);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::String));
    }

    #[test]
    fn test_rust_number() {
        let h = RustHighlighter;
        let tokens = h.highlight_line("let x = 42;", 0);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::Number));
    }

    #[test]
    fn test_python_comment() {
        let h = PythonHighlighter;
        let tokens = h.highlight_line("# comment", 0);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, HighlightKind::Comment);
    }

    #[test]
    fn test_python_keyword() {
        let h = PythonHighlighter;
        let tokens = h.highlight_line("def foo():", 0);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::Keyword));
    }

    #[test]
    fn test_shell_variable() {
        let h = ShellHighlighter;
        let tokens = h.highlight_line("echo $HOME", 0);
        assert!(tokens.iter().any(|t| t.kind == HighlightKind::Type));
    }

    #[test]
    fn test_generic_comment() {
        let h = GenericHighlighter;
        let tokens = h.highlight_line("// comment", 0);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, HighlightKind::Comment);
    }

    #[test]
    fn test_create_highlighter() {
        assert!(create_highlighter("Markdown").highlight_line("# test", 0).len() > 0);
        assert!(create_highlighter("Rust").highlight_line("// test", 0).len() > 0);
        assert!(create_highlighter("Python").highlight_line("# test", 0).len() > 0);
        assert!(create_highlighter("Shell").highlight_line("# test", 0).len() > 0);
        assert!(create_highlighter("Unknown").highlight_line("// test", 0).len() > 0);
    }
}
