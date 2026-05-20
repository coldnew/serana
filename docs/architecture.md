# Serana Architecture

## Overview

Serana is a personal coding agent built in Rust with a TUI frontend. It features built-in LSP and tree-sitter for deep code understanding without relying on external tools.

## Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                        TUI Frontend                          │
│  ratatui-based interface with chat, sidebar, status bar     │
│  Handles keyboard input, renders streaming responses        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     Agent (agent/mod.rs)                     │
│  Agent trait + CodingAgent implementation                    │
│  - Plans tasks via LLM                                       │
│  - Executes tools based on LLM responses                     │
│  - Manages conversation state                                │
└─────────────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│   LLM Client    │  │  Tool Registry  │  │    Context      │
│  (llm/mod.rs)   │  │  (tools/mod.rs) │  │ (context/mod.rs)│
│                 │  │                 │  │                 │
│  - OpenAI API   │  │  - read_file    │  │  - Gatherer     │
│  - Streaming    │  │  - write_file   │  │  - Compactor    │
│  - Pluggable    │  │  - shell_exec   │  │                 │
└─────────────────┘  └─────────────────┘  └─────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│                 Code Intelligence Layer                      │
├───────────────────────────┬─────────────────────────────────┤
│       LSP Client          │      Tree-sitter Parser         │
│       (lsp/mod.rs)        │   (tree_sitter/mod.rs)          │
│                           │                                 │
│  - rust-analyzer          │  - AST parsing                  │
│  - typescript-language-   │  - Syntax queries               │
│    server                 │  - Incremental edits            │
│  - gopls                  │  - Multi-language support       │
│  - (extensible)           │                                 │
│                           │  Languages:                     │
│  Features:                │  - Rust                         │
│  - go_to_def              │  - TypeScript/JavaScript        │
│  - find_refs              │  - Python                       │
│  - rename                 │  - Go                           │
│  - hover                  │  - (extensible)                 │
│  - diagnostics            │                                 │
└───────────────────────────┴─────────────────────────────────┘
```

## TUI Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Main Window                           │
├──────────────────────┬──────────────────────────────────────┤
│                      │                                      │
│     Sidebar          │         Chat Panel                   │
│   (File Tree)        │    (Conversation History)            │
│                      │                                      │
│   - workspace files  │   User: <message>                    │
│   - context files    │   Agent: <streaming response...>     │
│   - tool outputs     │   [tool call indicators]             │
│                      │                                      │
├──────────────────────┴──────────────────────────────────────┤
│  Input: [_____________________________________________] [Send]│
├─────────────────────────────────────────────────────────────┤
│  Status: workspace: /path/to/project | model: gpt-4 | 2.3k  │
└─────────────────────────────────────────────────────────────┘
```

### TUI Module Structure

| Module | Responsibility |
|--------|----------------|
| `tui/mod.rs` | TUI entry point, terminal setup/teardown |
| `tui/app.rs` | Application state machine, event loop |
| `tui/ui.rs` | Widget layout and rendering |
| `tui/event.rs` | Keyboard/mouse events, resize handling |
| `tui/components/mod.rs` | Component trait |
| `tui/components/chat.rs` | Conversation display, message list |
| `tui/components/sidebar.rs` | File tree, context panel |
| `tui/components/status.rs` | Status bar with workspace/model info |

## LSP Architecture

### LSP Manager

The LSP manager handles multiple language servers per workspace:

```rust
pub struct LspManager {
    servers: HashMap<LanguageId, LspClient>,
    workspace_root: PathBuf,
}

pub struct LspClient {
    process: Child,
    transport: LspTransport,
    capabilities: ServerCapabilities,
    pending_requests: HashMap<RequestId, Sender<LspResponse>>,
}
```

### LSP Transport

Communication with language servers via stdio:

```rust
pub enum LspTransport {
    Stdio {
        stdin: ChildStdin,
        stdout: BufReader<ChildStdout>,
    },
    Socket { /* TCP/Unix socket */ },
}
```

### Supported Operations

| Operation | LSP Method | Use Case |
|-----------|------------|----------|
| Definition | textDocument/definition | "Go to where X is defined" |
| References | textDocument/references | "Find all uses of X" |
| Hover | textDocument/hover | "What type is X?" |
| Rename | textDocument/rename | "Rename X to Y" |
| Diagnostics | textDocument/publishDiagnostics | Real-time error checking |
| Symbols | textDocument/documentSymbol | "List all functions in file" |

### Language Server Detection

```rust
impl LspManager {
    pub fn detect_language_servers(&self, workspace: &Path) -> Vec<(LanguageId, &str)> {
        let mut servers = Vec::new();
        
        // Check for Cargo.toml -> rust-analyzer
        if workspace.join("Cargo.toml").exists() {
            servers.push((LanguageId::Rust, "rust-analyzer"));
        }
        
        // Check for package.json -> typescript-language-server
        if workspace.join("package.json").exists() {
            servers.push((LanguageId::TypeScript, "typescript-language-server"));
        }
        
        // Check for go.mod -> gopls
        if workspace.join("go.mod").exists() {
            servers.push((LanguageId::Go, "gopls"));
        }
        
        servers
    }
}
```

## Tree-sitter Architecture

### Parser Manager

```rust
pub struct ParserManager {
    parsers: HashMap<LanguageId, Parser>,
    queries: HashMap<LanguageId, QueryMap>,
}

pub struct QueryMap {
    function_definitions: Query,
    struct_definitions: Query,
    imports: Query,
    // ... language-specific queries
}
```

### Supported Languages

```rust
pub enum LanguageId {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
}
```

### Query Examples

```rust
// Find all function definitions (Rust)
const RUST_FUNCTION_QUERY: &str = r#"
(function_item
  name: (identifier) @name)
"#;

// Find all struct definitions (Rust)
const RUST_STRUCT_QUERY: &str = r#"
(struct_item
  name: (type_identifier) @name)
"#;

// Find all imports (TypeScript)
const TS_IMPORT_QUERY: &str = r#"
(import_statement
  source: (string) @source)
"#;
```

### Usage in Agent

```rust
// Agent can use tree-sitter for fast code queries
let parser_manager = ParserManager::new();
let tree = parser_manager.parse_file(&path, &content)?;
let functions = parser_manager.query_functions(&tree, &content)?;

// Or use LSP for more accurate results
let lsp_manager = LspManager::new(workspace);
let definition = lsp_manager.goto_definition(&path, position).await?;
```

## Data Flow

```
User Input (TUI)
       │
       ▼
┌──────────────────┐
│ Parse Intent     │ ─── "Where is X defined?"
└──────────────────┘
       │
       ▼
┌──────────────────────────────────────┐
│ Choose Code Intelligence Tool        │
│                                      │
│  - LSP for accurate cross-file refs  │
│  - Tree-sitter for fast local AST    │
└──────────────────────────────────────┘
       │
       ▼
┌──────────────────┐
│ Execute Query    │
└──────────────────┘
       │
       ▼
┌──────────────────┐
│   LLM Request    │ ─── Context + query results + instruction
└──────────────────┘
       │
       ▼
┌──────────────────┐
│  Update TUI      │ ─── Display results in chat panel
└──────────────────┘
```

## Key Traits

### `Agent`
```rust
#[async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> &'static str;
    async fn execute(&self, instruction: &str) -> Result<AgentOutput>;
    async fn chat(&self, message: &str) -> Result<String>;
}
```

### `Tool`
```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    async fn execute(&self, input: Value) -> Result<Value>;
}
```

### `LlmClient`
```rust
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat(&self, messages: &[Message]) -> Result<String>;
}
```

## Module Responsibilities

| Module | Purpose |
|--------|---------|
| `config` | Configuration loading from ~/.serana/config.toml, provider setup, environment overrides |
| `tui` | Terminal UI, event handling, widget rendering |
| `web` | Web frontend stub (future implementation) |
| `agent` | Agent trait, planning logic, task execution |
| `tools` | Tool trait, file/shell/search implementations |
| `lsp` | LSP client, transport, request/response handling |
| `tree_sitter` | Parser management, syntax queries, language bindings |
| `llm` | LLM client trait, OpenAI implementation, prompt templates |
| `context` | File gathering, context compaction for long sessions |

## Configuration

Configuration is loaded from `~/.serana/config.toml` with environment variable overrides.

### Provider Configuration

```toml
[provider]
name = "openai"  # openai, anthropic, ollama, openrouter, custom
url = ""         # Required only for custom provider

[llm]
model = "gpt-4"
temperature = 0.7

[agent]
max_tokens = 4096
```

### Environment Variables

| Variable | Purpose |
|---------|---------|
| `SERANA_PROVIDER` | Override provider name |
| `SERANA_API_KEY` | API key (takes precedence over config) |
| `SERANA_MODEL` | Override model name |
| `OPENAI_API_KEY` | Fallback API key for OpenAI |
## Extension Points

### Adding a New Language Server

```rust
impl LspManager {
    pub fn register_server(&mut self, lang: LanguageId, command: &str) -> Result<()> {
        let client = LspClient::spawn(command, &self.workspace_root)?;
        self.servers.insert(lang, client);
        Ok(())
    }
}
```

### Adding a New Tree-sitter Language

```rust
impl ParserManager {
    pub fn register_language(&mut self, lang: LanguageId, language: Language) -> Result<()> {
        let mut parser = Parser::new();
        parser.set_language(&language)?;
        self.parsers.insert(lang, parser);
        self.load_queries(lang)?;
        Ok(())
    }
}
```

## Future Work

1. **LSP implementation** - Full LSP client with rust-analyzer support
2. **Tree-sitter integration** - Parse Rust/TypeScript, query syntax trees
3. **Tool execution loop** - Structured output parsing from LLM
4. **Context compaction** - Summarize long conversations
5. **RAG integration** - Index codebase for semantic search
6. **Web frontend** - Alternative interface via WebSocket
7. **Hermes capabilities** - Multi-agent collaboration, self-reflection
