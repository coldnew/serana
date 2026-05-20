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
```

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
│   │   ├── search.rs     # Code search (grep, ast)
│   │   └── lsp.rs        # LSP integration
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
- [ ] Can read and understand a Rust project structure
- [ ] Can answer questions about codebase ("where is X implemented?")
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
