# serana-tools — Tool Implementations

## Overview

`serana-tools` implements the `Tool` trait from `serana-core` for all agent-accessible operations. Tools are organized into three tiers (Core, Workspace, Hermes) and registered via `ToolRegistry`.

**Crate path:** `crates/serana-tools/`

## Dependencies

- **Internal:** `serana-core`, `serana-lsp`, `serana-tree-sitter`
- **External:** `tokio`, `async-trait`, `serde`, `serde_json`, `anyhow`, `tracing`, `reqwest`, `rusqlite`, `dirs`, `regex`, `glob`, `ignore`, `toml`, `chrono`

## Module Map

| Module | Tools | Purpose |
|--------|-------|---------|
| `fs` | `ReadFileTool`, `WriteFileTool`, `EditFileTool` | File I/O |
| `search_native` | `FindTool`, `SearchTool` | Glob file finding + regex content search |
| `code_intel` | `AstOutlineTool`, `AstFunctionsTool`, `AstImportsTool`, `LspDefinitionTool`, `LspReferencesTool`, `LspHoverTool` | AST + LSP code queries |
| `shell` | `ShellTool`, `ShellSession` | Persistent bash subprocess |
| `git_ops` | `GitStatusTool`, `GitDiffTool`, `GitLogTool`, `GitCommitTool` | Git porcelain commands |
| `checkpoint` | `CheckpointTool`, `RewindTool` | Conversation checkpoints |
| `review` | `CodeReviewTool` | LLM-powered code review |
| `conflict` | `ConflictResolveTool` | Merge conflict resolution |
| `stats` | `StatsTool`, `SessionStats` | Session analytics |
| `clipboard` | `ClipboardCopyTool`, `ClipboardPasteTool` | System clipboard |
| `rules` | `RulesInfoTool`, `load_rules()`, `check_rules()` | Rules system |
| `skill` | `SkillCreateTool`, `SkillStore` | Reusable skill sets |
| `memory` | `RetainTool`, `RecallTool`, `ReflectTool`, `MemoryStore` | Persistent memory (SQLite + FTS5) |
| `hashline` | (internal) | Line-anchored patch format |
| `self_evolve` | `ReadSelfTool`, `EditSelfTool`, `CargoTool`, `GitTool`, `SearchCodeTool`, `WorkspaceRootTool`, `RecordModificationTool`, `ModificationStatsTool`, `ReflectModificationTool` | Self-modification tools |
| `eval` | `EvalTool` | Code execution (Python/JS) |
| `web_search` | `WebSearchTool`, `UrlFetchTool` | Web search + URL fetch |
| `github` | `GitHubPrViewTool`, `GitHubIssueViewTool`, `GitHubPrDiffTool` | GitHub REST API |
| `ssh` | `SshTool` | Remote command execution |
| `browser` | `BrowserTool` | Headless Chromium CDP |
| `mcp` | `McpTool`, `McpConnection` | Model Context Protocol |
| `dap` | `DebugTool`, `DapSession` | Debug Adapter Protocol |
| `recipe` | `RecipeTool` | Task runner |

## ToolRegistry

Central registry for all tools.

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

pub enum ToolProfile {
    Core,      // essential tools (read, write, edit, search, shell)
    Workspace, // + git, code intel, rules
    Hermes,    // + self-evolve, eval, web, github, ssh, browser, dap, mcp
}
```

### Registration

```rust
// Tiered registration
let registry = ToolRegistry::core();       // 8 tools
let registry = ToolRegistry::workspace();  // 16 tools
let registry = ToolRegistry::hermes();     // 24+ tools

// Add LSP tools (shared LspManager)
registry.register_lsp(lsp_manager);

// Add skill tool
registry.register_skill(workspace_path);

// Manual registration
registry.register(Arc::new(MyCustomTool));
```

### Lookup

```rust
let tool = registry.get("read_file")?;           // by name
let tools = registry.list();                      // all tools
let defs = registry.definitions();                // Vec<ToolDefinition> for LLM
```

## Notable Tools

### ShellSession (`shell.rs`)

Persistent bash subprocess with sentinel-based output capture.

```rust
pub struct ShellSession {
    stdin: ChildStdin,
    stdout: ChildStdin,
    // ...
}
```

**How it works:**
1. Spawns `bash` with piped stdin/stdout
2. For each command, wraps it with a sentinel: `{command}; echo __SERANA_DONE_{nanos}__`
3. Reads stdout until sentinel appears
4. Returns output between command start and sentinel

**Benefits:**
- No fork/exec overhead per command
- Persistent environment (cd, exports, etc.)
- Configurable timeout per command (default 30s)

### EditFileTool (`hashline.rs`)

Line-anchored patch format for safe file edits.

**Hashline format:**
```
@@ path/to/file.rs
+ anchor_line   ← insert after this anchor
  new content here
< anchor_line   ← insert before this anchor
  another line
- start..end    ← delete lines start to end
= start..end    ← replace lines start..end
  replacement content
```

**Anchor format:** `{line_number}{2_char_hash}` (e.g., `41th`)
- Hash is computed from line content for stability
- Stale anchor recovery via content hash matching

### MemoryStore (`memory.rs`)

SQLite-backed persistent memory with FTS5 full-text search.

```rust
pub struct MemoryStore {
    conn: rusqlite::Connection,
}
```

**Schema:**
```sql
CREATE TABLE facts (
    id INTEGER PRIMARY KEY,
    project TEXT,
    fact_text TEXT,
    tags TEXT,
    content_hash TEXT UNIQUE,
    created_at TEXT
);
CREATE VIRTUAL TABLE facts_fts USING fts5(fact_text, tags, content='facts', content_rowid='id');
```

**Features:**
- Auto-sync triggers on INSERT/UPDATE/DELETE
- Tag-based filtering
- Full-text search via FTS5
- Dedup by content hash

### DapSession (`dap.rs`)

Debug Adapter Protocol client.

```rust
pub struct DapSession {
    process: Child,
    // ...
}
```

**Lifecycle:** launch → initialize → setBreakpoints → configurationDone → continue

**Supported operations:**
- `pause`, `step_in`, `next`, `continue`
- `stack_trace`, `scopes`, `variables`
- `set_breakpoints`

### McpTool / McpConnection (`mcp.rs`)

Model Context Protocol client for external tool servers.

```rust
pub struct McpConnection {
    stdin: ChildStdin,
    stdout: ChildStdin,
    // ...
}
```

- Reads `mcp_servers.toml` for server config (command, args, env)
- stdio-based transport with JSON-RPC 2.0 framing
- `list_tools` and `call_tool` proxied to external MCP servers

### WebSearchTool (`web_search.rs`)

Web search via Brave Search API.

- Requires `BRAVE_API_KEY` environment variable
- Returns title, URL, description for each result
- `UrlFetchTool` fetches and returns page content (truncated)

### GitHub Tools (`github.rs`)

GitHub REST API integration.

- Requires `GITHUB_TOKEN` environment variable
- `GitHubPrViewTool` — view PR details
- `GitHubIssueViewTool` — view issue details
- `GitHubPrDiffTool` — view PR diff

### Rules System (`rules.rs`)

Regex-triggered rule injections from TOML files.

```toml
# .serana/rules.toml
[[rules]]
pattern = "cargo test"
injection = "Remember to run `cargo fmt` before committing."
```

- `load_rules(workspace)` — reads `.serana/rules.toml` and `~/.serana/rules.toml`
- `check_rules(text)` — matches text against patterns, returns injections
- `RulesInfoTool` — displays active rules

### SkillStore (`skill.rs`)

Markdown-based reusable skill sets.

```rust
pub struct SkillStore {
    skills: HashMap<String, Skill>,
}

pub struct Skill {
    pub name: String,
    pub description: String,
    pub content: String,  // markdown instructions
}
```

- Skills stored in `.serana/skills/` as markdown files
- `SkillCreateTool` — creates new skill from current conversation
- Skills injected into system prompt by `PromptBuilder`

## Tool Tiers

### Core (8 tools)

Essential file and shell operations:
- `read_file`, `write_file`, `edit_file`
- `find`, `search`
- `shell`
- `checkpoint`, `rewind`

### Workspace (+8 tools)

Code intelligence and project management:
- `git_status`, `git_diff`, `git_log`, `git_commit`
- `ast_outline`, `ast_functions`, `ast_imports`
- `lsp_definition`, `lsp_references`, `lsp_hover`
- `rules_info`

### Hermes (+10 tools)

Full power tools:
- `read_self`, `edit_self`, `cargo`, `git`, `search_code`, `workspace_root`
- `record_modification`, `modification_stats`, `reflect_modification`
- `eval`
- `web_search`, `url_fetch`
- `github_pr_view`, `github_issue_view`, `github_pr_diff`
- `ssh`
- `browser`
- `debug`
- `mcp`

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| Sequential tool execution | `execute_tools_concurrent` in serana-agent iterates sequentially; true concurrency is safe but not yet enabled |
| Shared LSP state via `Arc<Mutex<>>` | LSP servers persist across agent iterations instead of spawning fresh per call |
| `BRAVE_API_KEY` for web search | Default search provider; no built-in fallback |
| `GITHUB_TOKEN` for GitHub | No OAuth flow; expects pre-configured PAT |
| Hashline format for edits | Line-anchored patches are LLM-friendly and recoverable from stale anchors |
| SQLite + FTS5 for memory | Full-text search without external dependencies |
| Tool tiers (Core/Workspace/Hermes) | Progressive capability exposure based on agent configuration |
