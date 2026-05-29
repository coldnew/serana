use std::sync::Arc;

use crate::lisp::types::{Symbol, Value};

#[derive(Debug, Clone, thiserror::Error)]
pub enum ReadError {
    #[error("unexpected EOF")]
    UnexpectedEof,
    #[error("unexpected character: '{0}'")]
    UnexpectedChar(char),
    #[error("unexpected token: {0}")]
    UnexpectedToken(String),
    #[error("unmatched delimiter: '{0}'")]
    UnmatchedDelimiter(char),
    #[error("invalid escape: '\\{0}'")]
    InvalidEscape(char),
    #[error("invalid number: {0}")]
    InvalidNumber(String),
    #[error("read error at line {line}, col {col}: {msg}")]
    Positional {
        line: usize,
        col: usize,
        msg: String,
    },
}

pub struct Reader {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Reader {
    pub fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else if ch == ';' {
                // Line comment
                while let Some(ch) = self.peek() {
                    if ch == '\n' {
                        break;
                    }
                    self.advance();
                }
            } else if self.check_prefix("#_") {
                // Discard macro: skip next form
                self.advance(); // #
                self.advance(); // _
                self.skip_whitespace();
                let _ = self.read_form();
            } else if self.check_prefix("#!") {
                // Shebang line
                while let Some(ch) = self.peek() {
                    if ch == '\n' {
                        break;
                    }
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn check_prefix(&self, prefix: &str) -> bool {
        let remaining: String = self.chars[self.pos..].iter().collect();
        remaining.starts_with(prefix)
    }

    pub fn read_forms(&mut self) -> Result<Vec<Value>, ReadError> {
        let mut forms = Vec::new();
        self.skip_whitespace();
        while self.peek().is_some() {
            forms.push(self.read_form()?);
            self.skip_whitespace();
        }
        Ok(forms)
    }

    pub fn read_form(&mut self) -> Result<Value, ReadError> {
        self.skip_whitespace();
        let ch = self.peek().ok_or(ReadError::UnexpectedEof)?;

        match ch {
            '(' => self.read_list(),
            '[' => self.read_vector(),
            '{' => self.read_map(),
            '"' => self.read_string(),
            '\\' => self.read_char(),
            ':' => self.read_keyword(),
            '#' => self.read_dispatch(),
            '\'' => self.read_quote(),
            '`' => self.read_syntax_quote(),
            '~' => self.read_unquote(),
            '@' => self.read_deref(),
            '^' => self.read_meta(),
            ')' | ']' | '}' => Err(ReadError::UnmatchedDelimiter(ch)),
            _ if ch.is_ascii_digit() || (ch == '-' && self.peek_digit()) => self.read_number(),
            _ => self.read_symbol(),
        }
    }

    fn peek_digit(&self) -> bool {
        self.chars
            .get(self.pos + 1)
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
    }

    fn read_list(&mut self) -> Result<Value, ReadError> {
        self.advance(); // (
        let mut items = Vec::new();
        self.skip_whitespace();
        while self.peek().map_or(false, |c| c != ')') {
            items.push(self.read_form()?);
            self.skip_whitespace();
        }
        self.advance().ok_or(ReadError::UnexpectedEof)?; // )
        Ok(Value::list(items))
    }

    fn read_vector(&mut self) -> Result<Value, ReadError> {
        self.advance(); // [
        let mut items = Vec::new();
        self.skip_whitespace();
        while self.peek().map_or(false, |c| c != ']') {
            items.push(self.read_form()?);
            self.skip_whitespace();
        }
        self.advance().ok_or(ReadError::UnexpectedEof)?; // ]
        Ok(Value::vector(items))
    }

    fn read_map(&mut self) -> Result<Value, ReadError> {
        self.advance(); // {
        let mut pairs = Vec::new();
        self.skip_whitespace();
        while self.peek().map_or(false, |c| c != '}') {
            let key = self.read_form()?;
            self.skip_whitespace();
            let val = self.read_form()?;
            pairs.push((key, val));
            self.skip_whitespace();
        }
        self.advance().ok_or(ReadError::UnexpectedEof)?; // }
        Ok(Value::map(pairs))
    }

    fn read_set(&mut self) -> Result<Value, ReadError> {
        self.advance(); // { (already consumed #)
        let mut items = Vec::new();
        self.skip_whitespace();
        while self.peek().map_or(false, |c| c != '}') {
            items.push(self.read_form()?);
            self.skip_whitespace();
        }
        self.advance().ok_or(ReadError::UnexpectedEof)?; // }
        Ok(Value::set(items))
    }

    fn read_string(&mut self) -> Result<Value, ReadError> {
        self.advance(); // "
        let mut s = String::new();
        loop {
            match self.advance().ok_or(ReadError::UnexpectedEof)? {
                '"' => break,
                '\\' => {
                    let esc = self.advance().ok_or(ReadError::UnexpectedEof)?;
                    match esc {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        'r' => s.push('\r'),
                        '\\' => s.push('\\'),
                        '"' => s.push('"'),
                        _ => return Err(ReadError::InvalidEscape(esc)),
                    }
                }
                ch => s.push(ch),
            }
        }
        Ok(Value::string(s))
    }

    fn read_char(&mut self) -> Result<Value, ReadError> {
        self.advance(); // \
        let ch = self.advance().ok_or(ReadError::UnexpectedEof)?;
        // Handle named characters like \newline, \space, \tab
        if ch.is_ascii_alphabetic() {
            let mut name = String::from(ch);
            while let Some(c) = self.peek() {
                if c.is_ascii_alphanumeric() {
                    name.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
            match name.as_str() {
                "newline" => Ok(Value::Char('\n')),
                "space" => Ok(Value::Char(' ')),
                "tab" => Ok(Value::Char('\t')),
                "return" => Ok(Value::Char('\r')),
                _ => Err(ReadError::UnexpectedToken(format!("\\{}", name))),
            }
        } else {
            Ok(Value::Char(ch))
        }
    }

    fn read_keyword(&mut self) -> Result<Value, ReadError> {
        self.advance(); // :
        let mut name = String::new();
        // Check for ::
        let _auto_namespace = if self.peek() == Some(':') {
            self.advance();
            true
        } else {
            false
        };
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric()
                || ch == '_'
                || ch == '-'
                || ch == '*'
                || ch == '!'
                || ch == '?'
                || ch == '/'
                || ch == '.'
            {
                if ch == '/' && !name.is_empty() && !name.ends_with(':') {
                    name.push(ch);
                    self.advance();
                    // Read the name part after /
                    while let Some(ch2) = self.peek() {
                        if ch2.is_alphanumeric()
                            || ch2 == '_'
                            || ch2 == '-'
                            || ch2 == '*'
                            || ch2 == '!'
                            || ch2 == '?'
                            || ch2 == '.'
                        {
                            name.push(ch2);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    break;
                }
                name.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        if name.is_empty() {
            return Err(ReadError::UnexpectedToken("empty keyword".to_string()));
        }
        Ok(Value::keyword(name))
    }

    fn read_symbol(&mut self) -> Result<Value, ReadError> {
        let mut name = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric()
                || ch == '_'
                || ch == '-'
                || ch == '*'
                || ch == '!'
                || ch == '?'
                || ch == '/'
                || ch == '.'
                || ch == '+'
                || ch == '='
                || ch == '<'
                || ch == '>'
                || ch == '&'
                || ch == '%'
            {
                name.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        match name.as_str() {
            "nil" => Ok(Value::Nil),
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => Ok(Value::symbol(name)),
        }
    }

    fn read_number(&mut self) -> Result<Value, ReadError> {
        let mut num_str = String::new();
        let mut is_float = false;

        if self.peek() == Some('-') {
            num_str.push('-');
            self.advance();
        }

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.advance();
            } else if ch == '.' && !is_float {
                is_float = true;
                num_str.push(ch);
                self.advance();
            } else if ch == 'e' || ch == 'E' {
                is_float = true;
                num_str.push(ch);
                self.advance();
                if self.peek() == Some('+') || self.peek() == Some('-') {
                    num_str.push(self.advance().unwrap());
                }
            } else if ch == '_' {
                self.advance(); // skip underscores in numbers
            } else {
                break;
            }
        }

        if is_float {
            num_str
                .parse::<f64>()
                .map(Value::Float)
                .map_err(|_| ReadError::InvalidNumber(num_str))
        } else {
            num_str
                .parse::<i64>()
                .map(Value::Int)
                .map_err(|_| ReadError::InvalidNumber(num_str))
        }
    }

    fn read_dispatch(&mut self) -> Result<Value, ReadError> {
        self.advance(); // #
        let ch = self.peek().ok_or(ReadError::UnexpectedEof)?;
        match ch {
            '{' => self.read_set(),
            '"' => self.read_regex(),
            '(' => {
                // Anonymous function: #(fn-body)
                self.advance(); // (
                let mut body = Vec::new();
                self.skip_whitespace();
                while self.peek().map_or(false, |c| c != ')') {
                    body.push(self.read_form()?);
                    self.skip_whitespace();
                }
                self.advance().ok_or(ReadError::UnexpectedEof)?; // )
                                                                 // Create a fn with auto-generated params %1, %2, etc.
                let mut params = Vec::new();
                let mut max_arg = 0;
                for form in &body {
                    Self::find_anon_args(form, &mut max_arg);
                }
                if max_arg == 0 {
                    max_arg = 1; // % alone means %1
                }
                for i in 1..=max_arg {
                    params.push(crate::lisp::types::Param::Named(Symbol {
                        ns: None,
                        name: Arc::new(format!("%{}", i)),
                    }));
                }
                // Also check for %& (rest arg)
                if Self::has_rest_arg(&body) {
                    params.push(crate::lisp::types::Param::Rest(Symbol {
                        ns: None,
                        name: Arc::new("%&".to_string()),
                    }));
                }
                Ok(Value::Fn(crate::lisp::types::FnValue {
                    name: None,
                    params,
                    body: Arc::new(body),
                    closure: Arc::new(parking_lot::Mutex::new(std::collections::HashMap::new())),
                    is_macro: false,
                    meta: None,
                }))
            }
            _ => Err(ReadError::UnexpectedChar(ch)),
        }
    }

    fn find_anon_args(form: &Value, max_arg: &mut usize) {
        if let Value::Symbol(s) = form {
            if s.name.starts_with('%') && s.ns.is_none() {
                if *s.name == *"%" {
                    *max_arg = (*max_arg).max(1);
                } else if let Ok(n) = s.name[1..].parse::<usize>() {
                    *max_arg = (*max_arg).max(n);
                }
            }
        }
        if let Some(vals) = form.seq_to_vec() {
            for v in vals {
                Self::find_anon_args(v, max_arg);
            }
        }
    }

    fn has_rest_arg(body: &[Value]) -> bool {
        for form in body {
            if let Value::Symbol(s) = form {
                if *s.name == *"%&" && s.ns.is_none() {
                    return true;
                }
            }
            if let Some(vals) = form.seq_to_vec() {
                if Self::has_rest_arg(vals) {
                    return true;
                }
            }
        }
        false
    }

    fn read_regex(&mut self) -> Result<Value, ReadError> {
        self.advance(); // "
        let mut pattern = String::new();
        loop {
            match self.advance().ok_or(ReadError::UnexpectedEof)? {
                '"' => break,
                '\\' => {
                    let esc = self.advance().ok_or(ReadError::UnexpectedEof)?;
                    pattern.push('\\');
                    pattern.push(esc);
                }
                ch => pattern.push(ch),
            }
        }
        // Store regex as a string with a special marker
        Ok(Value::string(format!("#regex:{}", pattern)))
    }

    fn read_quote(&mut self) -> Result<Value, ReadError> {
        self.advance(); // '
        let form = self.read_form()?;
        Ok(Value::list(vec![Value::symbol("quote"), form]))
    }

    fn read_syntax_quote(&mut self) -> Result<Value, ReadError> {
        self.advance(); // `
        let form = self.read_form()?;
        // Expand syntax-quote (simplified - full implementation would handle ~@ etc.)
        Ok(Value::list(vec![Value::symbol("syntax-quote"), form]))
    }

    fn read_unquote(&mut self) -> Result<Value, ReadError> {
        self.advance(); // ~
        if self.peek() == Some('@') {
            self.advance(); // @
            let form = self.read_form()?;
            Ok(Value::list(vec![Value::symbol("unquote-splicing"), form]))
        } else {
            let form = self.read_form()?;
            Ok(Value::list(vec![Value::symbol("unquote"), form]))
        }
    }

    fn read_deref(&mut self) -> Result<Value, ReadError> {
        self.advance(); // @
        let form = self.read_form()?;
        Ok(Value::list(vec![Value::symbol("deref"), form]))
    }

    fn read_meta(&mut self) -> Result<Value, ReadError> {
        self.advance(); // ^
        let meta = self.read_form()?;
        let form = self.read_form()?;
        Ok(Value::list(vec![Value::symbol("with-meta"), form, meta]))
    }
}

/// Convenience function to read a single form from a string
pub fn read_str(input: &str) -> Result<Value, ReadError> {
    let mut reader = Reader::new(input);
    reader.read_form()
}

/// Read all forms from a string
pub fn read_all(input: &str) -> Result<Vec<Value>, ReadError> {
    let mut reader = Reader::new(input);
    reader.read_forms()
}
