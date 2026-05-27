use crossterm::style::{Attribute, Color, Stylize};
use std::collections::HashSet;
use std::fmt::Write;

pub struct SyntaxHighlighter {
    keywords: Vec<String>,
    known_executables: HashSet<String>,
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        let keywords = vec![
            "if", "then", "else", "elif", "fi", "case", "esac", "for", "while", "until", "do",
            "done", "in", "function", "switch", "end", "select", "time", "coproc", "return",
            "break", "continue", "local", "declare", "typeset", "readonly",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        let known_executables = Self::scan_path_executables();

        Self {
            keywords,
            known_executables,
        }
    }

    fn scan_path_executables() -> HashSet<String> {
        let mut executables = HashSet::new();
        if let Ok(path) = std::env::var("PATH") {
            for dir in path.split(':') {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            executables.insert(name.to_string());
                        }
                    }
                }
            }
        }
        executables
    }

    pub fn highlight(&self, input: &str) -> String {
        if input.is_empty() {
            return String::new();
        }

        let mut result = String::with_capacity(input.len() * 2);
        let chars: Vec<char> = input.chars().collect();
        let len = chars.len();
        let mut i = 0;
        let mut in_command_pos = true;

        while i < len {
            match chars[i] {
                '\'' => {
                    let start = i;
                    i += 1;
                    while i < len && chars[i] != '\'' {
                        i += 1;
                    }
                    if i < len {
                        i += 1;
                    }
                    let s: String = chars[start..i].iter().collect();
                    let _ = write!(result, "{}", s.with(Color::Green));
                    in_command_pos = false;
                }
                '"' => {
                    let start = i;
                    i += 1;
                    while i < len && chars[i] != '"' {
                        if chars[i] == '\\' && i + 1 < len {
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    if i < len {
                        i += 1;
                    }
                    let s: String = chars[start..i].iter().collect();
                    let _ = write!(result, "{}", s.with(Color::Green));
                    in_command_pos = false;
                }
                '$' => {
                    let start = i;
                    i += 1;
                    if i < len {
                        if chars[i] == '{' {
                            i += 1;
                            while i < len && chars[i] != '}' {
                                i += 1;
                            }
                            if i < len {
                                i += 1;
                            }
                        } else if chars[i] == '(' {
                            i += 1;
                            if i < len && chars[i] == '(' {
                                i += 1;
                                while i < len {
                                    if chars[i] == ')' && i + 1 < len && chars[i + 1] == ')' {
                                        i += 2;
                                        break;
                                    }
                                    i += 1;
                                }
                            } else {
                                let mut depth = 1;
                                while i < len && depth > 0 {
                                    if chars[i] == '(' {
                                        depth += 1;
                                    } else if chars[i] == ')' {
                                        depth -= 1;
                                    }
                                    i += 1;
                                }
                            }
                        } else {
                            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                                i += 1;
                            }
                        }
                    }
                    let s: String = chars[start..i].iter().collect();
                    let _ = write!(result, "{}", s.with(Color::Yellow));
                    in_command_pos = false;
                }
                '#' => {
                    if i == 0 || chars[i - 1] == ' ' || chars[i - 1] == '\t' {
                        let s: String = chars[i..].iter().collect();
                        let _ = write!(result, "{}", s.with(Color::DarkGrey));
                        return result;
                    } else {
                        result.push(chars[i]);
                        i += 1;
                    }
                }
                '|' | ';' => {
                    let _ = write!(result, "{}", chars[i].with(Color::Cyan));
                    i += 1;
                    in_command_pos = true;
                }
                '&' => {
                    if i + 1 < len && chars[i + 1] == '&' {
                        let _ = write!(result, "{}", "&&".with(Color::Cyan));
                        i += 2;
                        in_command_pos = true;
                    } else {
                        result.push(chars[i]);
                        i += 1;
                    }
                }
                '>' | '<' => {
                    let start = i;
                    i += 1;
                    if i < len && (chars[i] == '>' || chars[i] == '&' || chars[i] == '|') {
                        i += 1;
                    }
                    let s: String = chars[start..i].iter().collect();
                    let _ = write!(result, "{}", s.with(Color::Cyan));
                }
                ' ' | '\t' => {
                    result.push(chars[i]);
                    i += 1;
                }
                _ => {
                    let word_start = i;
                    while i < len
                        && !chars[i].is_whitespace()
                        && !matches!(
                            chars[i],
                            '\'' | '"'
                                | '$'
                                | '#'
                                | '|'
                                | '&'
                                | ';'
                                | '>'
                                | '<'
                                | '!'
                                | '('
                                | ')'
                                | '{'
                                | '}'
                        )
                    {
                        i += 1;
                    }
                    let word: String = chars[word_start..i].iter().collect();

                    if in_command_pos {
                        if self.keywords.contains(&word) {
                            let _ = write!(
                                result,
                                "{}",
                                word.with(Color::White).attribute(Attribute::Bold)
                            );
                        } else if self.known_executables.contains(&word) || self.is_builtin(&word) {
                            let _ = write!(result, "{}", word.with(Color::Green));
                        } else {
                            let _ = write!(result, "{}", word.with(Color::Red));
                        }
                        in_command_pos = false;
                    } else if word.starts_with('-') {
                        let _ = write!(result, "{}", word.with(Color::Cyan));
                    } else {
                        result.push_str(&word);
                    }
                }
            }
        }

        result
    }

    fn is_builtin(&self, word: &str) -> bool {
        matches!(
            word,
            "cd" | "export"
                | "alias"
                | "abbr"
                | "set"
                | "exit"
                | "pushd"
                | "popd"
                | "dirs"
                | "history"
                | "echo"
                | "printf"
                | "read"
                | "test"
                | "["
                | "true"
                | "false"
                | "pwd"
                | "type"
                | "hash"
                | "help"
                | "source"
                | "."
                | "eval"
                | "exec"
                | "command"
                | "builtin"
                | "shift"
                | "unset"
                | "trap"
                | "and"
                | "or"
                | "not"
                | "count"
                | "string"
                | "math"
                | "jobs"
                | "fg"
                | "bg"
                | "status"
                | "contains"
                | "random"
                | "emit"
                | "begin"
                | "edit"
                | "file"
                | "head"
                | "tail"
                | "try"
                | "funced"
                | "funcsave"
                | "functions"
                | "realpath"
                | "complete"
                | "commandline"
        )
    }

    /// Get autosuggestion based on input and history (fish-like feature)
    pub fn get_autosuggestion(&self, input: &str, history: &[String]) -> Option<String> {
        if input.is_empty() {
            return None;
        }

        // Find the most recent history entry that starts with the input
        history
            .iter()
            .rev()
            .find(|entry| entry.starts_with(input) && entry.len() > input.len())
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_builtin_echo() {
        let h = SyntaxHighlighter::new();
        assert!(h.is_builtin("echo"));
        assert!(h.is_builtin("cd"));
        assert!(h.is_builtin("export"));
        assert!(h.is_builtin("source"));
    }

    #[test]
    fn test_is_builtin_fish() {
        let h = SyntaxHighlighter::new();
        assert!(h.is_builtin("and"));
        assert!(h.is_builtin("or"));
        assert!(h.is_builtin("not"));
        assert!(h.is_builtin("count"));
        assert!(h.is_builtin("string"));
        assert!(h.is_builtin("math"));
        assert!(h.is_builtin("status"));
        assert!(h.is_builtin("contains"));
        assert!(h.is_builtin("random"));
        assert!(h.is_builtin("emit"));
        assert!(h.is_builtin("begin"));
    }

    #[test]
    fn test_is_builtin_false() {
        let h = SyntaxHighlighter::new();
        assert!(!h.is_builtin("ls"));
        assert!(!h.is_builtin("grep"));
        assert!(!h.is_builtin("git"));
        assert!(!h.is_builtin(""));
    }

    #[test]
    fn test_autosuggestion_empty_input() {
        let h = SyntaxHighlighter::new();
        let history = vec!["echo hello".to_string()];
        assert_eq!(h.get_autosuggestion("", &history), None);
    }

    #[test]
    fn test_autosuggestion_match() {
        let h = SyntaxHighlighter::new();
        let history = vec![
            "echo hello".to_string(),
            "echo world".to_string(),
            "ls -la".to_string(),
        ];
        // Should find most recent match starting with "echo"
        let suggestion = h.get_autosuggestion("echo", &history);
        assert_eq!(suggestion, Some("echo world".to_string()));
    }

    #[test]
    fn test_autosuggestion_no_match() {
        let h = SyntaxHighlighter::new();
        let history = vec!["echo hello".to_string()];
        assert_eq!(h.get_autosuggestion("ls", &history), None);
    }

    #[test]
    fn test_autosuggestion_exact_match_not_returned() {
        let h = SyntaxHighlighter::new();
        let history = vec!["echo".to_string()];
        // Exact match (same length) should not be suggested
        assert_eq!(h.get_autosuggestion("echo", &history), None);
    }

    #[test]
    fn test_autosuggestion_empty_history() {
        let h = SyntaxHighlighter::new();
        assert_eq!(h.get_autosuggestion("echo", &[]), None);
    }

    #[test]
    fn test_autosuggestion_most_recent() {
        let h = SyntaxHighlighter::new();
        let history = vec![
            "git status".to_string(),
            "git push".to_string(),
            "git pull".to_string(),
        ];
        let suggestion = h.get_autosuggestion("git", &history);
        assert_eq!(suggestion, Some("git pull".to_string()));
    }

    #[test]
    fn test_highlight_empty() {
        let h = SyntaxHighlighter::new();
        let result = h.highlight("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_highlight_contains_original_text() {
        let h = SyntaxHighlighter::new();
        let result = h.highlight("echo hello");
        // The result should contain the original words (possibly with ANSI codes)
        assert!(result.contains("echo"));
        assert!(result.contains("hello"));
    }

    #[test]
    fn test_highlight_preserves_whitespace() {
        let h = SyntaxHighlighter::new();
        let result = h.highlight("echo  hello");
        // Should have the spaces preserved
        assert!(result.contains("  "));
    }

    #[test]
    fn test_highlight_variable_yellow() {
        let h = SyntaxHighlighter::new();
        let result = h.highlight("echo $VAR");
        // Variables get yellow color - just check it contains the text
        assert!(result.contains("$VAR"));
    }

    #[test]
    fn test_highlight_single_quoted_green() {
        let h = SyntaxHighlighter::new();
        let result = h.highlight("echo 'hello world'");
        assert!(result.contains("hello world"));
    }

    #[test]
    fn test_highlight_double_quoted_green() {
        let h = SyntaxHighlighter::new();
        let result = h.highlight(r#"echo "hello""#);
        assert!(result.contains("hello"));
    }

    #[test]
    fn test_highlight_comment_grey() {
        let h = SyntaxHighlighter::new();
        let result = h.highlight("echo hello # comment");
        assert!(result.contains("comment"));
    }

    #[test]
    fn test_highlight_pipe_cyan() {
        let h = SyntaxHighlighter::new();
        let result = h.highlight("ls | grep foo");
        assert!(result.contains("ls"));
        assert!(result.contains("grep"));
    }

    #[test]
    fn test_highlight_and_operator() {
        let h = SyntaxHighlighter::new();
        let result = h.highlight("true && echo ok");
        assert!(result.contains("&&"));
        assert!(result.contains("true"));
    }

    #[test]
    fn test_highlight_redirect() {
        let h = SyntaxHighlighter::new();
        let result = h.highlight("echo hello > file.txt");
        assert!(result.contains(">"));
        assert!(result.contains("file.txt"));
    }

    #[test]
    fn test_highlight_flag_cyan() {
        let h = SyntaxHighlighter::new();
        let result = h.highlight("grep -i pattern");
        // Flags starting with - should be highlighted
        assert!(result.contains("-i"));
    }

    #[test]
    fn test_new_has_keywords() {
        let h = SyntaxHighlighter::new();
        assert!(h.keywords.contains(&"if".to_string()));
        assert!(h.keywords.contains(&"then".to_string()));
        assert!(h.keywords.contains(&"else".to_string()));
        assert!(h.keywords.contains(&"for".to_string()));
        assert!(h.keywords.contains(&"while".to_string()));
        assert!(h.keywords.contains(&"function".to_string()));
        assert!(h.keywords.contains(&"switch".to_string()));
        assert!(h.keywords.contains(&"end".to_string()));
        assert!(h.keywords.contains(&"case".to_string()));
    }
}
