# Spec: Serana - Personal Coding Agent

## Objective

Build a personal coding agent in Rust that:
1. **Phase 1 (Current)**: Functions as a coding agent - understands codebases, edits files, runs commands, answers questions about code
2. **Phase 2 (Future)**: Migrates to Hermes agent architecture

**User**: Single developer (coldnew) for personal productivity
**Success**: Agent can autonomously complete coding tasks with natural language instructions

## Tech Stack

- **Language**: Rust (stable 1.75+)
- **Async Runtime**: tokio
- **LLM Backend**: Pluggable (start with OpenAI-compatible API)
- **Serialization**: serde, serde_json
- **Frontend**: TUI via ratatui (web frontend planned for future)
- **Code Intelligence**: Built-in LSP client + tree-sitter for AST parsing
- **Error Handling**: anyhow/thiserror

## Commands

```bash
# Build
cargo build --release

# Test
cargo test
cargo test -- --nocapture  # with output

# Lint
cargo clippy -- -D warnings
cargo fmt --check

# Run
cargo run                           # Launch TUI
cargo run -- /path/to/project       # Launch TUI with workspace
cargo run -- --help                 # Show CLI options

# Configuration
cargo run -- config                 # Show resolved configuration
cargo run -- config --sample        # Show sample config.toml
```

## Configuration

Serana reads configuration from `~/.serana/config.toml`. If the file doesn't exist, defaults are used.

### Config File Schema

```toml
[provider]
# Built-in providers: openai, anthropic, ollama, openrouter, custom
name = "openai"

# For custom providers, specify the URL:
# name = "custom"
# url = "http://localhost:11434/v1"

[llm]
model = "gpt-4"
temperature = 0.7

# API key is optional - prefer environment variables
# api_key = "sk-..."

[agent]
max_tokens = 4096
```

### Environment Variables

Environment variables take precedence over the config file:

- `SERANA_PROVIDER` - Provider name (openai, anthropic, ollama, openrouter, custom)
- `SERANA_API_KEY` - API key (recommended over config file)
- `SERANA_MODEL` - Model name
- `OPENAI_API_KEY` - Fallback API key for OpenAI

### Provider URLs

| Provider | API URL |
|----------|---------|
| openai | https://api.openai.com/v1 |
| anthropic | https://api.anthropic.com/v1 |
| ollama | http://localhost:11434/v1 |
| openrouter | https://openrouter.ai/api/v1 |
| custom | User-specified via `provider.url` |

## Project Structure

```
serana/
├── Cargo.toml
├── src/
│   ├── main.rs           # CLI entry point
│   ├── lib.rs            # Library root
│   ├── tui/              # TUI frontend (current focus)
│   │   ├── mod.rs        # TUI module root
│   │   ├── app.rs        # Application state machine
│   │   ├── ui.rs         # Widget rendering
│   │   ├── event.rs      # Event handling (keyboard, resize)
│   │   └── components/   # UI components
│   │       ├── mod.rs
│   │       ├── chat.rs   # Chat/conversation panel
│   │       ├── sidebar.rs # File tree / context panel
│   │       └── status.rs # Status bar
│   ├── web/              # Web frontend (future, stub)
│   │   └── mod.rs
│   ├── agent/
│   │   ├── mod.rs        # Agent trait and core types
│   │   ├── coding.rs     # Coding agent implementation
│   │   └── planner.rs    # Task planning logic
│   ├── tools/
│   │   ├── mod.rs        # Tool trait and registry
│   │   ├── fs.rs         # File operations (read, write, edit)
│   │   ├── shell.rs      # Command execution
│   │   └── search.rs     # Code search (grep, ast)
│   ├── lsp/              # Built-in LSP client
│   │   ├── mod.rs        # LSP manager and types
│   │   ├── client.rs     # LSP client implementation
│   │   ├── transport.rs  # stdio/socket transport
│   │   └── handlers.rs   # Request/response handlers
│   ├── tree_sitter/      # Built-in tree-sitter integration
│   │   ├── mod.rs        # Parser manager
│   │   ├── queries/      # Language-specific queries
│   │   └── languages/    # Language bindings (Rust, TS, etc.)
│   ├── llm/
│   │   ├── mod.rs        # LLM trait and types
│   │   ├── openai.rs     # OpenAI-compatible client
│   │   └── prompts.rs    # System prompts, templates
│   ├── context/
│   │   ├── mod.rs        # Context management
│   │   ├── gatherer.rs   # Gather relevant files
│   │   └── compactor.rs  # Context compaction for long sessions
│   └── config.rs         # Configuration
├── tests/
│   └── integration/      # Integration tests
└── docs/
    └── architecture.md   # Architecture documentation
```

## Code Intelligence Architecture

### LSP Integration

Serana includes a built-in LSP client that supports:
- **Definition navigation**: Go to definition, type definition, implementation
- **References**: Find all references to a symbol
- **Diagnostics**: Real-time errors, warnings, hints
- **Hover**: Type information and documentation
- **Rename**: Rename symbols across the codebase
- **Code actions**: Quick fixes, refactorings

```
┌────────────────────────────────────────────────────┐
│                 LSP Manager                         │
│  Manages multiple language servers per workspace    │
└────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│ rust-analyzer   │  │ typescript-ls   │  │ Other LSPs...   │
│ (Rust)          │  │ (TypeScript)    │  │                 │
└─────────────────┘  └─────────────────┘  └─────────────────┘
```

### Tree-sitter Integration

For fast, accurate code parsing without external processes:
- **AST parsing**: Parse code into syntax trees
- **Syntax queries**: Query patterns via S-expressions
- **Incremental parsing**: Efficient re-parsing on edits
- **Multi-language**: Rust, TypeScript, Python, Go, etc.

```rust
// Example: Find all function definitions
let query = Query::new(language, "(function_item name: (identifier) @name)")?;
let matches = cursor.matches(&query, tree.root_node(), source);

// Use in agent for:
// - "Find all functions named X"
// - "Where is struct Y defined?"
// - "List all imports in this file"
```

### Combined Use Cases

| Task | Primary Tool | Fallback |
|------|--------------|----------|
| Go to definition | LSP | tree-sitter search |
| Find references | LSP | tree-sitter + grep |
| Rename symbol | LSP | tree-sitter + edit |
| List functions | tree-sitter | LSP document symbols |
| Parse error locations | tree-sitter | LSP diagnostics |
| Type information | LSP hover | N/A |

## Code Style

```rust
// Prefer async trait for extensibility
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    async fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value>;
}

// Use thiserror for domain errors
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("LLM request failed: {0}")]
    LlmError(#[from] LlmError),
    
    #[error("Tool execution failed: {tool}")]
    ToolError { tool: String, source: anyhow::Error },
}

// Prefer strongly-typed configurations
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub llm: LlmConfig,
    pub workspace: PathBuf,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}
```

## Testing Strategy

- **Unit tests**: Each module has inline `#[cfg(test)]` tests
- **Integration tests**: `tests/` directory for multi-module scenarios
- **Coverage**: `cargo tarpaulin` for coverage reports
- **Test levels**:
  - Tool execution (mocked LLM responses)
  - Agent planning logic
  - TUI component rendering
  - LSP client behavior (with mock language server)
  - Tree-sitter parsing and queries
  - End-to-end task completion (with recorded LLM interactions)

## Boundaries

**Always**:
- Run `cargo test` and `cargo clippy` before commits
- Use `thiserror` for public error types
- Document public APIs with doc comments
- Handle all `Result` explicitly (no `.unwrap()` in library code)

**Ask first**:
- Adding new dependencies
- Changing public API signatures
- Modifying agent architecture

**Never**:
- Commit `target/` or IDE files
- Use `unwrap()` or `expect()` in library code
- Log sensitive data (API keys, file contents in debug logs)

## Success Criteria

### MVP (Coding Agent Phase)

- [ ] TUI launches and displays chat interface
- [ ] Built-in LSP client can connect to rust-analyzer
- [ ] Tree-sitter can parse Rust/TypeScript files
- [ ] Can answer "where is X defined?" using LSP/tree-sitter
- [ ] Can find references via LSP
- [ ] Can rename symbols via LSP
- [ ] Can edit files with natural language instructions
- [ ] Can run shell commands and capture output
- [ ] Shows streaming LLM responses in TUI
- [ ] All tests pass, clippy clean

### Future

- [ ] Web frontend implementation
- [ ] Architecture supports hot-swapping agent strategies
- [ ] Can explain its own reasoning process
- [ ] Supports multi-agent collaboration

## Open Questions

1. **LLM Provider**: Start with OpenAI, or support local models (Ollama, llama.cpp)?
2. **Context Strategy**: Sliding window? Summary-based? RAG?
3. **Hermes Architecture**: What specific capabilities define "hermes agent"?
4. **Permissions**: Should agent ask before destructive operations (rm, git push)?
5. **LSP Strategy**: Which LSPs to bundle vs. require user to install?
