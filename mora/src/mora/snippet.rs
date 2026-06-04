/// A single snippet definition.
#[derive(Debug, Clone)]
pub struct Snippet {
    /// Trigger key (what the user types to activate)
    pub trigger: String,
    /// Human-readable description
    pub description: String,
    /// Body with placeholder markers
    /// Placeholders: $1, $2, ${1:default}, $0 (final cursor)
    pub body: String,
    /// File type filter (e.g., "rust", "python"), or empty for all
    pub file_type: String,
}

/// A parsed placeholder in a snippet body.
#[derive(Debug, Clone)]
pub struct Placeholder {
    /// Tab stop number (0 = final cursor position)
    pub tab_stop: u32,
    /// Default text for this placeholder
    pub default: String,
    /// Byte offset in the expanded body
    pub offset: usize,
    /// Length in the expanded body
    pub length: usize,
}

/// Active snippet expansion state.
pub struct SnippetExpansion {
    /// The fully expanded body text
    pub body: String,
    /// Parsed placeholders sorted by tab stop
    pub placeholders: Vec<Placeholder>,
    /// Current active tab stop index
    pub current_index: usize,
}

/// Snippet registry — manages snippet definitions.
pub struct SnippetEngine {
    snippets: Vec<Snippet>,
}

impl SnippetEngine {
    pub fn new() -> Self {
        Self {
            snippets: Vec::new(),
        }
    }

    /// Register a snippet.
    pub fn register(&mut self, snippet: Snippet) {
        self.snippets.push(snippet);
    }

    /// Find snippets matching the trigger prefix for a file type.
    pub fn find(&self, trigger: &str, file_type: &str) -> Vec<&Snippet> {
        self.snippets
            .iter()
            .filter(|s| {
                s.trigger == trigger && (s.file_type.is_empty() || s.file_type == file_type)
            })
            .collect()
    }

    /// Find a snippet by exact trigger match.
    pub fn find_exact(&self, trigger: &str, file_type: &str) -> Option<&Snippet> {
        self.snippets
            .iter()
            .find(|s| s.trigger == trigger && (s.file_type.is_empty() || s.file_type == file_type))
    }

    /// Expand a snippet body, returning the expansion state.
    pub fn expand(body: &str) -> SnippetExpansion {
        let mut placeholders = Vec::new();
        let mut result = String::new();
        let mut chars = body.chars().peekable();
        let mut pos = 0;

        while let Some(ch) = chars.next() {
            if ch == '$' {
                if let Some(&next) = chars.peek() {
                    if next.is_ascii_digit() {
                        // $N placeholder
                        let mut num_str = String::new();
                        while let Some(&d) = chars.peek() {
                            if d.is_ascii_digit() {
                                num_str.push(d);
                                chars.next();
                            } else {
                                break;
                            }
                        }
                        let tab_stop: u32 = num_str.parse().unwrap_or(0);
                        placeholders.push(Placeholder {
                            tab_stop,
                            default: String::new(),
                            offset: pos,
                            length: 0,
                        });
                        // Empty placeholder — no text inserted
                        continue;
                    } else if next == '{' {
                        // ${N:default} placeholder
                        chars.next(); // consume '{'
                        let mut num_str = String::new();
                        let mut default = String::new();
                        let mut in_default = false;

                        while let Some(c) = chars.next() {
                            if c == '}' {
                                break;
                            } else if c == ':' && !in_default {
                                in_default = true;
                            } else if in_default {
                                default.push(c);
                            } else if c.is_ascii_digit() {
                                num_str.push(c);
                            }
                        }

                        let tab_stop: u32 = num_str.parse().unwrap_or(0);
                        let default_len = default.len();

                        placeholders.push(Placeholder {
                            tab_stop,
                            default: default.clone(),
                            offset: pos,
                            length: default_len,
                        });

                        result.push_str(&default);
                        pos += default_len;
                        continue;
                    }
                }
                // Literal $
                result.push('$');
                pos += 1;
            } else {
                result.push(ch);
                pos += ch.len_utf8();
            }
        }

        // Sort: $0 (final) always goes last, others by number
        placeholders.sort_by(|a, b| match (a.tab_stop == 0, b.tab_stop == 0) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => a.tab_stop.cmp(&b.tab_stop),
        });

        SnippetExpansion {
            body: result,
            placeholders,
            current_index: 0,
        }
    }

    /// Load built-in snippets for common languages.
    pub fn load_defaults(&mut self) {
        // Rust snippets
        self.register(Snippet {
            trigger: "fn".into(),
            description: "function".into(),
            body: "fn ${1:name}(${2:args}) {\n\t$0\n}".into(),
            file_type: "rust".into(),
        });
        self.register(Snippet {
            trigger: "if".into(),
            description: "if block".into(),
            body: "if ${1:condition} {\n\t$0\n}".into(),
            file_type: "rust".into(),
        });
        self.register(Snippet {
            trigger: "for".into(),
            description: "for loop".into(),
            body: "for ${1:item} in ${2:iter} {\n\t$0\n}".into(),
            file_type: "rust".into(),
        });
        self.register(Snippet {
            trigger: "match".into(),
            description: "match expression".into(),
            body: "match ${1:expr} {\n\t${2:pattern} => ${3:value},\n\t_ => $0,\n}".into(),
            file_type: "rust".into(),
        });
        self.register(Snippet {
            trigger: "struct".into(),
            description: "struct definition".into(),
            body: "struct ${1:Name} {\n\t${2:field}: ${3:Type},\n}".into(),
            file_type: "rust".into(),
        });
        self.register(Snippet {
            trigger: "impl".into(),
            description: "impl block".into(),
            body: "impl ${1:Type} {\n\t$0\n}".into(),
            file_type: "rust".into(),
        });
        self.register(Snippet {
            trigger: "test".into(),
            description: "test function".into(),
            body: "#[test]\nfn ${1:test_name}() {\n\t$0\n}".into(),
            file_type: "rust".into(),
        });
        self.register(Snippet {
            trigger: "enum".into(),
            description: "enum definition".into(),
            body: "enum ${1:Name} {\n\t${2:Variant},\n}".into(),
            file_type: "rust".into(),
        });
        self.register(Snippet {
            trigger: "let".into(),
            description: "let binding".into(),
            body: "let ${1:name} = ${2:value};".into(),
            file_type: "rust".into(),
        });
        self.register(Snippet {
            trigger: "macro".into(),
            description: "macro_rules".into(),
            body: "macro_rules! ${1:name} {\n\t($${2:args}) => {\n\t\t$0\n\t};\n}".into(),
            file_type: "rust".into(),
        });

        // Python snippets
        self.register(Snippet {
            trigger: "def".into(),
            description: "function".into(),
            body: "def ${1:name}(${2:args}):\n\t$0".into(),
            file_type: "python".into(),
        });
        self.register(Snippet {
            trigger: "if".into(),
            description: "if block".into(),
            body: "if ${1:condition}:\n\t$0".into(),
            file_type: "python".into(),
        });
        self.register(Snippet {
            trigger: "for".into(),
            description: "for loop".into(),
            body: "for ${1:item} in ${2:iter}:\n\t$0".into(),
            file_type: "python".into(),
        });
        self.register(Snippet {
            trigger: "class".into(),
            description: "class definition".into(),
            body: "class ${1:Name}:\n\tdef __init__(self${2:, args}):\n\t\t$0".into(),
            file_type: "python".into(),
        });
        self.register(Snippet {
            trigger: "try".into(),
            description: "try/except".into(),
            body: "try:\n\t$1\nexcept ${2:Exception} as ${3:e}:\n\t$0".into(),
            file_type: "python".into(),
        });
        self.register(Snippet {
            trigger: "with".into(),
            description: "with statement".into(),
            body: "with ${1:expr} as ${2:var}:\n\t$0".into(),
            file_type: "python".into(),
        });

        // JavaScript/TypeScript snippets
        self.register(Snippet {
            trigger: "fn".into(),
            description: "function".into(),
            body: "function ${1:name}(${2:args}) {\n\t$0\n}".into(),
            file_type: "javascript".into(),
        });
        self.register(Snippet {
            trigger: "af".into(),
            description: "arrow function".into(),
            body: "(${1:args}) => {\n\t$0\n}".into(),
            file_type: "javascript".into(),
        });
        self.register(Snippet {
            trigger: "if".into(),
            description: "if block".into(),
            body: "if (${1:condition}) {\n\t$0\n}".into(),
            file_type: "javascript".into(),
        });
        self.register(Snippet {
            trigger: "for".into(),
            description: "for loop".into(),
            body: "for (let ${1:i} = 0; $1 < ${2:length}; $1++) {\n\t$0\n}".into(),
            file_type: "javascript".into(),
        });

        // Go snippets
        self.register(Snippet {
            trigger: "func".into(),
            description: "function".into(),
            body: "func ${1:name}(${2:args}) {\n\t$0\n}".into(),
            file_type: "go".into(),
        });
        self.register(Snippet {
            trigger: "if".into(),
            description: "if block".into(),
            body: "if ${1:condition} {\n\t$0\n}".into(),
            file_type: "go".into(),
        });
        self.register(Snippet {
            trigger: "for".into(),
            description: "for loop".into(),
            body: "for ${1:i} := 0; $1 < ${2:n}; $1++ {\n\t$0\n}".into(),
            file_type: "go".into(),
        });
        self.register(Snippet {
            trigger: "err".into(),
            description: "error check".into(),
            body: "if err != nil {\n\t$0\n}".into(),
            file_type: "go".into(),
        });

        // C/C++ snippets
        self.register(Snippet {
            trigger: "if".into(),
            description: "if block".into(),
            body: "if (${1:condition}) {\n\t$0\n}".into(),
            file_type: "c".into(),
        });
        self.register(Snippet {
            trigger: "for".into(),
            description: "for loop".into(),
            body: "for (int ${1:i} = 0; $1 < ${2:n}; $1++) {\n\t$0\n}".into(),
            file_type: "c".into(),
        });
        self.register(Snippet {
            trigger: "while".into(),
            description: "while loop".into(),
            body: "while (${1:condition}) {\n\t$0\n}".into(),
            file_type: "c".into(),
        });

        // Common snippets (all languages)
        self.register(Snippet {
            trigger: "todo".into(),
            description: "TODO comment".into(),
            body: "// TODO: $0".into(),
            file_type: "".into(),
        });
        self.register(Snippet {
            trigger: "fixme".into(),
            description: "FIXME comment".into(),
            body: "// FIXME: $0".into(),
            file_type: "".into(),
        });
        self.register(Snippet {
            trigger: "note".into(),
            description: "NOTE comment".into(),
            body: "// NOTE: $0".into(),
            file_type: "".into(),
        });
    }
}

impl SnippetExpansion {
    /// Move to the next tab stop. Returns false if at the end.
    pub fn next(&mut self) -> bool {
        if self.current_index + 1 < self.placeholders.len() {
            self.current_index += 1;
            true
        } else {
            false
        }
    }

    /// Get the current placeholder.
    pub fn current(&self) -> Option<&Placeholder> {
        self.placeholders.get(self.current_index)
    }

    /// Get the current placeholder's tab stop number.
    pub fn current_tab_stop(&self) -> u32 {
        self.current().map(|p| p.tab_stop).unwrap_or(0)
    }

    /// Check if we're at the final tab stop ($0).
    pub fn is_finished(&self) -> bool {
        self.current_tab_stop() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_simple() {
        let exp = SnippetEngine::expand("hello $1 world $2");
        assert_eq!(exp.body, "hello  world ");
        assert_eq!(exp.placeholders.len(), 2);
        assert_eq!(exp.placeholders[0].tab_stop, 1);
        assert_eq!(exp.placeholders[1].tab_stop, 2);
    }

    #[test]
    fn test_expand_with_defaults() {
        let exp = SnippetEngine::expand("fn ${1:name}(${2:args})");
        assert_eq!(exp.body, "fn name(args)");
        assert_eq!(exp.placeholders.len(), 2);
        assert_eq!(exp.placeholders[0].default, "name");
        assert_eq!(exp.placeholders[1].default, "args");
    }

    #[test]
    fn test_snippet_find() {
        let mut engine = SnippetEngine::new();
        engine.register(Snippet {
            trigger: "fn".into(),
            description: "function".into(),
            body: "fn $1()".into(),
            file_type: "rust".into(),
        });
        engine.register(Snippet {
            trigger: "fn".into(),
            description: "function".into(),
            body: "def $1():".into(),
            file_type: "python".into(),
        });

        assert_eq!(engine.find("fn", "rust").len(), 1);
        assert_eq!(engine.find("fn", "python").len(), 1);
        assert_eq!(engine.find("fn", "go").len(), 0);
    }

    #[test]
    fn test_navigation() {
        let mut exp = SnippetEngine::expand("$1, $2, $0");
        assert_eq!(exp.current_tab_stop(), 1);
        assert!(exp.next());
        assert_eq!(exp.current_tab_stop(), 2);
        assert!(exp.next());
        assert_eq!(exp.current_tab_stop(), 0);
        assert!(exp.is_finished());
        assert!(!exp.next());
    }
}
