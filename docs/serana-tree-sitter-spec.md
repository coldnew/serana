# serana-tree-sitter — AST Parsing & Syntax Queries

## Purpose

In-process code parsing using tree-sitter for fast, offline syntax analysis. Used by the agent for symbol discovery without spinning up LSP servers.

## Dependencies

- `serana-core` (Result), `tree-sitter`, `tree-sitter-rust`, `tree-sitter-javascript`, `tree-sitter-python`, `tree-sitter-go`

## Module Map

Single-file module (`lib.rs`) with:

| Item | Purpose |
|------|---------|
| `ParserManager` | Parse files into syntax trees, run queries |
| `SyntaxTree` | Wrapper around `tree_sitter::Tree` + `Language` |
| `LanguageId` | Enum matching serana-lsp's `LanguageId` (Rust, TypeScript, JavaScript, Python, Go) |
| `FunctionDef` | `{ name, start_line, end_line }` |
| `StructDef` | `{ name, start_line, end_line }` |
| `Import` | `{ source, start_line }` |

## Queries

### Functions

| Language | Query |
|----------|-------|
| Rust | `(function_item name: (identifier) @name)` |
| JS/TS | `(function_declaration ...)`, `(method_definition ...)`, arrow functions |
| Python | `(function_definition name: (identifier) @name)` |
| Go | `(function_declaration ...)`, `(method_declaration ...)` |

### Types (Structs/Classes)

| Language | Query |
|----------|-------|
| Rust | `struct_item`, `enum_item`, `trait_item`, `impl_item` |
| JS/TS | `class_declaration` |
| Python | `class_definition` |
| Go | `type_spec` |

### Imports

| Language | Query |
|----------|-------|
| Rust | `use_declaration` arguments |
| JS/TS | `import_statement` sources |
| Python | `import_statement`, `import_from_statement` module names |
| Go | `import_spec` interpreted string literals |

## Design Decisions

- **Same `LanguageId` as serana-lsp**: Consistent extension-to-language mapping across both crates.
- **Stateless `ParserManager`**: Creates a fresh `Parser` per parse. Acceptable for occasional code queries (not hot-path).
- **No incremental parsing**: Full re-parse each call. Tree-sitter is fast enough (~MB/s) for file-level queries.
- **Queries are &'static str**: Returned by match on `Language` pointer comparison. Simple, no config files.
- **`SyntaxTree` does not expose raw `Tree`**: Keeps internal `Language` alongside for query methods.
