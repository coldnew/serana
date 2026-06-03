# SPEC: Serana — Personal Hermes Agent

## Objective

A personal Hermes agent in Rust that understands codebases (via LSP + tree-sitter), edits files, runs commands, searches the web, debugs programs, persists memory, and answers code questions — all from a terminal UI.

**User**: Single developer (coldnew) for personal productivity.
**Architecture**: Serana-owned modules in the `serana` crate, with neutral
workspace crates only for infrastructure shared by multiple products.

## Tech Stack

| Component | Choice |
|-----------|--------|
| Language | Rust (edition 2021, stable) |
| Async | tokio (full features) |
| LLM Protocol | OpenAI-compatible REST API + Anthropic Messages API |
| Serialization | serde + serde_json |
| Frontend | ratatui + crossterm (TUI) |
| LSP | Built-in client via stdio (rust-analyzer, typescript-language-server, pylsp, gopls) |
| AST | tree-sitter (Rust, JS/TS, Python, Go) |
| Storage | rusqlite (sessions + memory) |
| Clipboard | OSC 52 escape sequences |
| Web Search | Brave Search API |
| Debugger | DAP protocol stdio client |

## Module Architecture

```
serana
  ├── core   — Agent, Tool, LlmClient traits, Config, Message, callbacks,
  │            cancellation, budgets, compression, MetaCognition, TokenCounter,
  │            ToolApproval, Verification
  ├── llm    — LlmClient impls: OpenAI, OpenRouter, Anthropic, streaming SSE,
  │            credential refresh, fallback chains, role-based routing,
  │            auxiliary tasks
  ├── lsp    — LSP client: spawn servers, JSON-RPC 2.0, go-to-def, refs, hover
  ├── tools  — Tool impls: fs, shell, search, git, GitHub, SSH, eval, debug,
  │            browser, MCP, memory, skills, code review, checkpoint, AST, etc.
  ├── agent  — Agent orchestration: HermesAgent, session persistence, subagent
  │            spawning, context compression, stream rules, message validation
  └── tui    — TUI: ratatui rendering, event loop, markdown, syntax highlight,
               editor, dialogs, slash commands

crates/code-tree-sitter
  └── Shared AST parsing/highlighting for Serana tools and Mora syntax display
```

## Key Traits

```rust
#[async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> &'static str;
    async fn execute(&self, instruction: &str) -> Result<AgentOutput>;
    async fn chat(&self, message: &str) -> Result<String>;
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters(&self) -> Value { json!({"type": "object"}) }
    async fn execute(&self, input: Value) -> Result<Value>;
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat(&self, messages: &[Message]) -> Result<String>;
    async fn chat_with_tools(&self, messages: &[Message], tools: &[ToolDefinition]) -> Result<Message>;
    fn chat_stream<'a>(...) -> Pin<Box<dyn Stream<Item=Result<String>> + Send + 'a>>;
    fn chat_with_tools_stream<'a>(...) -> Pin<Box<dyn Stream<Item=Result<Message>> + Send + 'a>>;
}
```

## Version

0.1.0 — MIT License, authored by coldnew.

## Build & Test

```bash
cargo build --release        # builds all crates + binary
cargo test                   # tests all crates
cargo clippy -- -D warnings  # lint all crates
cargo fmt --check            # format check
```

## Configuration

Config file: `~/.serana/config.toml` (TOML format).

```toml
[provider]
name = "openai"   # openai, anthropic, ollama, openrouter, custom

[llm]
model = "gpt-4"
temperature = 0.7
# api_key = "sk-..."  # prefer SERANA_API_KEY env var
```

Environment overrides (highest priority):
- `SERANA_API_KEY` — API key
- `SERANA_PROVIDER` — provider name (openai, anthropic, etc.)
- `SERANA_MODEL` — model name
- `OPENAI_API_KEY` — fallback for OpenAI
- `ANTHROPIC_API_KEY` — used by AnthropicClient
- `BRAVE_API_KEY` — required for web_search tool
- `GITHUB_TOKEN` — required for GitHub tools

Provider URLs (built-in):
| Provider | URL |
|----------|-----|
| openai | https://api.openai.com/v1 |
| anthropic | https://api.anthropic.com/v1 |
| ollama | http://localhost:11434/v1 |
| openrouter | https://openrouter.ai/api/v1 |
| custom | `provider.url` from config |

## CLI Usage

```bash
cargo run                      # Launch TUI (if TTY) or REPL
cargo run -- -w /path/to/proj  # Launch with workspace
cargo run -- "instruction"     # Non-interactive mode
cargo run -- interactive       # Force interactive REPL
cargo run -- config            # Show resolved config
cargo run -- config --sample   # Show sample config.toml
cargo run -- login codex       # Start Codex CLI device-auth sign-in
```

In the TUI, `/model codex/gpt-5.5` selects the Codex CLI provider. Codex-backed
responses run through `codex exec` in read-only, non-streaming mode.

## Coding Rules

- `unsafe_code = "deny"` across entire workspace (enforced in workspace Cargo.toml)
- Use `thiserror` for public error types where useful; `serana::core::Result`
  is currently an `anyhow::Error` result alias used by Serana contracts.
- No `.unwrap()` in library code (tolerated in tests and CLI main)
- All tests must pass before commit

## Known Stubs / Incomplete

- `ContextGatherer` — returns empty Vec (placeholder)
- `ContextCompactor` — returns new empty Context (placeholder)
- `BrowserTool` — CDP HTTP endpoint works, WebSocket path needs tungstenite
- `LspManager` — no `didChange` text sync yet
- `DapSession` — single-session only, no multi-session manager
- `Message::ToolCall` function arguments in streaming mode may be incomplete (no argument accumulation in SSE parser branch for OpenAI)

## Future Directions

- Full Hermes agent architecture (multi-agent, self-reflection)
- Web frontend (WebSocket-based)
- RAG for semantic codebase indexing
- Full workspace text sync for LSP
- Incremental tree-sitter parsing
- Streaming output rules (TTSR) integration with all providers
