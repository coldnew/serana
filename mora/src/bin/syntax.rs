use crossterm::style::{self, Color, Stylize};

pub struct SyntaxHighlighter {
    keywords: Vec<String>,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        let keywords = vec![
            "if", "then", "else", "elif", "fi",
            "case", "esac",
            "for", "while", "until", "do", "done",
            "in", "function",
            "select", "time",
            "coproc",
        ].iter().map(|s| s.to_string()).collect();

        Self { keywords }
    }

    pub fn highlight(&self, input: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            match chars[i] {
                // Single quoted string
                '\'' => {
                    result.push('\'');
                    i += 1;
                    while i < chars.len() && chars[i] != '\'' {
                        result.push(chars[i]);
                        i += 1;
                    }
                    if i < chars.len() {
                        result.push('\'');
                        i += 1;
                    }
                }
                // Double quoted string
                '"' => {
                    result.push('"');
                    i += 1;
                    while i < chars.len() && chars[i] != '"' {
                        if chars[i] == '\\' && i + 1 < chars.len() {
                            result.push(chars[i]);
                            i += 1;
                            result.push(chars[i]);
                            i += 1;
                        } else {
                            result.push(chars[i]);
                            i += 1;
                        }
                    }
                    if i < chars.len() {
                        result.push('"');
                        i += 1;
                    }
                }
                // Variable
                '$' => {
                    let start = i;
                    i += 1;
                    if i < chars.len() {
                        if chars[i] == '{' {
                            // ${VAR}
                            result.push('$');
                            result.push('{');
                            i += 1;
                            while i < chars.len() && chars[i] != '}' {
                                result.push(chars[i]);
                                i += 1;
                            }
                            if i < chars.len() {
                                result.push('}');
                                i += 1;
                            }
                        } else if chars[i] == '(' {
                            // $(cmd) or $((expr))
                            result.push('$');
                            result.push('(');
                            i += 1;
                            if i < chars.len() && chars[i] == '(' {
                                result.push('(');
                                i += 1;
                                while i < chars.len() {
                                    if chars[i] == ')' && i + 1 < chars.len() && chars[i + 1] == ')' {
                                        result.push(')');
                                        result.push(')');
                                        i += 2;
                                        break;
                                    }
                                    result.push(chars[i]);
                                    i += 1;
                                }
                            } else {
                                // Simple command substitution
                                let mut depth = 1;
                                while i < chars.len() && depth > 0 {
                                    if chars[i] == '(' {
                                        depth += 1;
                                    } else if chars[i] == ')' {
                                        depth -= 1;
                                    }
                                    if depth > 0 {
                                        result.push(chars[i]);
                                    }
                                    i += 1;
                                }
                                result.push(')');
                            }
                        } else {
                            // Simple variable $VAR
                            let var_start = i;
                            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                                i += 1;
                            }
                            let var_name: String = chars[var_start..i].iter().collect();
                            result.push('$');
                            result.push_str(&var_name);
                        }
                    }
                }
                // Comment
                '#' => {
                    if i == 0 || chars[i - 1] == ' ' || chars[i - 1] == '\t' {
                        // Highlight rest of line as comment
                        let comment: String = chars[i..].iter().collect();
                        result.push_str(&comment);
                        break;
                    } else {
                        result.push('#');
                        i += 1;
                    }
                }
                // Operators
                '|' | '&' | ';' | '>' | '<' | '!' => {
                    result.push(chars[i]);
                    i += 1;
                }
                // Regular word
                _ => {
                    let word_start = i;
                    while i < chars.len() && !chars[i].is_whitespace() && !matches!(chars[i], '\'' | '"' | '$' | '#' | '|' | '&' | ';' | '>' | '<' | '!' | '(' | ')' | '{' | '}') {
                        i += 1;
                    }
                    let word: String = chars[word_start..i].iter().collect();

                    if self.keywords.contains(&word) {
                        result.push_str(&word);
                    } else if word.starts_with('-') {
                        // Flag
                        result.push_str(&word);
                    } else {
                        result.push_str(&word);
                    }
                }
            }
        }

        result
    }
}
