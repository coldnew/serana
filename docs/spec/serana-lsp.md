# serana-lsp — Language Server Protocol Client

## Overview

`serana-lsp` provides a built-in LSP client that spawns and manages language server subprocesses over stdio. It enables code intelligence features (go-to-definition, find-references, hover) without requiring external tools.

**Crate path:** `crates/serana-lsp/`

## Dependencies

- **Internal:** `serana-core` (for `Result`)
- **External:** `tokio` (async process I/O), `serde`, `serde_json`, `anyhow`, `tracing`

## Module Map

| Module | File | Exports | Purpose |
|--------|------|---------|---------|
| `client` | `client.rs` | `LspManager`, `LspClient`, `LanguageId` | Multi-server manager + per-server client |
| `transport` | `transport.rs` | `LspTransport` | Low-level stdio message framing |
| `types` | `types.rs` | `Position`, `Range`, `Location`, `Diagnostic`, `DiagnosticSeverity` | LSP protocol types |

## Architecture

```
LspManager (HashMap<LanguageId, LspClient>)
  │
  ├── LspClient { process, stdin, stdout, next_id }
  │     ├── JSON-RPC 2.0 request/response
  │     └── JSON-RPC 2.0 notification
  │
  └── LspTransport { stdin, stdout }
        └── "Content-Length: N\r\n\r\n{body}" framing
```

## LspManager (`client.rs`)

Multi-language server lifecycle manager.

```rust
pub struct LspManager {
    workspace_root: PathBuf,
    servers: HashMap<LanguageId, LspClient>,
}
```

### Language Detection

`detect_language_servers()` scans the workspace for project markers:

| Marker File | Language | Server Binary |
|-------------|----------|---------------|
| `Cargo.toml` | Rust | `rust-analyzer` |
| `package.json` | TypeScript/JavaScript | `typescript-language-server` |
| `go.mod` | Go | `gopls` |
| `requirements.txt` or `pyproject.toml` | Python | `pylsp` |

### Server Lifecycle

```rust
// Lazy start on first use
manager.definition("src/main.rs", Position { line: 10, character: 5 }).await?;
// ^ auto-starts rust-analyzer if not already running

// Explicit shutdown
manager.shutdown_all().await;
```

### Convenience Methods

| Method | LSP Method | Returns |
|--------|------------|---------|
| `definition(file, position)` | `textDocument/definition` | `Vec<Location>` |
| `references(file, position)` | `textDocument/references` | `Vec<Location>` |
| `hover(file, position)` | `textDocument/hover` | `Option<String>` |

Each method:
1. Calls `ensure_server(language)` to lazy-start if needed
2. Calls `client.did_open(file)` to inform the server about the file
3. Sends the LSP request
4. Returns parsed results

## LspClient (`client.rs`)

Single language server client with async stdio communication.

```rust
pub struct LspClient {
    process: tokio::process::Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
    next_id: AtomicI64,
}
```

### Lifecycle

```
spawn(command, workspace_root)
  → initialize (send "initialize" request, wait for response)
  → initialized (send "initialized" notification)
  → ready (accepts requests)
  → shutdown (send "shutdown" request, wait for response)
  → exit (send "exit" notification, kill process)
```

### Request/Response

```rust
// Send request, wait for matching response
let result: Value = client.request("textDocument/definition", params).await?;

// Send notification (fire-and-forget)
client.notify("textDocument/didOpen", params).await?;
```

- Each request gets a unique `id` (atomically incremented)
- Response matching: reads messages until finding one with matching `id`
- Non-matching responses (notifications, out-of-order) are logged and skipped

### File Notification

Before querying a file, the client sends `textDocument/didOpen`:
```json
{
  "method": "textDocument/didOpen",
  "params": {
    "textDocument": {
      "uri": "file:///path/to/file.rs",
      "languageId": "rust",
      "version": 1,
      "text": "<file contents>"
    }
  }
}
```

## LspTransport (`transport.rs`)

Low-level stdio message framing.

```rust
pub struct LspTransport {
    stdin: Box<dyn Write + Send>,
    stdout: Box<dyn Read + Send>,
}
```

### Wire Format

LSP uses Content-Length header framing:
```
Content-Length: 123\r\n
\r\n
{"jsonrpc":"2.0","id":1,"method":"textDocument/definition","params":{...}}
```

### Methods

- `send(message: &Value) -> Result<()>` — serializes JSON, writes with Content-Length header
- `receive() -> Result<Value>` — reads Content-Length header, reads that many bytes, parses JSON

**Note:** This is a synchronous transport (blocking read/write). The async layer is handled by `LspClient` using `tokio::task::spawn_blocking`.

## LanguageId (`client.rs`)

```rust
pub enum LanguageId {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
}
```

| Variant | Extensions | Server Command |
|---------|------------|----------------|
| `Rust` | `.rs` | `rust-analyzer --stdio` |
| `TypeScript` | `.ts`, `.tsx` | `typescript-language-server --stdio` |
| `JavaScript` | `.js`, `.jsx` | `typescript-language-server --stdio` |
| `Python` | `.py` | `pylsp` |
| `Go` | `.go` | `gopls` |

- `from_extension(ext) -> Option<LanguageId>` — maps file extension to language
- `server_command() -> (&str, Vec<&str>)` — returns (binary, args)

## Types (`types.rs`)

LSP protocol types, 0-indexed:

```rust
pub struct Position {
    pub line: u32,      // 0-based line number
    pub character: u32,  // 0-based column
}

pub struct Range {
    pub start: Position,
    pub end: Position,
}

pub struct Location {
    pub uri: String,   // "file:///path/to/file.rs"
    pub range: Range,
}

pub struct Diagnostic {
    pub range: Range,
    pub severity: Option<DiagnosticSeverity>,
    pub message: String,
    pub source: Option<String>,
}

pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}
```

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| Shared LSP state via `Arc<Mutex<>>` | `serana-tools` wraps `LspManager` so tool calls share server connections |
| Synchronous request/response | LSP servers are single-threaded; blocking per-request is correct |
| No capability negotiation | Sends empty `capabilities: {}` — assumes minimum feature set |
| No `textDocument/didChange` | Single-file queries only; full text sync is future work |
| Lazy server start | Avoids spawning servers for languages not present in workspace |
| Content-Length framing | Standard LSP wire format; no HTTP overhead |
| `spawn_blocking` for I/O | Stdio transport is blocking; wrapped in tokio blocking task |

## Limitations

- **No incremental sync:** Only sends full file text on `didOpen`, no updates on file changes
- **No workspace symbols:** Only document-level queries
- **No code actions:** Only go-to-def, references, hover
- **Single workspace:** One `LspManager` per workspace root
- **No multi-root support:** One workspace root only
