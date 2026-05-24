# Serana

A personal Hermes agent built in Rust with TUI frontend.

## Features

- **TUI Interface**: Terminal-based chat interface using ratatui
- **Tool Execution**: Built-in tools for file operations (read, write, edit)
- **LLM Integration**: OpenAI-compatible API with multi-provider support
- **Code Intelligence**: LSP client and tree-sitter integration (in progress)

## Quick Start

```bash
# Build
cargo build --release

# Run TUI
cargo run

# Run with instruction (non-interactive)
cargo run -- "explain the project structure"

# Show configuration
cargo run -- config

# Show sample config
cargo run -- config --sample
```

## Configuration

Serana reads configuration from `~/.serana/config.toml`:

```toml
[provider]
name = "openai"  # openai, anthropic, ollama, openrouter, custom

[llm]
model = "gpt-4"
temperature = 0.7

[agent]
max_tokens = 4096
```

### Environment Variables

Environment variables take precedence over config file:

- `SERANA_API_KEY` - API key (recommended)
- `SERANA_PROVIDER` - Provider name
- `SERANA_MODEL` - Model name
- `OPENAI_API_KEY` - Fallback for OpenAI

### Supported Providers

| Provider | API URL |
|----------|---------|
| openai | https://api.openai.com/v1 |
| anthropic | https://api.anthropic.com/v1 |
| ollama | http://localhost:11434/v1 |
| openrouter | https://openrouter.ai/api/v1 |
| custom | User-specified |

## Development

```bash
# Run tests
cargo test

# Lint
cargo clippy -- -D warnings

# Format check
cargo fmt --check
```

## License

MIT
