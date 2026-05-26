# Configuration Reference

## Overview

Serana uses a TOML-based configuration system with environment variable overrides. The primary config file is at `~/.serana/config.toml`.

## Config File Location

```
~/.serana/config.toml
```

## Config Structure

```toml
# Default provider and model
default_provider = "openai"
default_model = "gpt-4o"

# Optional workspace root
workspace = "/path/to/workspace"

# Provider configurations
[providers.openai]
api_url = "https://api.openai.com/v1"
api_key = "sk-..."
model = "gpt-4o"

[providers.anthropic]
api_url = "https://api.anthropic.com"
api_key = "sk-ant-..."
model = "claude-sonnet-4-20250514"

[providers.openrouter]
api_url = "https://openrouter.ai/api/v1"
api_key = "sk-or-..."
model = "anthropic/claude-sonnet-4-20250514"
```

## Environment Variable Overrides

All config values can be overridden via environment variables. Env vars take precedence over the config file.

### Provider Configuration

| Env Var Pattern | Example | Purpose |
|-----------------|---------|---------|
| `SERANA_<PROVIDER>_API_URL` | `SERANA_OPENAI_API_URL` | Override API URL |
| `SERANA_<PROVIDER>_API_KEY` | `SERANA_ANTHROPIC_API_KEY` | Override API key |
| `SERANA_<PROVIDER>_MODEL` | `SERANA_OPENROUTER_MODEL` | Override model name |
| `<PROVIDER>_API_KEY` | `OPENAI_API_KEY` | Shorthand for API key |

### Other Environment Variables

| Env Var | Purpose |
|---------|---------|
| `BRAVE_API_KEY` | Brave Search API key for `web_search` tool |
| `GITHUB_TOKEN` | GitHub PAT for GitHub tools |

## Default Providers

| Provider | Default API URL | Notes |
|----------|-----------------|-------|
| `openai` | `https://api.openai.com/v1` | OpenAI, OpenRouter-compatible |
| `anthropic` | `https://api.anthropic.com` | Anthropic Messages API |
| `openrouter` | `https://openrouter.ai/api/v1` | Multi-provider gateway |

## Workspace Configuration

Serana looks for workspace-specific configuration in the `.serana/` directory:

```
<workspace>/
  .serana/
    config.toml          # workspace-specific config (overrides global)
    personality.md       # agent personality instructions
    memory.md            # project-specific memory
    skills/              # skill definitions (markdown files)
    context/             # context files injected into prompt
    rules.toml           # regex-triggered rule injections
    stream-rules.toml    # TTSR stream rules
    sessions.db          # session persistence database
```

### personality.md

Custom instructions for the agent's behavior in this workspace.

```markdown
You are a Rust expert. Always prefer idiomatic Rust patterns.
When suggesting changes, explain the reasoning behind them.
```

### memory.md

Project-specific knowledge the agent should remember.

```markdown
This project uses tokio for async runtime.
The main entry point is in src/main.rs.
Tests are in the tests/ directory.
```

### context/

Files in this directory are automatically loaded into the system prompt.

```
.serana/
  context/
    architecture.md    # loaded as context
    conventions.md     # loaded as context
```

### rules.toml

Regex-triggered rule injections that fire when the agent's output matches patterns.

```toml
[[rules]]
pattern = "cargo test"
injection = "Remember to run `cargo fmt` before committing."

[[rules]]
pattern = "git push"
injection = "Make sure all tests pass before pushing."
```

### stream-rules.toml

Time-Traveling Stream Rules (TTSR) that monitor streaming output and inject corrections.

```toml
[[rules]]
name = "no-unwrap"
pattern = "\\.unwrap\\(\\)"
injection = "Avoid using .unwrap() in production code. Use proper error handling with ? or .expect() instead."

[[rules]]
name = "no-clone-everywhere"
pattern = "\\.clone\\(\\)"
injection = "Consider if cloning is necessary. Prefer borrowing (&) when possible."
```

## Session Storage

Sessions are stored in SQLite at:

```
~/.local/share/serana/sessions.db
```

This location follows the XDG Base Directory specification and can be overridden by the `XDG_DATA_HOME` environment variable.

## Theme Configuration

Color themes can be stored at:

```
~/.serana/theme.toml
```

```toml
[theme]
bg = "#1a1a2e"
fg = "#e0e0e0"
accent = "#0f3460"
user_msg = "#16213e"
agent_msg = "#1a1a2e"
system_msg = "#533483"
code_bg = "#0f3460"
diff_add = "#2d6a4f"
diff_del = "#d62828"
```

Switch themes at runtime with the `/theme` command.

## MCP Server Configuration

External MCP (Model Context Protocol) servers are configured in:

```
~/.serana/mcp_servers.toml
```

```toml
[[servers]]
name = "filesystem"
command = "mcp-server-filesystem"
args = ["/path/to/allow"]
env = { ROOT = "/path/to/allow" }

[[servers]]
name = "github"
command = "mcp-server-github"
env = { GITHUB_TOKEN = "ghp_..." }
```

## Model Roles

Serana supports role-based model routing. By default, all roles use the same model. Custom role mappings can be configured programmatically or via config:

| Role | Purpose | Default |
|------|---------|---------|
| `Default` | Primary model for general tasks | `default_model` |
| `Smol` | Fast/cheap model for simple tasks | same as Default |
| `Slow` | Powerful model for complex reasoning | same as Default |
| `Plan` | Planning and analysis | same as Default |
| `Commit` | Commit message generation | same as Default |

## Compression Configuration

Context compression thresholds (configured in code, not TOML):

| Threshold | Default | Description |
|-----------|---------|-------------|
| Preflight | 50% | Start background compression |
| Gateway | 85% | Block next LLM call until compression completes |
| Protect last N | 10 | Number of recent messages to protect from compression |

## Iteration Budgets

| Context | Default | Description |
|---------|---------|-------------|
| Parent agent | 50 | Maximum iterations per execution |
| Subagent | 20 | Maximum iterations per subagent task |

## Tool Profiles

| Profile | Tools Included |
|---------|---------------|
| Core | `read_file`, `write_file`, `edit_file`, `find`, `search`, `shell`, `checkpoint`, `rewind` |
| Workspace | Core + git tools, AST tools, LSP tools, `rules_info` |
| Hermes | Workspace + self-evolve tools, `eval`, web search, GitHub, SSH, browser, DAP, MCP |
