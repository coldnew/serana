use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MajorModeKind {
    Fundamental,
    Text,
    Rust,
    Markdown,
    Python,
    JavaScript,
    Go,
    Shell,
    C,
    Lisp,
}

impl MajorModeKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Fundamental => "Fundamental",
            Self::Text => "Text",
            Self::Rust => "Rust",
            Self::Markdown => "Markdown",
            Self::Python => "Python",
            Self::JavaScript => "JavaScript",
            Self::Go => "Go",
            Self::Shell => "Shell",
            Self::C => "C",
            Self::Lisp => "Lisp",
        }
    }

    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "rs" => Self::Rust,
            "md" | "markdown" => Self::Markdown,
            "py" | "pyw" => Self::Python,
            "js" | "mjs" | "cjs" | "ts" | "tsx" | "jsx" => Self::JavaScript,
            "go" => Self::Go,
            "sh" | "bash" | "zsh" | "fish" => Self::Shell,
            "c" | "h" | "cpp" | "hpp" | "cc" | "cxx" => Self::C,
            "el" | "cl" | "lisp" | "clj" | "cljs" | "scm" => Self::Lisp,
            "txt" => Self::Text,
            _ => Self::Fundamental,
        }
    }

    pub fn from_filename(name: &str) -> Self {
        match name {
            "Makefile" | "makefile" | "GNUmakefile" => Self::Shell,
            "CMakeLists.txt" => Self::C,
            "Cargo.toml" | "Cargo.lock" => Self::Rust,
            "go.mod" | "go.sum" => Self::Go,
            "package.json" | "tsconfig.json" => Self::JavaScript,
            _ => Self::Fundamental,
        }
    }

    pub fn detect(path: &Path) -> Self {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            let from_name = Self::from_filename(name);
            if from_name != Self::Fundamental {
                return from_name;
            }
        }
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            Self::from_extension(ext)
        } else {
            Self::Fundamental
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommentStyle {
    pub line_comment: &'static str,
    pub block_comment_start: Option<&'static str>,
    pub block_comment_end: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct BracketPair {
    pub open: char,
    pub close: char,
}

pub trait MajorMode: std::fmt::Debug {
    fn kind(&self) -> MajorModeKind;
    fn name(&self) -> &'static str;

    fn comment_style(&self) -> CommentStyle {
        CommentStyle {
            line_comment: "",
            block_comment_start: None,
            block_comment_end: None,
        }
    }

    fn indent_width(&self) -> usize {
        4
    }

    fn use_tabs(&self) -> bool {
        false
    }

    fn bracket_pairs(&self) -> Vec<BracketPair> {
        vec![
            BracketPair { open: '(', close: ')' },
            BracketPair { open: '{', close: '}' },
            BracketPair { open: '[', close: ']' },
        ]
    }

    fn quote_pairs(&self) -> Vec<char> {
        vec!['"', '\'']
    }

    fn is_word_char(&self, ch: char) -> bool {
        ch.is_alphanumeric() || ch == '_'
    }

    fn auto_indent(&self, prev_line: &str) -> String {
        let indent: String = prev_line
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();
        indent
    }
}

#[derive(Debug)]
pub struct FundamentalMode;

impl MajorMode for FundamentalMode {
    fn kind(&self) -> MajorModeKind { MajorModeKind::Fundamental }
    fn name(&self) -> &'static str { "Fundamental" }
}

#[derive(Debug)]
pub struct TextMode;

impl MajorMode for TextMode {
    fn kind(&self) -> MajorModeKind { MajorModeKind::Text }
    fn name(&self) -> &'static str { "Text" }
}

#[derive(Debug)]
pub struct RustMode;

impl MajorMode for RustMode {
    fn kind(&self) -> MajorModeKind { MajorModeKind::Rust }
    fn name(&self) -> &'static str { "Rust" }

    fn comment_style(&self) -> CommentStyle {
        CommentStyle {
            line_comment: "//",
            block_comment_start: Some("/*"),
            block_comment_end: Some("*/"),
        }
    }

    fn indent_width(&self) -> usize { 4 }

    fn bracket_pairs(&self) -> Vec<BracketPair> {
        vec![
            BracketPair { open: '(', close: ')' },
            BracketPair { open: '{', close: '}' },
            BracketPair { open: '[', close: ']' },
            BracketPair { open: '<', close: '>' },
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

#[derive(Debug)]
pub struct PythonMode;

impl MajorMode for PythonMode {
    fn kind(&self) -> MajorModeKind { MajorModeKind::Python }
    fn name(&self) -> &'static str { "Python" }

    fn comment_style(&self) -> CommentStyle {
        CommentStyle {
            line_comment: "#",
            block_comment_start: Some("\"\"\""),
            block_comment_end: Some("\"\"\""),
        }
    }

    fn indent_width(&self) -> usize { 4 }

    fn bracket_pairs(&self) -> Vec<BracketPair> {
        vec![
            BracketPair { open: '(', close: ')' },
            BracketPair { open: '{', close: '}' },
            BracketPair { open: '[', close: ']' },
        ]
    }

    fn auto_indent(&self, prev_line: &str) -> String {
        let trimmed = prev_line.trim_end();
        let base: String = prev_line
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();
        if trimmed.ends_with(':') || trimmed.ends_with('(') || trimmed.ends_with('[') || trimmed.ends_with('{') {
            format!("{}    ", base)
        } else {
            base
        }
    }
}

#[derive(Debug)]
pub struct JavaScriptMode;

impl MajorMode for JavaScriptMode {
    fn kind(&self) -> MajorModeKind { MajorModeKind::JavaScript }
    fn name(&self) -> &'static str { "JavaScript" }

    fn comment_style(&self) -> CommentStyle {
        CommentStyle {
            line_comment: "//",
            block_comment_start: Some("/*"),
            block_comment_end: Some("*/"),
        }
    }

    fn indent_width(&self) -> usize { 2 }

    fn auto_indent(&self, prev_line: &str) -> String {
        let trimmed = prev_line.trim_end();
        let base: String = prev_line
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();
        if trimmed.ends_with('{') || trimmed.ends_with('(') || trimmed.ends_with('[') {
            format!("{}  ", base)
        } else {
            base
        }
    }
}

#[derive(Debug)]
pub struct GoMode;

impl MajorMode for GoMode {
    fn kind(&self) -> MajorModeKind { MajorModeKind::Go }
    fn name(&self) -> &'static str { "Go" }

    fn comment_style(&self) -> CommentStyle {
        CommentStyle {
            line_comment: "//",
            block_comment_start: Some("/*"),
            block_comment_end: Some("*/"),
        }
    }

    fn indent_width(&self) -> usize { 1 }
    fn use_tabs(&self) -> bool { true }

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

#[derive(Debug)]
pub struct CMode;

impl MajorMode for CMode {
    fn kind(&self) -> MajorModeKind { MajorModeKind::C }
    fn name(&self) -> &'static str { "C" }

    fn comment_style(&self) -> CommentStyle {
        CommentStyle {
            line_comment: "//",
            block_comment_start: Some("/*"),
            block_comment_end: Some("*/"),
        }
    }

    fn indent_width(&self) -> usize { 4 }

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

#[derive(Debug)]
pub struct LispMode;

impl MajorMode for LispMode {
    fn kind(&self) -> MajorModeKind { MajorModeKind::Lisp }
    fn name(&self) -> &'static str { "Lisp" }

    fn comment_style(&self) -> CommentStyle {
        CommentStyle {
            line_comment: ";",
            block_comment_start: Some("#|"),
            block_comment_end: Some("|#"),
        }
    }

    fn indent_width(&self) -> usize { 2 }

    fn bracket_pairs(&self) -> Vec<BracketPair> {
        vec![
            BracketPair { open: '(', close: ')' },
            BracketPair { open: '[', close: ']' },
        ]
    }

    fn quote_pairs(&self) -> Vec<char> {
        vec!['"']
    }
}

pub fn create_mode(kind: MajorModeKind) -> Box<dyn MajorMode> {
    match kind {
        MajorModeKind::Fundamental => Box::new(FundamentalMode),
        MajorModeKind::Text => Box::new(TextMode),
        MajorModeKind::Rust => Box::new(RustMode),
        MajorModeKind::Markdown => Box::new(MarkdownMode),
        MajorModeKind::Python => Box::new(PythonMode),
        MajorModeKind::JavaScript => Box::new(JavaScriptMode),
        MajorModeKind::Go => Box::new(GoMode),
        MajorModeKind::Shell => Box::new(ShellMode),
        MajorModeKind::C => Box::new(CMode),
        MajorModeKind::Lisp => Box::new(LispMode),
    }
}

pub fn all_mode_names() -> Vec<&'static str> {
    vec![
        "Fundamental", "Text", "Rust", "Markdown", "Python",
        "JavaScript", "Go", "Shell", "C", "Lisp",
    ]
}

pub fn parse_mode_name(name: &str) -> Option<MajorModeKind> {
    match name.to_lowercase().as_str() {
        "fundamental" => Some(MajorModeKind::Fundamental),
        "text" => Some(MajorModeKind::Text),
        "rust" => Some(MajorModeKind::Rust),
        "markdown" => Some(MajorModeKind::Markdown),
        "python" => Some(MajorModeKind::Python),
        "javascript" | "js" | "typescript" | "ts" => Some(MajorModeKind::JavaScript),
        "go" | "golang" => Some(MajorModeKind::Go),
        "shell" | "sh" | "bash" => Some(MajorModeKind::Shell),
        "c" | "c++" | "cpp" => Some(MajorModeKind::C),
        "lisp" | "elisp" | "clojure" | "scheme" => Some(MajorModeKind::Lisp),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_extensions() {
        assert_eq!(MajorModeKind::detect(Path::new("foo.rs")), MajorModeKind::Rust);
        assert_eq!(MajorModeKind::detect(Path::new("foo.py")), MajorModeKind::Python);
        assert_eq!(MajorModeKind::detect(Path::new("foo.go")), MajorModeKind::Go);
        assert_eq!(MajorModeKind::detect(Path::new("foo.js")), MajorModeKind::JavaScript);
        assert_eq!(MajorModeKind::detect(Path::new("foo.ts")), MajorModeKind::JavaScript);
        assert_eq!(MajorModeKind::detect(Path::new("foo.sh")), MajorModeKind::Shell);
        assert_eq!(MajorModeKind::detect(Path::new("foo.c")), MajorModeKind::C);
        assert_eq!(MajorModeKind::detect(Path::new("foo.cpp")), MajorModeKind::C);
        assert_eq!(MajorModeKind::detect(Path::new("foo.md")), MajorModeKind::Markdown);
        assert_eq!(MajorModeKind::detect(Path::new("foo.el")), MajorModeKind::Lisp);
        assert_eq!(MajorModeKind::detect(Path::new("foo.clj")), MajorModeKind::Lisp);
        assert_eq!(MajorModeKind::detect(Path::new("foo.txt")), MajorModeKind::Text);
        assert_eq!(MajorModeKind::detect(Path::new("foo.xyz")), MajorModeKind::Fundamental);
    }

    #[test]
    fn test_detect_filenames() {
        assert_eq!(MajorModeKind::detect(Path::new("Makefile")), MajorModeKind::Shell);
        assert_eq!(MajorModeKind::detect(Path::new("CMakeLists.txt")), MajorModeKind::C);
        assert_eq!(MajorModeKind::detect(Path::new("Cargo.toml")), MajorModeKind::Rust);
        assert_eq!(MajorModeKind::detect(Path::new("go.mod")), MajorModeKind::Go);
        assert_eq!(MajorModeKind::detect(Path::new("package.json")), MajorModeKind::JavaScript);
    }

    #[test]
    fn test_parse_mode_name() {
        assert_eq!(parse_mode_name("rust"), Some(MajorModeKind::Rust));
        assert_eq!(parse_mode_name("Rust"), Some(MajorModeKind::Rust));
        assert_eq!(parse_mode_name("python"), Some(MajorModeKind::Python));
        assert_eq!(parse_mode_name("js"), Some(MajorModeKind::JavaScript));
        assert_eq!(parse_mode_name("ts"), Some(MajorModeKind::JavaScript));
        assert_eq!(parse_mode_name("golang"), Some(MajorModeKind::Go));
        assert_eq!(parse_mode_name("bash"), Some(MajorModeKind::Shell));
        assert_eq!(parse_mode_name("cpp"), Some(MajorModeKind::C));
        assert_eq!(parse_mode_name("elisp"), Some(MajorModeKind::Lisp));
        assert_eq!(parse_mode_name("clojure"), Some(MajorModeKind::Lisp));
        assert_eq!(parse_mode_name("nonexistent"), None);
    }

    #[test]
    fn test_all_mode_names() {
        let names = all_mode_names();
        assert!(names.contains(&"Rust"));
        assert!(names.contains(&"Python"));
        assert!(names.contains(&"Fundamental"));
        assert_eq!(names.len(), 10);
    }

    #[test]
    fn test_rust_mode() {
        let mode = create_mode(MajorModeKind::Rust);
        assert_eq!(mode.name(), "Rust");
        assert_eq!(mode.indent_width(), 4);
        assert!(!mode.use_tabs());
        assert_eq!(mode.comment_style().line_comment, "//");
        let brackets = mode.bracket_pairs();
        assert_eq!(brackets.len(), 4);
        assert_eq!(brackets[0].open, '(');
        assert_eq!(brackets[0].close, ')');
        assert_eq!(brackets[1].open, '{');
        assert_eq!(brackets[1].close, '}');
        assert_eq!(brackets[3].open, '<');
        assert_eq!(brackets[3].close, '>');
        assert_eq!(mode.quote_pairs(), vec!['"', '\'']);
        assert!(mode.is_word_char('a'));
        assert!(mode.is_word_char('_'));
        assert!(!mode.is_word_char(' '));
        assert_eq!(mode.auto_indent("fn main() {"), "    ");
        assert_eq!(mode.auto_indent("    let x = 1;"), "    ");
        assert_eq!(mode.auto_indent(""), "");
    }

    #[test]
    fn test_python_mode() {
        let mode = create_mode(MajorModeKind::Python);
        assert_eq!(mode.name(), "Python");
        assert_eq!(mode.comment_style().line_comment, "#");
        assert_eq!(mode.auto_indent("def foo():"), "    ");
        assert_eq!(mode.auto_indent("    if True:"), "        ");
        assert_eq!(mode.auto_indent("    x = 1"), "    ");
    }

    #[test]
    fn test_go_mode() {
        let mode = create_mode(MajorModeKind::Go);
        assert_eq!(mode.name(), "Go");
        assert!(mode.use_tabs());
        assert_eq!(mode.indent_width(), 1);
    }

    #[test]
    fn test_js_mode() {
        let mode = create_mode(MajorModeKind::JavaScript);
        assert_eq!(mode.name(), "JavaScript");
        assert_eq!(mode.indent_width(), 2);
        assert_eq!(mode.auto_indent("function foo() {"), "  ");
    }

    #[test]
    fn test_shell_mode() {
        let mode = create_mode(MajorModeKind::Shell);
        assert_eq!(mode.name(), "Shell");
        assert_eq!(mode.comment_style().line_comment, "#");
        assert_eq!(mode.indent_width(), 2);
    }

    #[test]
    fn test_c_mode() {
        let mode = create_mode(MajorModeKind::C);
        assert_eq!(mode.name(), "C");
        assert_eq!(mode.comment_style().line_comment, "//");
        assert_eq!(mode.indent_width(), 4);
    }

    #[test]
    fn test_lisp_mode() {
        let mode = create_mode(MajorModeKind::Lisp);
        assert_eq!(mode.name(), "Lisp");
        assert_eq!(mode.comment_style().line_comment, ";");
        assert_eq!(mode.indent_width(), 2);
    }

    #[test]
    fn test_text_mode() {
        let mode = create_mode(MajorModeKind::Text);
        assert_eq!(mode.name(), "Text");
        assert!(mode.comment_style().line_comment.is_empty());
        assert_eq!(mode.bracket_pairs().len(), 3); // default bracket pairs
    }

    #[test]
    fn test_fundamental_mode() {
        let mode = create_mode(MajorModeKind::Fundamental);
        assert_eq!(mode.name(), "Fundamental");
        assert!(mode.comment_style().line_comment.is_empty());
    }
}
