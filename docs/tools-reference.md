# Tools Reference

## Overview

Serana provides 24+ built-in tools organized into three tiers. Tools are the primary interface between the agent and the external world.

## Tool Tiers

| Tier | Profile | Description |
|------|---------|-------------|
| **Core** | `ToolProfile::Core` | Essential file and shell operations |
| **Workspace** | `ToolProfile::Workspace` | Code intelligence and project management |
| **Hermes** | `ToolProfile::Hermes` | Full power: self-evolution, web, remote, debug |

## Complete Tool Catalog

### File System Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `ReadFileTool` | `read_file` | Core | Read file contents with optional line range |
| `WriteFileTool` | `write_file` | Core | Write content to file (creates or overwrites) |
| `EditFileTool` | `edit_file` | Core | Apply hashline patches to existing files |

#### read_file

```json
{
  "path": "src/main.rs",
  "offset": 10,
  "limit": 50
}
```

Returns file contents. Optional `offset` (0-indexed line) and `limit` (number of lines).

#### write_file

```json
{
  "path": "src/new_file.rs",
  "content": "fn main() {\n    println!(\"hello\");\n}"
}
```

Creates parent directories if needed. Overwrites existing files.

#### edit_file

```json
{
  "path": "src/main.rs",
  "patch": "@@ src/main.rs\n+ 5ab3\n  new line here\n- 10cd..12ef\n= 20ab..25cd\n  replacement content"
}
```

Uses hashline patch format. Anchors are `{line_number}{2_char_hash}` computed from line content.

### Search Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `FindTool` | `find` | Core | Find files by glob pattern (respects .gitignore) |
| `SearchTool` | `search` | Core | Search file contents by regex |

#### find

```json
{
  "pattern": "**/*.rs",
  "path": "src/"
}
```

Returns matching file paths. Uses the `ignore` crate for .gitignore support.

#### search

```json
{
  "pattern": "fn\\s+\\w+",
  "include": "*.rs",
  "path": "src/"
}
```

Returns file paths and line numbers with matches. Uses ripgrep-compatible regex.

### Shell Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `ShellTool` | `shell` | Core | Execute shell commands in persistent bash session |

#### shell

```json
{
  "command": "cargo test",
  "timeout": 60
}
```

Executes in persistent bash session. Default timeout: 30s. Environment and working directory persist across calls.

### Git Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `GitStatusTool` | `git_status` | Workspace | Show working tree status |
| `GitDiffTool` | `git_diff` | Workspace | Show changes (staged and unstaged) |
| `GitLogTool` | `git_log` | Workspace | Show commit history |
| `GitCommitTool` | `git_commit` | Workspace | Create a commit |

#### git_status

```json
{}
```

Returns `git status --short` output.

#### git_diff

```json
{
  "staged": false,
  "path": "src/main.rs"
}
```

Returns diff output. Optional `staged` flag and `path` filter.

#### git_log

```json
{
  "limit": 10,
  "oneline": true
}
```

Returns commit history. Default: 10 commits, one-line format.

#### git_commit

```json
{
  "message": "feat: add new feature",
  "files": ["src/main.rs", "src/lib.rs"]
}
```

Stages specified files and creates commit.

### Code Intelligence Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `AstOutlineTool` | `ast_outline` | Workspace | Get file's AST outline (functions, structs, imports) |
| `AstFunctionsTool` | `ast_functions` | Workspace | Extract function definitions |
| `AstImportsTool` | `ast_imports` | Workspace | Extract import statements |
| `LspDefinitionTool` | `lsp_definition` | Workspace | Go to definition via LSP |
| `LspReferencesTool` | `lsp_references` | Workspace | Find references via LSP |
| `LspHoverTool` | `lsp_hover` | Workspace | Hover information via LSP |

#### ast_outline

```json
{
  "path": "src/main.rs"
}
```

Returns functions, structs, and imports found in the file via tree-sitter.

#### lsp_definition

```json
{
  "path": "src/main.rs",
  "line": 10,
  "character": 5
}
```

Returns definition location(s) via language server.

#### lsp_references

```json
{
  "path": "src/main.rs",
  "line": 10,
  "character": 5
}
```

Returns all reference locations via language server.

### Checkpoint Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `CheckpointTool` | `checkpoint` | Core | Create a named checkpoint (git stash) |
| `RewindTool` | `rewind` | Core | Rewind to a checkpoint |

#### checkpoint

```json
{
  "name": "before-refactor"
}
```

Creates git stash with the given name.

#### rewind

```json
{
  "name": "before-refactor"
}
```

Restores git stash by name.

### Self-Evolution Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `ReadSelfTool` | `read_self` | Hermes | Read Serana's own source files |
| `EditSelfTool` | `edit_self` | Hermes | Edit Serana's own source files |
| `CargoTool` | `cargo` | Hermes | Run cargo commands (build, test, etc.) |
| `GitTool` | `git` | Hermes | Run git commands |
| `SearchCodeTool` | `search_code` | Hermes | Search Serana's codebase |
| `WorkspaceRootTool` | `workspace_root` | Hermes | Get workspace root path |
| `RecordModificationTool` | `record_modification` | Hermes | Log a modification to MetaCognition |
| `ModificationStatsTool` | `modification_stats` | Hermes | Query modification history |
| `ReflectModificationTool` | `reflect_modification` | Hermes | Extract lessons from modifications |

#### cargo

```json
{
  "command": "build --release"
}
```

Runs `cargo {command}` in the workspace root. Used for build/test verification.

### Web Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `WebSearchTool` | `web_search` | Hermes | Search the web via Brave Search API |
| `UrlFetchTool` | `url_fetch` | Hermes | Fetch and return URL content |

#### web_search

```json
{
  "query": "rust async tokio tutorial",
  "count": 5
}
```

Requires `BRAVE_API_KEY` environment variable. Returns title, URL, description for each result.

#### url_fetch

```json
{
  "url": "https://docs.rs/tokio"
}
```

Fetches page content, returns as text (truncated).

### GitHub Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `GitHubPrViewTool` | `github_pr_view` | Hermes | View pull request details |
| `GitHubIssueViewTool` | `github_issue_view` | Hermes | View issue details |
| `GitHubPrDiffTool` | `github_pr_diff` | Hermes | View pull request diff |

#### github_pr_view

```json
{
  "repo": "owner/repo",
  "number": 42
}
```

Requires `GITHUB_TOKEN` environment variable.

### Remote Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `SshTool` | `ssh` | Hermes | Execute commands on remote hosts |

#### ssh

```json
{
  "host": "server.example.com",
  "command": "uname -a",
  "user": "deploy"
}
```

### Debug Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `DebugTool` | `debug` | Hermes | Debug code via DAP protocol |

#### debug

```json
{
  "action": "launch",
  "program": "target/debug/myapp",
  "args": ["--test"]
}
```

Actions: `launch`, `set_breakpoints`, `continue`, `step_in`, `next`, `pause`, `stack_trace`, `scopes`, `variables`, `evaluate`.

### Memory Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `RetainTool` | `retain` | Hermes | Store a fact in persistent memory |
| `RecallTool` | `recall` | Hermes | Search stored facts |
| `ReflectTool` | `reflect` | Hermes | List all stored facts |

#### retain

```json
{
  "fact": "The project uses tokio for async runtime",
  "tags": ["rust", "async", "tokio"]
}
```

#### recall

```json
{
  "query": "async runtime"
}
```

Full-text search across stored facts.

### Skill Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `SkillCreateTool` | `skill_create` | Hermes | Create a reusable skill from current conversation |

#### skill_create

```json
{
  "name": "rust-testing",
  "description": "Patterns for writing Rust tests"
}
```

Creates a markdown skill file in `.serana/skills/`.

### Code Review Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `CodeReviewTool` | `code_review` | Hermes | LLM-powered code review |

#### code_review

```json
{
  "path": "src/main.rs"
}
```

Returns structured review prompt for the LLM to analyze.

### Conflict Resolution Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `ConflictResolveTool` | `conflict_resolve` | Workspace | Resolve merge conflicts |

#### conflict_resolve

```json
{
  "path": "src/main.rs",
  "strategy": "ours"
}
```

Strategies: `ours`, `theirs`, `base`, `manual`.

### Stats Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `StatsTool` | `stats` | Core | Show session and token usage statistics |

#### stats

```json
{}
```

Returns tokens in/out, tool calls made, errors encountered, session duration.

### Clipboard Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `ClipboardCopyTool` | `clipboard_copy` | Core | Copy text to system clipboard |
| `ClipboardPasteTool` | `clipboard_paste` | Core | Paste text from system clipboard |

### Rules Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `RulesInfoTool` | `rules_info` | Workspace | Display active rules from AGENTS.md and rules files |

### Eval Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `EvalTool` | `eval` | Hermes | Execute code in Python or Node.js subprocess |

#### eval

```json
{
  "language": "python",
  "code": "print(sum(range(100)))"
}
```

Supported languages: `python`, `javascript`. Timeout: 30s.

### Browser Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `BrowserTool` | `browser` | Hermes | Headless browser via Chromium CDP |

#### browser

```json
{
  "action": "navigate",
  "url": "https://example.com"
}
```

### MCP Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `McpTool` | `mcp` | Hermes | Call external MCP server tools |

### Recipe Tools

| Tool | Name | Tier | Description |
|------|------|------|-------------|
| `RecipeTool` | `recipe` | Hermes | Auto-detect and run project tasks |

#### recipe

```json
{
  "task": "test"
}
```

Auto-detects task runner (Cargo, Just, Make, NPM) and runs the specified task.

## Tool Approval System

Tools are classified by risk level:

| Risk Level | Examples | Approval |
|------------|----------|----------|
| **Safe** | `read_file`, `search`, `ast_outline`, `git_status`, `stats` | Auto-approve |
| **Low** | `write_file`, `edit_file`, `git_diff`, `checkpoint` | Auto-approve |
| **Medium** | `git_commit`, `cargo`, `eval` | Depends on mode |
| **High** | `shell`, `ssh`, `edit_self` | Depends on mode |

### Approval Modes

| Mode | Behavior |
|------|----------|
| `Auto` | Approve all tools automatically |
| `Interactive` | Prompt user for Medium/High risk tools |
| `Whitelist` | Only approve tools in whitelist |
| `Smart` | Approve Safe/Low, prompt for Medium/High |
