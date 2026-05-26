//! Tree-sitter integration for AST parsing, syntax queries, and syntax highlighting.
//!
//! Provides fast parsing of source code into syntax trees, symbol queries,
//! and syntax highlighting via tree-sitter queries.

pub mod highlight;

use std::path::Path;

use serde::Serialize;
use tree_sitter::{Language, Parser, Query, QueryCursor, Tree};

/// Language identifier (shared with LSP)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageId {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Bash,
    C,
}

impl LanguageId {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Self::Rust),
            "ts" | "tsx" => Some(Self::TypeScript),
            "js" | "jsx" | "mjs" | "cjs" => Some(Self::JavaScript),
            "py" | "pyw" => Some(Self::Python),
            "go" => Some(Self::Go),
            "sh" | "bash" | "zsh" | "fish" => Some(Self::Bash),
            "c" | "h" | "cpp" | "hpp" | "cc" | "cxx" => Some(Self::C),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::TypeScript => "TypeScript",
            Self::JavaScript => "JavaScript",
            Self::Python => "Python",
            Self::Go => "Go",
            Self::Bash => "Bash",
            Self::C => "C",
        }
    }
}

pub fn language_for_id(id: LanguageId) -> Language {
    match id {
        LanguageId::Rust => tree_sitter_rust::language(),
        LanguageId::JavaScript | LanguageId::TypeScript => tree_sitter_javascript::language(),
        LanguageId::Python => tree_sitter_python::language(),
        LanguageId::Go => tree_sitter_go::language(),
        LanguageId::Bash => tree_sitter_bash::language(),
        LanguageId::C => tree_sitter_c::language(),
    }
}

/// Parser manager for supported languages.
pub struct ParserManager;

impl ParserManager {
    pub fn new() -> Self {
        Self
    }

    /// Parse a file into a syntax tree.
    pub fn parse_file(&self, path: &Path, content: &str) -> anyhow::Result<SyntaxTree> {
        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| anyhow::anyhow!("missing file extension for {}", path.display()))?;

        let lang_id = LanguageId::from_extension(ext)
            .ok_or_else(|| anyhow::anyhow!("unsupported source language: {}", path.display()))?;

        self.parse_source(lang_id, content)
    }

    /// Parse source code with an explicit language id.
    pub fn parse_source(&self, lang_id: LanguageId, content: &str) -> anyhow::Result<SyntaxTree> {
        let language = language_for_id(lang_id);
        let mut parser = Parser::new();
        parser.set_language(&language)?;
        let tree = parser
            .parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse"))?;
        Ok(SyntaxTree { tree, language })
    }

    /// Query function definitions in a syntax tree.
    pub fn query_functions(
        &self,
        tree: &SyntaxTree,
        content: &str,
    ) -> anyhow::Result<Vec<FunctionDef>> {
        let query = Query::new(&tree.language, function_query(&tree.language)?)?;
        let mut cursor = QueryCursor::new();
        let root = tree.tree.root_node();
        let mut functions = Vec::new();

        for m in cursor.matches(&query, root, content.as_bytes()) {
            let mut name = None;
            let mut node = None;
            for capture in m.captures {
                let capture_name = &query.capture_names()[capture.index as usize];
                match *capture_name {
                    "name" => {
                        name = capture
                            .node
                            .utf8_text(content.as_bytes())
                            .ok()
                            .map(str::to_string)
                    }
                    "item" => node = Some(capture.node),
                    _ => {}
                }
            }
            if let (Some(name), Some(node)) = (name, node) {
                functions.push(FunctionDef {
                    name,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                });
            }
        }

        functions.sort_by_key(|f| f.start_line);
        Ok(functions)
    }

    /// Query struct/type definitions in a syntax tree.
    pub fn query_structs(
        &self,
        tree: &SyntaxTree,
        content: &str,
    ) -> anyhow::Result<Vec<StructDef>> {
        let query_src = type_query(&tree.language)?;
        if query_src.is_empty() {
            return Ok(Vec::new());
        }
        let query = Query::new(&tree.language, query_src)?;
        let mut cursor = QueryCursor::new();
        let root = tree.tree.root_node();
        let mut structs = Vec::new();

        for m in cursor.matches(&query, root, content.as_bytes()) {
            let mut name = None;
            let mut node = None;
            for capture in m.captures {
                let capture_name = &query.capture_names()[capture.index as usize];
                match *capture_name {
                    "name" => {
                        name = capture
                            .node
                            .utf8_text(content.as_bytes())
                            .ok()
                            .map(str::to_string)
                    }
                    "item" => node = Some(capture.node),
                    _ => {}
                }
            }
            if let (Some(name), Some(node)) = (name, node) {
                structs.push(StructDef {
                    name,
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                });
            }
        }

        structs.sort_by_key(|s| s.start_line);
        Ok(structs)
    }

    /// Query imports in a syntax tree.
    pub fn query_imports(&self, tree: &SyntaxTree, content: &str) -> anyhow::Result<Vec<Import>> {
        let query_src = import_query(&tree.language)?;
        if query_src.is_empty() {
            return Ok(Vec::new());
        }
        let query = Query::new(&tree.language, query_src)?;
        let mut cursor = QueryCursor::new();
        let root = tree.tree.root_node();
        let mut imports = Vec::new();

        for m in cursor.matches(&query, root, content.as_bytes()) {
            for capture in m.captures {
                if query.capture_names()[capture.index as usize] == "source" {
                    imports.push(Import {
                        source: capture
                            .node
                            .utf8_text(content.as_bytes())
                            .unwrap_or_default()
                            .trim_matches(['\'', '"'])
                            .to_string(),
                        start_line: capture.node.start_position().row + 1,
                    });
                }
            }
        }

        imports.sort_by_key(|i| i.start_line);
        Ok(imports)
    }
}

impl Default for ParserManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Syntax tree wrapper.
pub struct SyntaxTree {
    tree: Tree,
    language: Language,
}

/// Function definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FunctionDef {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
}

/// Struct/type definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StructDef {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
}

/// Import statement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Import {
    pub source: String,
    pub start_line: usize,
}

fn function_query(language: &Language) -> anyhow::Result<&'static str> {
    if *language == tree_sitter_rust::language() {
        Ok(r#"
            (function_item name: (identifier) @name) @item
        "#)
    } else if *language == tree_sitter_javascript::language() {
        Ok(r#"
            (function_declaration name: (identifier) @name) @item
            (method_definition name: (property_identifier) @name) @item
            (lexical_declaration (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression)])) @item
        "#)
    } else if *language == tree_sitter_python::language() {
        Ok(r#"
            (function_definition name: (identifier) @name) @item
        "#)
    } else if *language == tree_sitter_go::language() {
        Ok(r#"
            (function_declaration name: (identifier) @name) @item
            (method_declaration name: (field_identifier) @name) @item
        "#)
    } else {
        anyhow::bail!("unsupported language")
    }
}

fn type_query(language: &Language) -> anyhow::Result<&'static str> {
    if *language == tree_sitter_rust::language() {
        Ok(r#"
            (struct_item name: (type_identifier) @name) @item
            (enum_item name: (type_identifier) @name) @item
            (trait_item name: (type_identifier) @name) @item
            (impl_item type: (_) @name) @item
        "#)
    } else if *language == tree_sitter_javascript::language() {
        Ok(r#"
            (class_declaration name: (identifier) @name) @item
        "#)
    } else if *language == tree_sitter_python::language() {
        Ok(r#"
            (class_definition name: (identifier) @name) @item
        "#)
    } else if *language == tree_sitter_go::language() {
        Ok(r#"
            (type_declaration (type_spec name: (type_identifier) @name)) @item
        "#)
    } else {
        Ok("")
    }
}

fn import_query(language: &Language) -> anyhow::Result<&'static str> {
    if *language == tree_sitter_rust::language() {
        Ok(r#"
            (use_declaration argument: (_) @source)
        "#)
    } else if *language == tree_sitter_javascript::language() {
        Ok(r#"
            (import_statement source: (string) @source)
        "#)
    } else if *language == tree_sitter_python::language() {
        Ok(r#"
            (import_statement name: (_) @source)
            (import_from_statement module_name: (_) @source)
        "#)
    } else if *language == tree_sitter_go::language() {
        Ok(r#"
            (import_spec path: (interpreted_string_literal) @source)
        "#)
    } else {
        Ok("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_rust_symbols_and_imports() {
        let manager = ParserManager::new();
        let source = r#"
use std::path::PathBuf;

struct App {
    name: String,
}

fn main() {}

impl App {
    fn new() -> Self { Self { name: String::new() } }
}
"#;
        let tree = manager.parse_file(Path::new("main.rs"), source).unwrap();

        let functions = manager.query_functions(&tree, source).unwrap();
        assert!(functions.iter().any(|f| f.name == "main"));
        assert!(functions.iter().any(|f| f.name == "new"));

        let structs = manager.query_structs(&tree, source).unwrap();
        assert!(structs.iter().any(|s| s.name == "App"));

        let imports = manager.query_imports(&tree, source).unwrap();
        assert_eq!(imports[0].source, "std::path::PathBuf");
    }

    #[test]
    fn queries_python_symbols() {
        let manager = ParserManager::new();
        let source = "import os\nclass Worker:\n    def run(self):\n        pass\n";
        let tree = manager.parse_file(Path::new("worker.py"), source).unwrap();

        let functions = manager.query_functions(&tree, source).unwrap();
        assert!(functions.iter().any(|f| f.name == "run"));

        let structs = manager.query_structs(&tree, source).unwrap();
        assert!(structs.iter().any(|s| s.name == "Worker"));
    }

    #[test]
    fn parses_bash() {
        let manager = ParserManager::new();
        let source = "#!/bin/bash\necho hello\n";
        let tree = manager.parse_source(LanguageId::Bash, source).unwrap();
        assert!(tree.tree.root_node().child_count() > 0);
    }

    #[test]
    fn parses_c() {
        let manager = ParserManager::new();
        let source = "#include <stdio.h>\nint main() { return 0; }\n";
        let tree = manager.parse_source(LanguageId::C, source).unwrap();
        assert!(tree.tree.root_node().child_count() > 0);
    }
}
