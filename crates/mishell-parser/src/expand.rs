use crate::ast::*;
use std::collections::HashMap;

pub struct Expander {
    vars: HashMap<String, String>,
    home: Option<String>,
}

impl Expander {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            home: std::env::var("HOME").ok(),
        }
    }

    pub fn with_vars(vars: HashMap<String, String>) -> Self {
        Self {
            vars,
            home: std::env::var("HOME").ok(),
        }
    }

    pub fn set_var(&mut self, name: String, value: String) {
        self.vars.insert(name, value);
    }

    pub fn expand_word(&self, word: &Word) -> String {
        let mut result = String::new();
        for part in &word.parts {
            result.push_str(&self.expand_part(part));
        }
        result
    }

    pub fn expand_part(&self, part: &WordPart) -> String {
        match part {
            WordPart::Literal(s) => s.clone(),
            WordPart::Variable(name) => self
                .vars
                .get(name)
                .cloned()
                .unwrap_or_else(|| std::env::var(name).unwrap_or_default()),
            WordPart::SingleQuoted(s) => s.clone(),
            WordPart::DoubleQuoted(parts) => {
                let mut result = String::new();
                for p in parts {
                    result.push_str(&self.expand_part(p));
                }
                result
            }
            WordPart::Tilde(user) => {
                if let Some(user) = user {
                    format!("~{}", user)
                } else {
                    self.home.clone().unwrap_or_else(|| "~".to_string())
                }
            }
            WordPart::Escape(c) => c.to_string(),
            _ => String::new(),
        }
    }

    pub fn expand_assignment(&self, assign: &Assignment) -> (String, String) {
        (assign.name.clone(), self.expand_word(&assign.value))
    }
}

impl Default for Expander {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word_literal(s: &str) -> Word {
        Word {
            parts: vec![WordPart::Literal(s.to_string())],
        }
    }

    fn word_var(name: &str) -> Word {
        Word {
            parts: vec![WordPart::Variable(name.to_string())],
        }
    }

    fn word_mixed(parts: Vec<WordPart>) -> Word {
        Word { parts }
    }

    #[test]
    fn test_expand_literal() {
        let exp = Expander::new();
        assert_eq!(exp.expand_word(&word_literal("hello")), "hello");
    }

    #[test]
    fn test_expand_variable() {
        let mut exp = Expander::new();
        exp.set_var("NAME".into(), "world".into());
        assert_eq!(exp.expand_word(&word_var("NAME")), "world");
    }

    #[test]
    fn test_expand_undefined_variable() {
        let exp = Expander::new();
        // Undefined vars expand to empty string (or env var)
        let result = exp.expand_word(&word_var("UNDEFINED_VAR_12345"));
        assert_eq!(result, "");
    }

    #[test]
    fn test_expand_mixed_parts() {
        let mut exp = Expander::new();
        exp.set_var("NAME".into(), "world".into());
        let word = word_mixed(vec![
            WordPart::Literal("hello ".to_string()),
            WordPart::Variable("NAME".to_string()),
        ]);
        assert_eq!(exp.expand_word(&word), "hello world");
    }

    #[test]
    fn test_expand_single_quoted() {
        let exp = Expander::new();
        let word = Word {
            parts: vec![WordPart::SingleQuoted("literal $VAR".to_string())],
        };
        assert_eq!(exp.expand_word(&word), "literal $VAR");
    }

    #[test]
    fn test_expand_double_quoted_with_var() {
        let mut exp = Expander::new();
        exp.set_var("X".into(), "42".into());
        let word = Word {
            parts: vec![WordPart::DoubleQuoted(vec![
                WordPart::Literal("value=".to_string()),
                WordPart::Variable("X".to_string()),
            ])],
        };
        assert_eq!(exp.expand_word(&word), "value=42");
    }

    #[test]
    fn test_expand_tilde() {
        let exp = Expander::new();
        let word = Word {
            parts: vec![WordPart::Tilde(None)],
        };
        let result = exp.expand_word(&word);
        // Should expand to home dir or ~
        assert!(!result.is_empty());
    }

    #[test]
    fn test_expand_escape() {
        let exp = Expander::new();
        let word = Word {
            parts: vec![WordPart::Escape('n')],
        };
        assert_eq!(exp.expand_word(&word), "n");
    }

    #[test]
    fn test_expand_assignment() {
        let mut exp = Expander::new();
        exp.set_var("VAL".into(), "test".into());
        let assign = Assignment {
            name: "FOO".to_string(),
            value: word_var("VAL"),
        };
        let (name, value) = exp.expand_assignment(&assign);
        assert_eq!(name, "FOO");
        assert_eq!(value, "test");
    }

    #[test]
    fn test_with_vars_constructor() {
        let mut vars = HashMap::new();
        vars.insert("A".to_string(), "1".to_string());
        vars.insert("B".to_string(), "2".to_string());
        let exp = Expander::with_vars(vars);
        assert_eq!(exp.expand_word(&word_var("A")), "1");
        assert_eq!(exp.expand_word(&word_var("B")), "2");
    }

    #[test]
    fn test_set_var_overwrite() {
        let mut exp = Expander::new();
        exp.set_var("X".into(), "first".into());
        assert_eq!(exp.expand_word(&word_var("X")), "first");
        exp.set_var("X".into(), "second".into());
        assert_eq!(exp.expand_word(&word_var("X")), "second");
    }

    #[test]
    fn test_expand_empty_word() {
        let exp = Expander::new();
        let word = Word { parts: vec![] };
        assert_eq!(exp.expand_word(&word), "");
    }

    #[test]
    fn test_expand_multiple_variables() {
        let mut exp = Expander::new();
        exp.set_var("A".into(), "1".into());
        exp.set_var("B".into(), "2".into());
        exp.set_var("C".into(), "3".into());
        let word = word_mixed(vec![
            WordPart::Variable("A".to_string()),
            WordPart::Literal("-".to_string()),
            WordPart::Variable("B".to_string()),
            WordPart::Literal("-".to_string()),
            WordPart::Variable("C".to_string()),
        ]);
        assert_eq!(exp.expand_word(&word), "1-2-3");
    }

    #[test]
    fn test_expand_double_quoted_literal_only() {
        let exp = Expander::new();
        let word = Word {
            parts: vec![WordPart::DoubleQuoted(vec![WordPart::Literal(
                "just text".to_string(),
            )])],
        };
        assert_eq!(exp.expand_word(&word), "just text");
    }
}
