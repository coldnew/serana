# serana-llm — LLM Client Implementations

## Purpose

Implements the `LlmClient` trait from `serana-core` for OpenAI-compatible and Anthropic APIs, plus auxiliary/fallback/registry abstractions.

## Dependencies

- `serana-core` (for `LlmClient`, `Message`, `Config`, `ToolDefinition`, etc.)
- `reqwest` (HTTP client), `async-stream` (streaming), `parking_lot` (RwLock)

## Module Map

| Module | Exports | Purpose |
|--------|---------|---------|
| `openai` | `OpenAiClient` | OpenAI-compatible chat completions API with streaming SSE |
| `anthropic` | `AnthropicClient` | Anthropic Messages API with content-block tool calls |
| `streaming` | `SseStream` | Server-Sent Events parser for HTTP streaming |
| `credential` | `CredentialProvider`, `StaticCredential`, `EnvCredential`, `RefreshableClient` | API key resolution and auto-refresh on 401/403 |
| `auxiliary` | `AuxiliaryClient`, `AuxiliaryConfig`, `AuxiliaryBuilder` | Summarization, title generation, tool validation, code review |
| `fallback` | `FallbackChain`, `FallbackConfig`, `ProviderEntry` | Priority-ordered provider chain with health tracking |
| `registry` | `ProviderRegistry`, `RoutingClient`, `ModelRole` | Role-based model routing (default/smol/slow/plan/commit) |

## Key Implementations

### OpenAiClient

- `chat` — non-streaming, concatenates stream chunks
- `chat_with_tools` — parses `tool_calls` from response (supports both non-streaming and SSE tool calls)
- `chat_stream` — SSE streaming via `SseStream`, yields content deltas
- `chat_with_tools_stream` — accumulates tool call deltas across chunks, yields complete `Message`
- Auth: `Authorization: Bearer <key>` header from `config.api_key()`

### AnthropicClient

- Message format conversion: system → top-level `system` field, tool_use blocks → `content` array
- Tool format: `name` + `description` + `input_schema` (vs OpenAI's `function` wrapper)
- `chat_with_tools` — posts to `<api_url>/messages`, parses content blocks
- `chat_stream` — SSE streaming, extracts `content_block_delta` events

### SseStream

- Parses `data:` lines from HTTP byte stream
- Skips comments (`:`) and `[DONE]` sentinel
- Recursive parser handles partial line buffering

### RefreshableClient

- Decorates any `LlmClient` with auth-error retry
- Calls `CredentialProvider::refresh()` on 401/403/Unauthorized/Forbidden
- Configurable max retries (default: 1)

### AuxiliaryClient

- Timeboxed LLM calls (default 30s timeout)
- Response truncated to `max_tokens * 4` chars
- Built-in tasks: `summarize()` for context compression, `generate_title()` for sessions, `validate_tool_call()` for safety checks, `review_code()` for linting
- `AuxiliaryBuilder` allows separate "fast model" client for cheap tasks

### FallbackChain

- Ordered list of providers, each with health status
- Skips `Status::Failed` providers (after `max_failures` consecutive errors)
- Falls through to next provider on error
- Mark healthy/failed for external health tracking

### ProviderRegistry / RoutingClient

- Maps `ModelRole` (Default, Smol, Slow, Plan, Commit) to `FallbackChain`
- `RoutingClient` implements `LlmClient` — delegates to active role's chain
- Lock-safe: clones the `Arc<FallbackChain>` before `.await`
- Role cycling for debug/override

## Design Decisions

- **SSE parsed inline**: No tungstenite or external SSE crate — `SseStream` is ~80 LOC of hand-rolled parser.
- **Anthropic shares SseStream**: Both providers use the same streaming parser despite different event structures.
- **Role routing starts simple**: `ProviderRegistry::from_config` maps all roles to the same primary client. Custom roles are configured programmatically.
- **Auxiliary is always LlmClient**: Implements the trait so it can be transparently swapped in/out.
