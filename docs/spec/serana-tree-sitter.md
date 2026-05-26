# serana-tree-sitter — AST Parsing & Syntax Queries

## Overview

`serana-tree-sitter` provides in-process code parsing using tree-sitter for fast, offline syntax analysis. Used by the agent for symbol discovery without spinning up LSP servers.

**Crate path:** `crates/serana-tree-sitter/`

## Dependencies

- **Internal:** `serana-core` (for `Result`)
- **External:** `tree-sitter`, `tree-sitter-rust`, `tree-sitter-javascript`, `tree-sitter-python`, `tree-sitter-go`, `anyhow`, `serde`

## Module Structure

Single-file module (`lib.rs`) containing all types and logic.

## Types

### `LanguageId`

```rust
pub enum LanguageId {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
}
```

- `from_extension(ext: &str) -> Option<LanguageId>` — maps file extension to language
- Matches `serana-lsp::LanguageId` for consistency across both code intelligence systems

### `ParserManager`

Stateless parser that creates a fresh `tree_sitter::Parser` per parse.

```rust
pub struct ParserManager;
```

| Method | Returns | Purpose |
|--------|---------|---------|
| `parse_file(path: &Path, content: &str)` | `Result<SyntaxTree>` | Parse file content into AST |
| `query_functions(tree: &SyntaxTree)` | `Result<Vec<FunctionDef>>` | Extract function definitions |
| `query_structs(tree: &SyntaxTree)` | `Result<Vec<StructDef>>` | Extract struct/class/type definitions |
| `query_imports(tree: &SyntaxTree)` | `Result<Vec<Import>>` | Extract import statements |

### `SyntaxTree`

Wrapper around tree-sitter's `Tree` + `Language`.

```rust
pub struct SyntaxTree {
    tree: tree_sitter::Tree,
    language: tree_sitter::Language,
}
```

- Holds both the parsed tree and the language for query execution
- Does not expose raw `Tree` — keeps `Language` alongside for query methods

### `FunctionDef`

```rust
pub struct FunctionDef {
    pub name: String,
    pub start_line: usize,  // 0-indexed
    pub end_line: usize,    // 0-indexed (exclusive)
}
```

### `StructDef`

```rust
pub struct StructDef {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
}
```

### `Import`

```rust
pub struct Import {
    pub source: String,     // the imported module/path
    pub start_line: usize,
}
```

## Tree-sitter Queries

### Function Queries

| Language | Query Pattern |
|----------|---------------|
| Rust | `(function_item name: (identifier) @name)` |
| JavaScript/TypeScript | `(function_declaration name: (identifier) @name)`, `(method_definition name: (property_identifier) @name)`, arrow functions |
| Python | `(function_definition name: (identifier) @name)` |
| Go | `(function_declaration name: (identifier) @name)`, `(method_declaration name: (field_identifier) @name)` |

### Type/Struct Queries

| Language | Query Pattern |
|----------|---------------|
| Rust | `(struct_item name: (type_identifier) @name)`, `(enum_item ...)`, `(trait_item ...)`, `(impl_item ...)` |
| JavaScript/TypeScript | `(class_declaration name: (identifier) @name)` |
| Python | `(class_definition name: (identifier) @name)` |
| Go | `(type_spec name: (type_identifier) @name)` |

### Import Queries

| Language | Query Pattern |
|----------|---------------|
| Rust | `(use_argument (identifier) @name)` — extracts `use` paths |
| JavaScript/TypeScript | `(import_statement source: (string) @source)` |
| Python | `(import_statement name: (dotted_name) @name)`, `(import_from_statement module_name: (dotted_name) @module)` |
| Go | `(import_spec path: (interpreted_string_literal) @path)` |

## Usage Example

```rust
use serana_tree_sitter::ParserManager;
use std::path::Path;

let manager = ParserManager;
let source = r#"
fn main() {
    println!("hello");
}

struct Config {
    name: String,
}
"#;

let tree = manager.parse_file(Path::new("main.rs"), source)?;
let functions = manager.query_functions(&tree)?;
// functions: [FunctionDef { name: "main", start_line: 1, end_line: 3 }]

let structs = manager.query_structs(&tree)?;
// structs: [StructDef { name: "Config", start_line: 5, end_line: 7 }]

let imports = manager.query_imports(&tree)?;
// imports: [] (no imports in this file)
```

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| Same `LanguageId` as serana-lsp | Consistent extension-to-language mapping across both crates |
| Stateless `ParserManager` | Creates fresh `Parser` per parse; acceptable for occasional queries (not hot-path) |
| No incremental parsing | Full re-parse each call; tree-sitter is fast enough (~MB/s) for file-level queries |
| Queries are `&'static str` | Returned by match on `Language` pointer comparison; simple, no config files |
| `SyntaxTree` hides raw `Tree` | Keeps `Language` alongside for query methods; prevents misuse |
| 0-indexed line numbers | Matches tree-sitter's internal representation; no conversion needed |

## Performance Characteristics

- **Parse speed:** ~1-10 MB/s depending on language and file complexity
- **Query speed:** Microseconds for typical queries on parsed trees
- **Memory:** Tree is owned, no references to source text after parse
- **Startup:** First parse per language loads the grammar (~1ms); subsequent parses are faster

## Limitations

- **No incremental parsing:** Full re-parse on every call
- **No error recovery queries:** Tree-sitter's error nodes are present but not specifically handled
- **No cross-file analysis:** Each file parsed independently
- **No rename support:** Queries extract symbols but don't support refactoring
- **Limited language support:** Rust, JS/TS, Python, Go only
