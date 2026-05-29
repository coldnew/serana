//! Interactive REPL for mora-lisp.
//!
//! Reads lines from stdin, evaluates them, prints results.
//! Supports multi-line input (parentheses/brackets must balance).
//! Errors display with stack traces.

use std::io::{self, BufRead, Write};

use crate::lisp::eval::EvalError;
use crate::lisp::types::Value;
use crate::lisp::vm;
use crate::lisp::MoraLisp;

pub struct Repl {
    lisp: MoraLisp,
    prompt: String,
    multiline: bool,
    buffer: String,
}

impl Repl {
    pub fn new() -> Self {
        Self {
            lisp: MoraLisp::new(),
            prompt: "mora> ".to_string(),
            multiline: false,
            buffer: String::new(),
        }
    }

    pub fn evaluator(&self) -> &crate::lisp::eval::Evaluator {
        &self.lisp.evaluator
    }

    pub fn evaluator_mut(&mut self) -> &mut crate::lisp::eval::Evaluator {
        &mut self.lisp.evaluator
    }

    /// Run the REPL loop. Returns when stdin closes or user sends (quit).
    pub fn run(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            // Print prompt
            if self.multiline {
                write!(stdout, "     | ")?;
            } else {
                write!(stdout, "{}", self.prompt)?;
            }
            stdout.flush()?;

            // Read line
            let mut line = String::new();
            if stdin.lock().read_line(&mut line)? == 0 {
                // EOF
                if !self.buffer.is_empty() {
                    eprintln!("unexpected EOF in multi-line input");
                }
                break;
            }

            let line = line.trim_end_matches('\n').trim_end_matches('\r');

            // Accumulate into buffer
            if !self.buffer.is_empty() {
                self.buffer.push('\n');
            }
            self.buffer.push_str(line);

            // Check if parens/brackets are balanced
            if !is_balanced(&self.buffer) {
                self.multiline = true;
                continue;
            }

            self.multiline = false;
            let input = std::mem::take(&mut self.buffer);

            // Skip empty input
            if input.trim().is_empty() {
                continue;
            }

            // Eval
            match self.eval_input(&input) {
                Ok(val) => {
                    println!("{}", print_value(&val));
                }
                Err(e) => {
                    eprintln!("{}", e.display_with_stack());
                    self.lisp.evaluator.clear_call_stack();
                }
            }
        }

        Ok(())
    }

    fn eval_input(&mut self, input: &str) -> Result<Value, EvalError> {
        let forms = self.lisp.evaluator.read_cached(input)?;
        vm::compile_and_run(&mut self.lisp.evaluator, &forms)
    }
}

impl Default for Repl {
    fn default() -> Self {
        Self::new()
    }
}

fn is_balanced(input: &str) -> bool {
    let mut depth_paren = 0i32;
    let mut depth_bracket = 0i32;
    let mut depth_brace = 0i32;
    let mut in_string = false;
    let mut in_comment = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
            continue;
        }
        if in_string {
            if ch == '\\' {
                chars.next(); // skip escaped char
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            ';' => in_comment = true,
            '"' => in_string = true,
            '(' => depth_paren += 1,
            ')' => depth_paren -= 1,
            '[' => depth_bracket += 1,
            ']' => depth_bracket -= 1,
            '{' => depth_brace += 1,
            '}' => depth_brace -= 1,
            '#' => {
                if chars.peek() == Some(&'_') {
                    chars.next(); // skip reader dispatch
                }
            }
            '\'' | '`' => {
                // quote/quasiquote — don't affect depth
            }
            _ => {}
        }
        if depth_paren < 0 || depth_bracket < 0 || depth_brace < 0 {
            return false;
        }
    }

    !in_string && depth_paren == 0 && depth_bracket == 0 && depth_brace == 0
}

fn print_value(val: &Value) -> String {
    match val {
        Value::Nil => "nil".to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => format!("\"{}\"", s),
        Value::Char(c) => format!("\\{}", c),
        Value::Keyword(kw) => {
            if let Some(ref ns) = kw.ns {
                format!(":{}/{}", ns, kw.name)
            } else {
                format!(":{}", kw.name)
            }
        }
        Value::Symbol(sym) => {
            if let Some(ref ns) = sym.ns {
                format!("{}/{}", ns, sym.name)
            } else {
                sym.name.to_string()
            }
        }
        Value::List(items) => {
            let parts: Vec<String> = items.iter().map(print_value).collect();
            format!("({})", parts.join(" "))
        }
        Value::Vector(items) => {
            let parts: Vec<String> = items.iter().map(print_value).collect();
            format!("[{}]", parts.join(" "))
        }
        Value::Map(m) => {
            let parts: Vec<String> = m
                .iter()
                .map(|(k, v)| format!("{} {}", print_value(k), print_value(v)))
                .collect();
            format!("{{{}}}", parts.join(", "))
        }
        Value::Set(items) => {
            let parts: Vec<String> = items.iter().map(print_value).collect();
            format!("#{{{}}}", parts.join(" "))
        }
        Value::Fn(f) => {
            let name = f.name.as_deref().unwrap_or("<fn>");
            format!("#<fn {}>", name)
        }
        Value::Native(_) => "#<native-fn>".to_string(),
        Value::Macro(m) => {
            format!("#<macro {}>", m.name.as_deref().unwrap_or("<anon>"))
        }
        Value::Atom(_) => "#<atom>".to_string(),
        other => format!("{:?}", other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_balanced() {
        assert!(is_balanced("(+ 1 2)"));
        assert!(!is_balanced("(+ 1 2"));
        assert!(is_balanced("(defn f [x] (+ x 1))"));
        assert!(!is_balanced("(defn f [x] (+ x 1)"));
        assert!(is_balanced("\"hello)\""));
        assert!(is_balanced("(do (println 1) (println 2))"));
        assert!(!is_balanced("(do (println 1) (println 2)"));
    }

    #[test]
    fn test_print_value() {
        assert_eq!(print_value(&Value::Nil), "nil");
        assert_eq!(print_value(&Value::Int(42)), "42");
        assert_eq!(print_value(&Value::Bool(true)), "true");
        assert_eq!(print_value(&Value::string("hello")), "\"hello\"");
    }

    #[test]
    fn test_repl_eval() {
        let mut repl = Repl::new();
        let result = repl.eval_input("(+ 1 2)").unwrap();
        assert_eq!(result, Value::Int(3));
    }
}
