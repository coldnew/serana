# serana-tools — Tool Implementations

## Purpose

Implements the `Tool` trait from `serana-core` for all agent-accessible operations. Registered via `ToolRegistry`.

## Dependencies

- `serana-core`, `serana-lsp`, `serana-tree-sitter`
- `reqwest`, `rusqlite`, `regex`, `glob`, `ignore`, `toml`

## Module Map (24 tools)

| Module | Tools | Purpose |
|--------|-------|---------|
| `fs` | `ReadFileTool`, `WriteFileTool`, `EditFileTool` | File I/O; `EditFileTool` uses hashline patch format |
| `shell` | `ShellTool`, `ShellSession` | Persistent bash subprocess with sentinel-based output capture |
| `search_native` | `FindTool`, `SearchTool` | Glob file finding + ripgrep content search (in-process) |
| `code_intel` | `AstOutlineTool`, `AstFunctionsTool`, `AstImportsTool`, `LspDefinitionTool`, `LspReferencesTool`, `LspHoverTool` | Tree-sitter AST + LSP code queries |
| `self_evolve` | `ReadSelfTool`, `EditSelfTool`, `CargoTool`, `GitTool`, `SearchCodeTool`, `WorkspaceRootTool`, `RecordModificationTool`, `ModificationStatsTool`, `ReflectModificationTool` | Serana modifying her own codebase |
| `eval` | `EvalTool` | Execute code in Python 3 or Node.js subprocess (timeout: 30s) |
| `web_search` | `WebSearchTool`, `UrlFetchTool` | Brave Search API + HTTP URL fetcher |
| `github` | `GitHubPrViewTool`, `GitHubIssueViewTool`, `GitHubPrDiffTool` | GitHub REST API (requires `GITHUB_TOKEN`) |
| `git_ops` | `GitStatusTool`, `GitDiffTool`, `GitLogTool`, `GitCommitTool` | Local git porcelain commands |
| `ssh` | `SshTool` | Remote command execution via `ssh` subprocess |
| `browser` | `BrowserTool` | Headless Chromium via CDP HTTP endpoint |
| `dap` | `DebugTool`, `DapSession` | DAP protocol client for launch/breakpoints/step/inspect |
| `conflict` | `ConflictResolveTool` | Git merge conflict resolution (ours/theirs/base/manual) |
| `checkpoint` | `CheckpointTool`, `RewindTool` | Git-based checkpoint/snapshot and rollback |
| `review` | `CodeReviewTool` | LLM-powered code review |
| `rules` | `RulesInfoTool` | Read AGENTS.md / CLAUDE.md rules files |
| `stats` | `StatsTool` | Session and token usage analytics |
| `mcp` | `McpTool` | Model Context Protocol client for external tool servers |
| `clipboard` | `ClipboardCopyTool`, `ClipboardPasteTool` | System clipboard (via OSC 52) |
| `memory` | `RetainTool`, `RecallTool`, `ReflectTool` | SQLite-backed persistent memory with FTS5 full-text search |
| `skill` | `SkillCreateTool`, `SkillStore` | Discover/create/load reusable skill sets |
| `recipe` | `RecipeTool` | Load and apply YAML recipe files |
| `hashline` | (internal) | Line-anchored patch format (`41th|`) for safe edits |

## ToolRegistry

- `new()` — registers all built-in tools
- `register_lsp(manager)` — registers LSP tools with a shared `LspManager`
- `register_skill(workspace)` — registers skill creation tool
- `get(name)` / `list()` / `describe_all()` — lookup and introspection

## Notable Tools

### ShellSession

- Persistent bash subprocess with piped stdin/stdout
- Sentinel-based command completion detection (`__SERANA_DONE_<nanos>__`)
- Configurable timeout per command (default 30s)
- Avoids fork/exec overhead for each command

### EditFileTool (Hashline)

- Parses hashline patch format: `@@ PATH`, `+ ANCHOR` (insert after), `< ANCHOR` (insert before), `- A..B` (delete), `= A..B` (replace)
- Anchors are `<line><2-char-hash>` (e.g., `41th`) computed from line content
- Stale anchor recovery via content hash matching

### MemoryStore (SQLite + FTS5)

- Tables: `facts` (id, project, fact_text, tags, created_at) + `facts_fts` virtual table
- Auto-sync triggers on INSERT/UPDATE/DELETE
- Tag-based filtering, full-text search, dedup by hash

### DapSession

- Full DAP lifecycle: launch → initialize → setBreakpoints → configurationDone → continue
- Event loop reads stdout on a background task, dispatches via mpsc channel
- Supports `pause`, `step_in`, `next`, `continue`, `stack_trace`, `scopes`, `variables`

### McpTool / McpConnection

- stdio-based MCP transport with JSON-RPC 2.0 framing
- Reads `mcp_servers.toml` for server config (command, args, env)
- `list_tools` and `call_tool` proxied to external MCP servers

## Design Decisions

- **Sequential tool execution**: `execute_tools_concurrent` in serana-agent actually runs tools sequentially (loop, not join_all). Tool isolation makes concurrent execution safe but not yet parallelized.
- **Shared LSP state via Arc<Mutex<>>**: LSP servers persist across agent iterations instead of spawning fresh per call.
- **BRAVE_API_KEY for web search**: Default search provider is Brave Search API. No built-in fallback.
- **GITHUB_TOKEN for GitHub**: No OAuth flow — expects pre-configured PAT in environment.
