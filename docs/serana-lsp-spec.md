# serana-lsp — Language Server Protocol Client

## Purpose

Built-in LSP client that spawns and manages language server subprocesses over stdio. Provides code intelligence (go-to-definition, references, hover) without requiring external tools.

## Dependencies

- `serana-core` (Result), `tokio` (async process I/O)

## Module Map

| Module | Exports | Purpose |
|--------|---------|---------|
| `client` | `LspManager`, `LspClient`, `LanguageId` | Multi-server lifecycle manager + per-server JSON-RPC client |
| `transport` | `LspTransport` | Low-level stdio message framing |
| `types` | `Position`, `Range`, `Location`, `Diagnostic`, `DiagnosticSeverity` | LSP protocol types |

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

## LspManager

- `detect_language_servers()` — scans workspace for `Cargo.toml` → Rust, `package.json` → TS/JS, `go.mod` → Go, `requirements.txt`/`pyproject.toml` → Python
- `start_server()` — spawns server process (e.g., `rust-analyzer --stdio`), sends `initialize`/`initialized` handshake
- `ensure_server()` — lazy start on first use per language
- `definition()`, `references()`, `hover()` — convenience methods that auto-start the right server

## LspClient

- Lifecycle: `spawn` → `initialize` → `ready` → `shutdown`
- `request()` — sends method + params, waits for matching `id` in response, returns `result`
- `notify()` — fire-and-forget (no id, no response)
- `did_open()` — tells the server about a file
- `shutdown()` — sends `shutdown` + `exit`, kills process
- Handles Content-Length header parsing in `receive()`
- Skips responses with non-matching ids (out-of-order notifications)

## LanguageId

| Variant | Extension | Server Binary |
|---------|-----------|---------------|
| `Rust` | `.rs` | `rust-analyzer` |
| `TypeScript` | `.ts`, `.tsx` | `typescript-language-server` |
| `JavaScript` | `.js`, `.jsx` | `typescript-language-server` |
| `Python` | `.py` | `pylsp` |
| `Go` | `.go` | `gopls` |

## Types

- `Position { line: u32, character: u32 }` — 0-based
- `Range { start: Position, end: Position }`
- `Location { uri: String, range: Range }`
- `Diagnostic { range, severity, message, source }`

## Design Decisions

- **Shared LSP state**: `serana-tools` wraps `LspManager` in `Arc<Mutex<>>` so tool calls share server connections.
- **Synchronous request/response loop**: Each request blocks until the matching response arrives. This is correct for LSP since servers are single-threaded.
- **No capability negotiation**: Sends empty `capabilities: {}` — assumes minimum feature set works.
- **No workspace/didChange yet**: Single-file queries only. Full text sync is future work.
