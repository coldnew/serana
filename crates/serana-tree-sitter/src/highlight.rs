use crate::{language_for_id, LanguageId};
use tree_sitter::{Parser, Query, QueryCursor};

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
    Property,
    Variable,
    Constant,
    Heading,
    Bold,
    Italic,
    Link,
    Code,
    ListMarker,
    Blockquote,
    HorizontalRule,
    Tag,
    Attribute,
}

#[derive(Debug, Clone)]
pub struct HighlightToken {
    pub start: usize,
    pub end: usize,
    pub kind: HighlightKind,
}

fn query_tokens_for_lang(
    language: &tree_sitter::Language,
    tree: &tree_sitter::Tree,
    content: &str,
    query_src: &str,
) -> Vec<Vec<HighlightToken>> {
    let line_count = content.lines().count().max(1);
    let mut result: Vec<Vec<HighlightToken>> = vec![Vec::new(); line_count];

    let query = match Query::new(language, query_src) {
        Ok(q) => q,
        Err(_) => return result,
    };

    let mut cursor = QueryCursor::new();
    let root = tree.root_node();
    let byte_content = content.as_bytes();

    for m in cursor.matches(&query, root, byte_content) {
        for capture in m.captures {
            let name = &query.capture_names()[capture.index as usize];
            let kind = capture_to_kind(name);
            if kind == HighlightKind::Normal {
                continue;
            }

            let node = capture.node;
            let sp = node.start_position();
            let ep = node.end_position();

            if sp.row == ep.row {
                if sp.row < result.len() {
                    result[sp.row].push(HighlightToken {
                        start: sp.column,
                        end: ep.column,
                        kind,
                    });
                }
            } else {
                for row in sp.row..=ep.row.min(line_count.saturating_sub(1)) {
                    let tok_start = if row == sp.row { sp.column } else { 0 };
                    let lines: Vec<&str> = content.lines().collect();
                    let tok_end = if row == ep.row {
                        ep.column
                    } else {
                        lines.get(row).map_or(0, |l| l.len())
                    };
                    result[row].push(HighlightToken {
                        start: tok_start,
                        end: tok_end,
                        kind,
                    });
                }
            }
        }
    }

    result
}

pub fn highlight_document(lang_id: LanguageId, content: &str) -> Vec<Vec<HighlightToken>> {
    let lines: Vec<&str> = content.lines().collect();
    let line_count = lines.len().max(1);
    let empty: Vec<Vec<HighlightToken>> = vec![Vec::new(); line_count];

    let language = language_for_id(lang_id);
    let mut parser = Parser::new();
    if parser.set_language(&language).is_err() {
        return empty;
    }
    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => return empty,
    };

    let query_srcs = match query_sources_for_lang(lang_id) {
        Some(qs) => qs,
        None => return empty,
    };

    let mut result = empty;

    for qs in &query_srcs {
        let tokens = query_tokens_for_lang(&language, &tree, content, qs);
        for (row, line_tokens) in tokens.into_iter().enumerate() {
            if row < result.len() {
                result[row].extend(line_tokens);
            }
        }
    }

    let kw_tokens = highlight_keywords_range(content, lang_id);
    for (row, line_tokens) in kw_tokens.into_iter().enumerate() {
        if row < result.len() {
            result[row].extend(line_tokens);
        }
    }

    for line in &mut result {
        line.sort_by_key(|t| t.start);
    }

    result
}

fn keyword_set_for_lang(lang_id: LanguageId) -> &'static [(&'static str, HighlightKind)] {
    match lang_id {
        LanguageId::Rust => &RUST_KEYWORD_SET,
        LanguageId::Python => &PYTHON_KEYWORD_SET,
        LanguageId::JavaScript | LanguageId::TypeScript => &JS_KEYWORD_SET,
        LanguageId::Go => &GO_KEYWORD_SET,
        LanguageId::Bash => &BASH_KEYWORD_SET,
        LanguageId::C => &C_KEYWORD_SET,
    }
}

fn highlight_keywords_range(content: &str, lang_id: LanguageId) -> Vec<Vec<HighlightToken>> {
    let line_count = content.lines().count().max(1);
    let mut result: Vec<Vec<HighlightToken>> = vec![Vec::new(); line_count];
    let keywords = keyword_set_for_lang(lang_id);

    for (row, line) in content.lines().enumerate() {
        if row >= result.len() {
            break;
        }
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
                let start = i;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'!')
                {
                    i += 1;
                }
                let word = &line[start..i];
                for &(kw, kind) in keywords {
                    if word == kw {
                        result[row].push(HighlightToken {
                            start,
                            end: i,
                            kind,
                        });
                        break;
                    }
                }
            } else {
                i += 1;
            }
        }
    }

    result
}

fn query_sources_for_lang(lang_id: LanguageId) -> Option<Vec<&'static str>> {
    match lang_id {
        LanguageId::Rust => Some(vec![RUST_NAMED_HIGHLIGHTS]),
        LanguageId::JavaScript | LanguageId::TypeScript => Some(vec![JS_NAMED_HIGHLIGHTS]),
        LanguageId::Python => Some(vec![PYTHON_NAMED_HIGHLIGHTS]),
        LanguageId::Go => Some(vec![GO_NAMED_HIGHLIGHTS]),
        LanguageId::Bash => Some(vec![BASH_NAMED_HIGHLIGHTS]),
        LanguageId::C => Some(vec![C_NAMED_HIGHLIGHTS]),
    }
}

// Keywords are matched using simple word scanning instead of tree-sitter queries
// because tree-sitter 0.22 has a query compilation bug where string literal patterns
// don't work correctly in multi-pattern queries.

const RUST_KEYWORD_SET: &[(&str, HighlightKind)] = &[
    ("as", HighlightKind::Keyword),
    ("async", HighlightKind::Keyword),
    ("await", HighlightKind::Keyword),
    ("break", HighlightKind::Keyword),
    ("const", HighlightKind::Keyword),
    ("continue", HighlightKind::Keyword),
    ("crate", HighlightKind::Keyword),
    ("default", HighlightKind::Keyword),
    ("dyn", HighlightKind::Keyword),
    ("else", HighlightKind::Keyword),
    ("enum", HighlightKind::Keyword),
    ("extern", HighlightKind::Keyword),
    ("fn", HighlightKind::Keyword),
    ("for", HighlightKind::Keyword),
    ("if", HighlightKind::Keyword),
    ("impl", HighlightKind::Keyword),
    ("in", HighlightKind::Keyword),
    ("let", HighlightKind::Keyword),
    ("loop", HighlightKind::Keyword),
    ("match", HighlightKind::Keyword),
    ("mod", HighlightKind::Keyword),
    ("move", HighlightKind::Keyword),
    ("mut", HighlightKind::Keyword),
    ("pub", HighlightKind::Keyword),
    ("ref", HighlightKind::Keyword),
    ("return", HighlightKind::Keyword),
    ("self", HighlightKind::Keyword),
    ("static", HighlightKind::Keyword),
    ("struct", HighlightKind::Keyword),
    ("super", HighlightKind::Keyword),
    ("trait", HighlightKind::Keyword),
    ("try", HighlightKind::Keyword),
    ("type", HighlightKind::Keyword),
    ("union", HighlightKind::Keyword),
    ("unsafe", HighlightKind::Keyword),
    ("use", HighlightKind::Keyword),
    ("where", HighlightKind::Keyword),
    ("while", HighlightKind::Keyword),
    ("yield", HighlightKind::Keyword),
    ("true", HighlightKind::Constant),
    ("false", HighlightKind::Constant),
    ("Some", HighlightKind::Type),
    ("None", HighlightKind::Constant),
    ("Ok", HighlightKind::Type),
    ("Err", HighlightKind::Type),
];

const PYTHON_KEYWORD_SET: &[(&str, HighlightKind)] = &[
    ("def", HighlightKind::Keyword),
    ("class", HighlightKind::Keyword),
    ("if", HighlightKind::Keyword),
    ("elif", HighlightKind::Keyword),
    ("else", HighlightKind::Keyword),
    ("for", HighlightKind::Keyword),
    ("while", HighlightKind::Keyword),
    ("return", HighlightKind::Keyword),
    ("import", HighlightKind::Keyword),
    ("from", HighlightKind::Keyword),
    ("as", HighlightKind::Keyword),
    ("try", HighlightKind::Keyword),
    ("except", HighlightKind::Keyword),
    ("finally", HighlightKind::Keyword),
    ("raise", HighlightKind::Keyword),
    ("with", HighlightKind::Keyword),
    ("yield", HighlightKind::Keyword),
    ("lambda", HighlightKind::Keyword),
    ("pass", HighlightKind::Keyword),
    ("break", HighlightKind::Keyword),
    ("continue", HighlightKind::Keyword),
    ("and", HighlightKind::Keyword),
    ("or", HighlightKind::Keyword),
    ("not", HighlightKind::Keyword),
    ("in", HighlightKind::Keyword),
    ("is", HighlightKind::Keyword),
    ("async", HighlightKind::Keyword),
    ("await", HighlightKind::Keyword),
    ("global", HighlightKind::Keyword),
    ("nonlocal", HighlightKind::Keyword),
    ("assert", HighlightKind::Keyword),
    ("del", HighlightKind::Keyword),
    ("True", HighlightKind::Constant),
    ("False", HighlightKind::Constant),
    ("None", HighlightKind::Constant),
    ("self", HighlightKind::Variable),
];

const JS_KEYWORD_SET: &[(&str, HighlightKind)] = &[
    ("var", HighlightKind::Keyword),
    ("let", HighlightKind::Keyword),
    ("const", HighlightKind::Keyword),
    ("function", HighlightKind::Keyword),
    ("return", HighlightKind::Keyword),
    ("if", HighlightKind::Keyword),
    ("else", HighlightKind::Keyword),
    ("for", HighlightKind::Keyword),
    ("while", HighlightKind::Keyword),
    ("do", HighlightKind::Keyword),
    ("switch", HighlightKind::Keyword),
    ("case", HighlightKind::Keyword),
    ("break", HighlightKind::Keyword),
    ("continue", HighlightKind::Keyword),
    ("new", HighlightKind::Keyword),
    ("class", HighlightKind::Keyword),
    ("extends", HighlightKind::Keyword),
    ("import", HighlightKind::Keyword),
    ("from", HighlightKind::Keyword),
    ("export", HighlightKind::Keyword),
    ("default", HighlightKind::Keyword),
    ("try", HighlightKind::Keyword),
    ("catch", HighlightKind::Keyword),
    ("finally", HighlightKind::Keyword),
    ("throw", HighlightKind::Keyword),
    ("async", HighlightKind::Keyword),
    ("await", HighlightKind::Keyword),
    ("yield", HighlightKind::Keyword),
    ("of", HighlightKind::Keyword),
    ("in", HighlightKind::Keyword),
    ("typeof", HighlightKind::Keyword),
    ("instanceof", HighlightKind::Keyword),
    ("void", HighlightKind::Keyword),
    ("delete", HighlightKind::Keyword),
    ("this", HighlightKind::Variable),
    ("null", HighlightKind::Constant),
    ("undefined", HighlightKind::Constant),
    ("true", HighlightKind::Constant),
    ("false", HighlightKind::Constant),
    ("super", HighlightKind::Keyword),
];

const GO_KEYWORD_SET: &[(&str, HighlightKind)] = &[
    ("func", HighlightKind::Keyword),
    ("var", HighlightKind::Keyword),
    ("const", HighlightKind::Keyword),
    ("type", HighlightKind::Keyword),
    ("struct", HighlightKind::Keyword),
    ("interface", HighlightKind::Keyword),
    ("map", HighlightKind::Keyword),
    ("chan", HighlightKind::Keyword),
    ("package", HighlightKind::Keyword),
    ("import", HighlightKind::Keyword),
    ("return", HighlightKind::Keyword),
    ("if", HighlightKind::Keyword),
    ("else", HighlightKind::Keyword),
    ("for", HighlightKind::Keyword),
    ("range", HighlightKind::Keyword),
    ("switch", HighlightKind::Keyword),
    ("case", HighlightKind::Keyword),
    ("default", HighlightKind::Keyword),
    ("break", HighlightKind::Keyword),
    ("continue", HighlightKind::Keyword),
    ("go", HighlightKind::Keyword),
    ("defer", HighlightKind::Keyword),
    ("select", HighlightKind::Keyword),
    ("fallthrough", HighlightKind::Keyword),
    ("nil", HighlightKind::Constant),
    ("true", HighlightKind::Constant),
    ("false", HighlightKind::Constant),
    ("iota", HighlightKind::Constant),
];

const BASH_KEYWORD_SET: &[(&str, HighlightKind)] = &[
    ("if", HighlightKind::Keyword),
    ("then", HighlightKind::Keyword),
    ("else", HighlightKind::Keyword),
    ("elif", HighlightKind::Keyword),
    ("fi", HighlightKind::Keyword),
    ("for", HighlightKind::Keyword),
    ("while", HighlightKind::Keyword),
    ("do", HighlightKind::Keyword),
    ("done", HighlightKind::Keyword),
    ("case", HighlightKind::Keyword),
    ("esac", HighlightKind::Keyword),
    ("function", HighlightKind::Keyword),
    ("return", HighlightKind::Keyword),
    ("local", HighlightKind::Keyword),
    ("export", HighlightKind::Keyword),
    ("source", HighlightKind::Keyword),
    ("alias", HighlightKind::Keyword),
    ("unset", HighlightKind::Keyword),
    ("shift", HighlightKind::Keyword),
    ("exit", HighlightKind::Keyword),
    ("in", HighlightKind::Keyword),
    ("true", HighlightKind::Constant),
    ("false", HighlightKind::Constant),
];

const C_KEYWORD_SET: &[(&str, HighlightKind)] = &[
    ("if", HighlightKind::Keyword),
    ("else", HighlightKind::Keyword),
    ("for", HighlightKind::Keyword),
    ("while", HighlightKind::Keyword),
    ("do", HighlightKind::Keyword),
    ("switch", HighlightKind::Keyword),
    ("case", HighlightKind::Keyword),
    ("default", HighlightKind::Keyword),
    ("break", HighlightKind::Keyword),
    ("continue", HighlightKind::Keyword),
    ("return", HighlightKind::Keyword),
    ("goto", HighlightKind::Keyword),
    ("struct", HighlightKind::Keyword),
    ("union", HighlightKind::Keyword),
    ("enum", HighlightKind::Keyword),
    ("typedef", HighlightKind::Keyword),
    ("sizeof", HighlightKind::Keyword),
    ("extern", HighlightKind::Keyword),
    ("static", HighlightKind::Keyword),
    ("const", HighlightKind::Keyword),
    ("volatile", HighlightKind::Keyword),
    ("restrict", HighlightKind::Keyword),
    ("inline", HighlightKind::Keyword),
    ("register", HighlightKind::Keyword),
    ("auto", HighlightKind::Keyword),
    ("NULL", HighlightKind::Constant),
    ("true", HighlightKind::Constant),
    ("false", HighlightKind::Constant),
];

const RUST_NAMED_HIGHLIGHTS: &str = r#"
; Named tokens for keywords defined as named nodes
(mutable_specifier) @keyword
(crate) @keyword
(self) @keyword
(super) @keyword

(line_comment) @comment
(block_comment) @comment

(string_literal) @string
(char_literal) @string

(integer_literal) @number
(float_literal) @number

(call_expression
  function: (identifier) @function)

(call_expression
  function: (field_expression
    field: (field_identifier) @function))

(macro_invocation
  macro: (identifier) @function)

(type_identifier) @type
(primitive_type) @type

(field_identifier) @property

(attribute_item) @attribute
"#;

const PYTHON_NAMED_HIGHLIGHTS: &str = r#"
(comment) @comment

(string) @string

(integer) @number
(float) @number

(call
  function: (identifier) @function)

(class_definition
  name: (identifier) @type)

(decorated_definition
  (decorator) @attribute)
"#;

const JS_NAMED_HIGHLIGHTS: &str = r#"
(comment) @comment

(string) @string
(template_string) @string

(number) @number

(call_expression
  function: (identifier) @function)

(call_expression
  function: (member_expression
    property: (property_identifier) @function))

(class_declaration
  name: (identifier) @type)

(property_identifier) @property
"#;

const GO_NAMED_HIGHLIGHTS: &str = r#"
(comment) @comment

(interpreted_string_literal) @string
(raw_string_literal) @string
(rune_literal) @string

(int_literal) @number
(float_literal) @number

(call_expression
  function: (identifier) @function)

(call_expression
  function: (selector_expression
    field: (field_identifier) @function))

(function_declaration
  name: (identifier) @function)

(method_declaration
  name: (field_identifier) @function)

(type_identifier) @type

(field_identifier) @property
"#;

const BASH_NAMED_HIGHLIGHTS: &str = r#"
(comment) @comment

(string) @string
(raw_string) @string
(ansi_c_string) @string

(simple_expansion) @variable
(expansion) @variable
(variable_name) @variable
(special_variable_name) @variable.builtin

(command_name) @function
"#;

const C_NAMED_HIGHLIGHTS: &str = r#"
(comment) @comment

(string_literal) @string

(number_literal) @number

(call_expression
  function: (identifier) @function)

(type_identifier) @type
(primitive_type) @type

(field_identifier) @property
"#;

fn capture_to_kind(name: &str) -> HighlightKind {
    match name {
        "keyword" => HighlightKind::Keyword,
        "string" | "string.special" => HighlightKind::String,
        "comment" => HighlightKind::Comment,
        "number" => HighlightKind::Number,
        "function" | "function.call" | "function.macro" => HighlightKind::Function,
        "type" | "type.builtin" => HighlightKind::Type,
        "operator" => HighlightKind::Operator,
        "bracket" => HighlightKind::Bracket,
        "property" | "field" => HighlightKind::Property,
        "variable" | "variable.builtin" | "variable.parameter" => HighlightKind::Variable,
        "constant" | "constant.builtin" => HighlightKind::Constant,
        "heading" => HighlightKind::Heading,
        "bold" => HighlightKind::Bold,
        "italic" => HighlightKind::Italic,
        "link" => HighlightKind::Link,
        "code" => HighlightKind::Code,
        "list_marker" => HighlightKind::ListMarker,
        "tag" => HighlightKind::Tag,
        "attribute" | "decorator" => HighlightKind::Attribute,
        _ => HighlightKind::Normal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_rust() {
        let tokens = highlight_document(
            LanguageId::Rust,
            "fn main() {\n    // hello\n    let x = 42;\n}\n",
        );
        assert!(tokens.len() >= 4);
        let line0 = &tokens[0];
        assert!(line0.iter().any(|t| t.kind == HighlightKind::Keyword));
        let line1 = &tokens[1];
        assert!(line1.iter().any(|t| t.kind == HighlightKind::Comment));
        let line2 = &tokens[2];
        assert!(line2.iter().any(|t| t.kind == HighlightKind::Number));
    }

    #[test]
    fn highlight_python() {
        let tokens = highlight_document(
            LanguageId::Python,
            "def foo():\n    # comment\n    return 42\n",
        );
        assert!(tokens.len() >= 3);
        assert!(tokens[0].iter().any(|t| t.kind == HighlightKind::Keyword));
        assert!(tokens[1].iter().any(|t| t.kind == HighlightKind::Comment));
        assert!(tokens[2].iter().any(|t| t.kind == HighlightKind::Number));
    }

    #[test]
    fn highlight_javascript() {
        let tokens = highlight_document(
            LanguageId::JavaScript,
            "const x = () => {\n  // hi\n  return 1;\n};\n",
        );
        assert!(tokens[0].iter().any(|t| t.kind == HighlightKind::Keyword));
        assert!(tokens[1].iter().any(|t| t.kind == HighlightKind::Comment));
    }

    #[test]
    fn highlight_go() {
        let tokens = highlight_document(
            LanguageId::Go,
            "package main\n\nfunc main() {\n\t// hello\n}\n",
        );
        assert!(tokens[0].iter().any(|t| t.kind == HighlightKind::Keyword));
        assert!(tokens[2].iter().any(|t| t.kind == HighlightKind::Function));
    }

    #[test]
    fn highlight_bash() {
        let tokens = highlight_document(
            LanguageId::Bash,
            "#!/bin/bash\nif true; then\n  echo hello\nfi\n",
        );
        assert!(
            tokens[1].iter().any(|t| t.kind == HighlightKind::Keyword),
            "line 1 should have keywords, got {:?}",
            tokens[1]
        );
        assert!(
            tokens[2].iter().any(|t| t.kind == HighlightKind::Function),
            "line 2 should have command name function, got {:?}",
            tokens[2]
        );
    }

    #[test]
    fn highlight_c() {
        let tokens = highlight_document(
            LanguageId::C,
            "#include <stdio.h>\nint main() {\n  // hi\n  return 0;\n}\n",
        );
        assert!(
            tokens[3].iter().any(|t| t.kind == HighlightKind::Keyword),
            "line 3 should have 'return' keyword, got {:?}",
            tokens[3]
        );
        assert!(tokens[2].iter().any(|t| t.kind == HighlightKind::Comment));
    }

    #[test]
    fn highlight_empty() {
        let tokens = highlight_document(LanguageId::Rust, "");
        assert_eq!(tokens.len(), 1);
        assert!(tokens[0].is_empty());
    }

    #[test]
    fn debug_rust_parse() {
        let language = language_for_id(LanguageId::Rust);
        let mut parser = Parser::new();
        parser.set_language(&language).unwrap();
        let tree = parser.parse("fn main() {}", None).unwrap();
        let sexp = tree.root_node().to_sexp();
        assert!(sexp.contains("function_item"), "AST: {}", sexp);
    }

    #[test]
    fn validate_keyword_queries() {
        // Verify that all NAMED_HIGHLIGHTS queries parse correctly and produce matches
        let cases = [
            (LanguageId::Rust, RUST_NAMED_HIGHLIGHTS, "Rust"),
            (LanguageId::Python, PYTHON_NAMED_HIGHLIGHTS, "Python"),
            (LanguageId::JavaScript, JS_NAMED_HIGHLIGHTS, "JS"),
            (LanguageId::Go, GO_NAMED_HIGHLIGHTS, "Go"),
            (LanguageId::Bash, BASH_NAMED_HIGHLIGHTS, "Bash"),
            (LanguageId::C, C_NAMED_HIGHLIGHTS, "C"),
        ];
        for (lang_id, qs, name) in &cases {
            let language = language_for_id(*lang_id);
            match Query::new(&language, qs) {
                Ok(q) => eprintln!(
                    "{} NAMED_HIGHLIGHTS OK: {} captures",
                    name,
                    q.capture_names().len()
                ),
                Err(e) => panic!("{} NAMED_HIGHLIGHTS FAILED: {:?}", name, e),
            }
        }

        // Verify keyword sets exist and have entries
        let all_sets: [(&str, &[(&str, HighlightKind)]); 6] = [
            ("Rust", RUST_KEYWORD_SET),
            ("Python", PYTHON_KEYWORD_SET),
            ("JS", JS_KEYWORD_SET),
            ("Go", GO_KEYWORD_SET),
            ("Bash", BASH_KEYWORD_SET),
            ("C", C_KEYWORD_SET),
        ];
        for (name, set) in &all_sets {
            assert!(!set.is_empty(), "{} keyword set should not be empty", name);
            eprintln!("{} keywords: {} entries", name, set.len());
        }
    }
}
