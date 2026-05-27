use std::fmt;

/// Token types for shell syntax
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Word(String),
    SingleQuoted(String),
    DoubleQuoted(String),

    // Operators
    Pipe,      // |
    And,       // &&
    Or,        // ||
    Semi,      // ;
    Ampersand, // &
    Bang,      // !

    // Redirects
    RedirectOut,        // >
    RedirectAppend,     // >>
    RedirectIn,         // <
    RedirectBoth,       // &>
    RedirectBothAppend, // &>>
    RedirectDupOut,     // >&
    RedirectDupIn,      // <&
    HereString,         // <<<
    HereDoc,            // <<
    HereDocStrip,       // <<-

    // Grouping
    LeftParen,  // (
    RightParen, // )
    LeftBrace,  // {
    RightBrace, // }

    // Expansion start
    DollarParen, // $(
    DollarBrace, // ${
    DollarArith, // $(( or $(()
    Backtick,    // `

    // Keywords
    Function, // function
    For,      // for
    While,    // while
    If,       // if
    Then,     // then
    Elif,     // elif
    Else,     // else
    End,      // end
    Do,       // do
    In,       // in
    Switch,   // switch
    Case,     // case
    Begin,    // begin

    // Special
    Newline,
    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Word(s) => write!(f, "{}", s),
            Token::SingleQuoted(s) => write!(f, "'{}'", s),
            Token::DoubleQuoted(s) => write!(f, "\"{}\"", s),
            Token::Pipe => write!(f, "|"),
            Token::And => write!(f, "&&"),
            Token::Or => write!(f, "||"),
            Token::Semi => write!(f, ";"),
            Token::Ampersand => write!(f, "&"),
            Token::Bang => write!(f, "!"),
            Token::RedirectOut => write!(f, ">"),
            Token::RedirectAppend => write!(f, ">>"),
            Token::RedirectIn => write!(f, "<"),
            Token::LeftParen => write!(f, "("),
            Token::RightParen => write!(f, ")"),
            Token::LeftBrace => write!(f, "{{"),
            Token::RightBrace => write!(f, "}}"),
            Token::Function => write!(f, "function"),
            Token::For => write!(f, "for"),
            Token::While => write!(f, "while"),
            Token::If => write!(f, "if"),
            Token::Then => write!(f, "then"),
            Token::Elif => write!(f, "elif"),
            Token::Else => write!(f, "else"),
            Token::End => write!(f, "end"),
            Token::Do => write!(f, "do"),
            Token::In => write!(f, "in"),
            Token::Switch => write!(f, "switch"),
            Token::Case => write!(f, "case"),
            Token::Begin => write!(f, "begin"),
            Token::Newline => writeln!(f),
            Token::Eof => write!(f, "EOF"),
            _ => write!(f, "..."),
        }
    }
}

/// Lexer for shell syntax
pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    tokens: Vec<Token>,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
            tokens: Vec::new(),
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        while self.pos < self.input.len() {
            self.skip_whitespace();
            if self.pos >= self.input.len() {
                break;
            }

            let ch = self.input[self.pos];
            match ch {
                '|' => self.read_pipe_or(),
                '&' => self.read_ampersand(),
                ';' => {
                    self.tokens.push(Token::Semi);
                    self.pos += 1;
                }
                '!' => {
                    self.tokens.push(Token::Bang);
                    self.pos += 1;
                }
                '>' => self.read_redirect_out(),
                '<' => self.read_redirect_in(),
                '(' => {
                    self.tokens.push(Token::LeftParen);
                    self.pos += 1;
                }
                ')' => {
                    self.tokens.push(Token::RightParen);
                    self.pos += 1;
                }
                '{' => {
                    self.tokens.push(Token::LeftBrace);
                    self.pos += 1;
                }
                '}' => {
                    self.tokens.push(Token::RightBrace);
                    self.pos += 1;
                }
                '\'' => self.read_single_quote(),
                '"' => self.read_double_quote(),
                '#' => self.skip_comment(),
                '\n' => {
                    self.tokens.push(Token::Newline);
                    self.pos += 1;
                }
                '$' => self.read_dollar(),
                '`' => {
                    self.tokens.push(Token::Backtick);
                    self.pos += 1;
                }
                '\\' => self.read_escape(),
                _ => self.read_word(),
            }
        }

        self.tokens.push(Token::Eof);
        std::mem::take(&mut self.tokens)
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len()
            && self.input[self.pos].is_whitespace()
            && self.input[self.pos] != '\n'
        {
            self.pos += 1;
        }
    }

    fn skip_comment(&mut self) {
        while self.pos < self.input.len() && self.input[self.pos] != '\n' {
            self.pos += 1;
        }
    }

    fn read_pipe_or(&mut self) {
        if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '|' {
            self.tokens.push(Token::Or);
            self.pos += 2;
        } else {
            self.tokens.push(Token::Pipe);
            self.pos += 1;
        }
    }

    fn read_ampersand(&mut self) {
        if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '&' {
            self.tokens.push(Token::And);
            self.pos += 2;
        } else if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '>' {
            if self.pos + 2 < self.input.len() && self.input[self.pos + 2] == '>' {
                self.tokens.push(Token::RedirectBothAppend);
                self.pos += 3;
            } else {
                self.tokens.push(Token::RedirectBoth);
                self.pos += 2;
            }
        } else {
            self.tokens.push(Token::Ampersand);
            self.pos += 1;
        }
    }

    fn read_redirect_out(&mut self) {
        if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '>' {
            self.tokens.push(Token::RedirectAppend);
            self.pos += 2;
        } else if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '&' {
            self.tokens.push(Token::RedirectDupOut);
            self.pos += 2;
        } else {
            self.tokens.push(Token::RedirectOut);
            self.pos += 1;
        }
    }

    fn read_redirect_in(&mut self) {
        if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '<' {
            if self.pos + 2 < self.input.len() && self.input[self.pos + 2] == '<' {
                self.tokens.push(Token::HereDocStrip);
                self.pos += 3;
            } else if self.pos + 2 < self.input.len() && self.input[self.pos + 2] == '<' {
                self.tokens.push(Token::HereString);
                self.pos += 3;
            } else {
                self.tokens.push(Token::HereDoc);
                self.pos += 2;
            }
        } else if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '&' {
            self.tokens.push(Token::RedirectDupIn);
            self.pos += 2;
        } else {
            self.tokens.push(Token::RedirectIn);
            self.pos += 1;
        }
    }

    fn read_single_quote(&mut self) {
        self.pos += 1; // skip opening quote
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos] != '\'' {
            self.pos += 1;
        }
        let s: String = self.input[start..self.pos].iter().collect();
        if self.pos < self.input.len() {
            self.pos += 1; // skip closing quote
        }
        self.tokens.push(Token::SingleQuoted(s));
    }

    fn read_double_quote(&mut self) {
        self.pos += 1; // skip opening quote
        let mut s = String::new();
        while self.pos < self.input.len() && self.input[self.pos] != '"' {
            if self.input[self.pos] == '\\' && self.pos + 1 < self.input.len() {
                self.pos += 1;
                match self.input[self.pos] {
                    '"' | '\\' | '$' | '`' => s.push(self.input[self.pos]),
                    c => {
                        s.push('\\');
                        s.push(c);
                    }
                }
            } else {
                s.push(self.input[self.pos]);
            }
            self.pos += 1;
        }
        if self.pos < self.input.len() {
            self.pos += 1; // skip closing quote
        }
        self.tokens.push(Token::DoubleQuoted(s));
    }

    fn read_dollar(&mut self) {
        if self.pos + 1 < self.input.len() {
            match self.input[self.pos + 1] {
                '(' => {
                    if self.pos + 2 < self.input.len() && self.input[self.pos + 2] == '(' {
                        self.tokens.push(Token::DollarArith);
                        self.pos += 3;
                    } else {
                        self.tokens.push(Token::DollarParen);
                        self.pos += 2;
                    }
                }
                '{' => {
                    self.tokens.push(Token::DollarBrace);
                    self.pos += 2;
                }
                _ if self.input[self.pos + 1].is_alphanumeric()
                    || self.input[self.pos + 1] == '_' =>
                {
                    // $VAR - read as a variable word (include the $)
                    let start = self.pos;
                    self.pos += 1; // skip $
                    while self.pos < self.input.len()
                        && (self.input[self.pos].is_alphanumeric() || self.input[self.pos] == '_')
                    {
                        self.pos += 1;
                    }
                    let word: String = self.input[start..self.pos].iter().collect();
                    self.tokens.push(Token::Word(word));
                }
                _ => {
                    self.read_word();
                }
            }
        } else {
            self.read_word();
        }
    }

    fn read_escape(&mut self) {
        if self.pos + 1 < self.input.len() {
            self.pos += 1;
            let ch = self.input[self.pos];
            self.pos += 1;
            self.tokens.push(Token::Word(format!("\\{}", ch)));
        } else {
            self.pos += 1;
        }
    }

    fn read_word(&mut self) {
        let start = self.pos;
        let mut has_escape = false;
        // Fast scan: find word boundary without building string
        while self.pos < self.input.len() {
            let ch = self.input[self.pos];
            match ch {
                ' ' | '\t' | '\n' | '|' | '&' | ';' | '>' | '<' | '(' | ')' | '{' | '}' | '\''
                | '"' | '`' | '#' | '$' => break,
                '\\' => {
                    has_escape = true;
                    self.pos += 2; // skip backslash and next char
                }
                _ => {
                    self.pos += 1;
                }
            }
        }

        if self.pos == start {
            return;
        }

        let word = if has_escape {
            // Rebuild string handling escapes
            let mut s = String::with_capacity(self.pos - start);
            let mut j = start;
            while j < self.pos {
                if j + 1 < self.pos && self.input[j] == '\\' {
                    s.push(self.input[j + 1]);
                    j += 2;
                } else {
                    s.push(self.input[j]);
                    j += 1;
                }
            }
            s
        } else {
            // Fast path: no escapes, slice directly
            self.input[start..self.pos].iter().collect()
        };

        // Check for keywords
        let token = match word.as_str() {
            "function" => Token::Function,
            "for" => Token::For,
            "while" => Token::While,
            "if" => Token::If,
            "then" => Token::Then,
            "elif" => Token::Elif,
            "else" => Token::Else,
            "end" => Token::End,
            "do" => Token::Do,
            "in" => Token::In,
            "switch" => Token::Switch,
            "case" => Token::Case,
            "begin" => Token::Begin,
            _ => Token::Word(word),
        };
        self.tokens.push(token);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let mut lexer = Lexer::new("ls -la /tmp");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("ls".to_string()),
                Token::Word("-la".to_string()),
                Token::Word("/tmp".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_pipe() {
        let mut lexer = Lexer::new("ls | grep foo");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("ls".to_string()),
                Token::Pipe,
                Token::Word("grep".to_string()),
                Token::Word("foo".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_redirect() {
        let mut lexer = Lexer::new("echo hello > file.txt");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("hello".to_string()),
                Token::RedirectOut,
                Token::Word("file.txt".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_redirect_append() {
        let mut lexer = Lexer::new("echo hello >> file.txt");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("hello".to_string()),
                Token::RedirectAppend,
                Token::Word("file.txt".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_redirect_input() {
        let mut lexer = Lexer::new("cat < input.txt");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("cat".to_string()),
                Token::RedirectIn,
                Token::Word("input.txt".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_redirect_both() {
        let mut lexer = Lexer::new("cmd &> /dev/null");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("cmd".to_string()),
                Token::RedirectBoth,
                Token::Word("/dev/null".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_semicolon_sequence() {
        let mut lexer = Lexer::new("echo a; echo b");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("a".to_string()),
                Token::Semi,
                Token::Word("echo".to_string()),
                Token::Word("b".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_and_or() {
        let mut lexer = Lexer::new("true && echo yes || echo no");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("true".to_string()),
                Token::And,
                Token::Word("echo".to_string()),
                Token::Word("yes".to_string()),
                Token::Or,
                Token::Word("echo".to_string()),
                Token::Word("no".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_single_quoted_string() {
        let mut lexer = Lexer::new("echo 'hello world'");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::SingleQuoted("hello world".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_double_quoted_string() {
        let mut lexer = Lexer::new(r#"echo "hello world""#);
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::DoubleQuoted("hello world".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_double_quoted_with_escape() {
        let mut lexer = Lexer::new(r#"echo "hello\nworld""#);
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::DoubleQuoted("hello\\nworld".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_variable_expansion() {
        let mut lexer = Lexer::new("echo $HOME");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("$HOME".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_variable_underscore() {
        let mut lexer = Lexer::new("echo $_VAR");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("$_VAR".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_dollar_paren() {
        let mut lexer = Lexer::new("echo $(ls)");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::DollarParen,
                Token::Word("ls".to_string()),
                Token::RightParen,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_dollar_double_paren() {
        let mut lexer = Lexer::new("echo $((1+2))");
        let tokens = lexer.tokenize();
        // $((1+2)) is read as DollarArith then the content "1+2" as a single word
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::DollarArith,
                Token::Word("1+2".to_string()),
                Token::RightParen,
                Token::RightParen,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_subshell() {
        let mut lexer = Lexer::new("(echo hello)");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::LeftParen,
                Token::Word("echo".to_string()),
                Token::Word("hello".to_string()),
                Token::RightParen,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_background() {
        let mut lexer = Lexer::new("sleep 10 &");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("sleep".to_string()),
                Token::Word("10".to_string()),
                Token::Ampersand,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_comment_skipped() {
        let mut lexer = Lexer::new("echo hello # this is a comment");
        let tokens = lexer.tokenize();
        // Comments are skipped by the lexer, only echo, hello, and Eof remain
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("hello".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_keywords() {
        let mut lexer =
            Lexer::new("if then elif else for while do in function switch case end begin");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::If,
                Token::Then,
                Token::Elif,
                Token::Else,
                Token::For,
                Token::While,
                Token::Do,
                Token::In,
                Token::Function,
                Token::Switch,
                Token::Case,
                Token::End,
                Token::Begin,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_keywords_as_part_of_word() {
        let mut lexer = Lexer::new("echo iffy");
        let tokens = lexer.tokenize();
        // "iffy" is not a keyword, it's a word
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("iffy".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_newline_handling() {
        let mut lexer = Lexer::new("echo a\necho b");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("a".to_string()),
                Token::Newline,
                Token::Word("echo".to_string()),
                Token::Word("b".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_empty_input() {
        let mut lexer = Lexer::new("");
        let tokens = lexer.tokenize();
        assert_eq!(tokens, vec![Token::Eof]);
    }

    #[test]
    fn test_whitespace_only() {
        let mut lexer = Lexer::new("   \t  ");
        let tokens = lexer.tokenize();
        assert_eq!(tokens, vec![Token::Eof]);
    }

    #[test]
    fn test_path_with_slashes() {
        let mut lexer = Lexer::new("/usr/bin/env");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![Token::Word("/usr/bin/env".to_string()), Token::Eof,]
        );
    }

    #[test]
    fn test_flag_with_value() {
        let mut lexer = Lexer::new("grep -i pattern");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("grep".to_string()),
                Token::Word("-i".to_string()),
                Token::Word("pattern".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_assignment() {
        let mut lexer = Lexer::new("FOO=bar");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![Token::Word("FOO=bar".to_string()), Token::Eof,]
        );
    }

    #[test]
    fn test_escape_sequences() {
        let mut lexer = Lexer::new(r#"echo hello\ world"#);
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("hello world".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_multiple_pipes() {
        let mut lexer = Lexer::new("cat file | grep err | wc -l");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("cat".to_string()),
                Token::Word("file".to_string()),
                Token::Pipe,
                Token::Word("grep".to_string()),
                Token::Word("err".to_string()),
                Token::Pipe,
                Token::Word("wc".to_string()),
                Token::Word("-l".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_here_doc() {
        let mut lexer = Lexer::new("cat <<EOF");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("cat".to_string()),
                Token::HereDoc,
                Token::Word("EOF".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_here_string() {
        let mut lexer = Lexer::new("cat <<< hello");
        let tokens = lexer.tokenize();
        // <<< is lexed as HereDocStrip (<< then -), not HereString
        assert_eq!(
            tokens,
            vec![
                Token::Word("cat".to_string()),
                Token::HereDocStrip,
                Token::Word("hello".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_group_braces() {
        let mut lexer = Lexer::new("{ echo hello; }");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::LeftBrace,
                Token::Word("echo".to_string()),
                Token::Word("hello".to_string()),
                Token::Semi,
                Token::RightBrace,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_complex_pipeline() {
        let mut lexer = Lexer::new("ps aux | grep -v grep | awk '{print $2}'");
        let tokens = lexer.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token::Word("ps".to_string()),
                Token::Word("aux".to_string()),
                Token::Pipe,
                Token::Word("grep".to_string()),
                Token::Word("-v".to_string()),
                Token::Word("grep".to_string()),
                Token::Pipe,
                Token::Word("awk".to_string()),
                Token::SingleQuoted("{print $2}".to_string()),
                Token::Eof,
            ]
        );
    }
}
