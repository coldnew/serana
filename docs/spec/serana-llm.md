# serana-llm — LLM Client Implementations

## Overview

`serana-llm` implements the `LlmClient` trait from `serana-core` for OpenAI-compatible and Anthropic APIs. It provides streaming SSE support, fallback chains, credential management, role-based model routing, and an auxiliary client for background tasks.

**Crate path:** `crates/serana-llm/`

## Dependencies

- **Internal:** `serana-core` (for `LlmClient`, `Message`, `Config`, `ToolDefinition`)
- **External:** `reqwest` (HTTP), `async-stream` (streaming), `parking_lot` (RwLock), `tokio`, `serde`, `serde_json`, `anyhow`, `futures`, `tracing`, `bytes`

## Module Map

| Module | File | Exports | Purpose |
|--------|------|---------|---------|
| `openai` | `openai.rs` | `OpenAiClient` | OpenAI-compatible chat completions API |
| `anthropic` | `anthropic.rs` | `AnthropicClient` | Anthropic Messages API |
| `streaming` | `streaming.rs` | `SseStream` | Server-Sent Events parser |
| `credential` | `credential.rs` | `CredentialProvider`, `StaticCredential`, `EnvCredential`, `RefreshableClient` | API key management |
| `auxiliary` | `auxiliary.rs` | `AuxiliaryClient`, `AuxiliaryConfig`, `AuxiliaryBuilder` | Background LLM tasks |
| `fallback` | `fallback.rs` | `FallbackChain`, `FallbackConfig`, `ProviderEntry` | Provider failover |
| `registry` | `registry.rs` | `ProviderRegistry`, `RoutingClient`, `ModelRole` | Role-based routing |

## OpenAI Client (`openai.rs`)

Implements `LlmClient` for any OpenAI-compatible API (OpenAI, OpenRouter, local servers).

### API Details

- **Endpoint:** `{api_url}/chat/completions`
- **Auth:** `Authorization: Bearer {api_key}`
- **Streaming:** SSE with `data: {json}` lines, terminated by `data: [DONE]`

### Method Implementations

| Method | Behavior |
|--------|----------|
| `chat()` | Non-streaming POST, extracts `choices[0].message.content` |
| `chat_with_tools()` | Adds `tools` parameter, parses `tool_calls` from response |
| `chat_stream()` | SSE streaming, yields content `delta` chunks |
| `chat_with_tools_stream()` | SSE streaming with tool call accumulation across chunks |

### Tool Call Handling

OpenAI sends tool calls as incremental deltas in streaming mode:
```json
{"choices": [{"delta": {"tool_calls": [{"index": 0, "id": "call_123", "function": {"name": "tool", "arguments": "{\"pa"}}]}}]}
```

The client accumulates these across chunks, concatenating `arguments` strings until the stream ends, then yields a complete `Message::ToolCall`.

## Anthropic Client (`anthropic.rs`)

Implements `LlmClient` for the Anthropic Messages API.

### API Details

- **Endpoint:** `{api_url}/messages`
- **Auth:** `x-api-key: {api_key}`, `anthropic-version: 2023-06-01`
- **Streaming:** SSE with `event: {type}` + `data: {json}` lines

### Message Format Conversion

Serana's internal `Message` format is converted to Anthropic's content block format:

| Internal | Anthropic |
|----------|-----------|
| `Message::System { content }` | Top-level `system` field |
| `Message::User { content }` | `role: "user"`, `content: [{type: "text", text}]` |
| `Message::Assistant { content }` | `role: "assistant"`, `content: [{type: "text", text}]` |
| `Message::ToolCall { id, function }` | `role: "assistant"`, `content: [{type: "tool_use", id, name, input}]` |
| `Message::ToolResult { tool_call_id, content }` | `role: "user"`, `content: [{type: "tool_result", tool_use_id, content}]` |

### Tool Definition Format

Anthropic uses a different tool schema than OpenAI:
```json
{
  "name": "tool_name",
  "description": "what it does",
  "input_schema": { /* JSON Schema */ }
}
```

vs OpenAI's:
```json
{
  "type": "function",
  "function": {
    "name": "tool_name",
    "description": "what it does",
    "parameters": { /* JSON Schema */ }
  }
}
```

### Streaming Events

Anthropic streaming uses typed events:
- `message_start` — contains `message.id` and model info
- `content_block_start` — new content block (text or tool_use)
- `content_block_delta` — incremental content update
- `content_block_stop` — block complete
- `message_delta` — stop reason, usage
- `message_stop` — stream complete

## SSE Parser (`streaming.rs`)

Hand-rolled SSE parser (~80 LOC) shared by both providers.

```rust
pub struct SseStream {
    // Implements Stream<Item = Result<String>>
}
```

- Parses `data:` lines from HTTP byte stream
- Skips comment lines (starting with `:`)
- Returns `None` on `data: [DONE]`
- Handles partial line buffering across chunks
- Yields raw JSON string chunks (provider-specific parsing happens in the client)

## Credential Management (`credential.rs`)

### `CredentialProvider` trait

```rust
#[async_trait]
pub trait CredentialProvider: Send + Sync {
    async fn get_credentials(&self) -> Result<(String, String)>;  // (api_url, api_key)
    async fn refresh(&self) -> Result<()>;
}
```

### Implementations

| Type | Source |
|------|--------|
| `StaticCredential` | Fixed values from `Config` |
| `EnvCredential` | Environment variables, re-read on each call |
| `RefreshableClient` | Wraps any `LlmClient` + `CredentialProvider`, auto-refreshes on 401/403 |

### `RefreshableClient`

Decorates any `LlmClient` with automatic credential refresh:

```rust
pub struct RefreshableClient<T: LlmClient> {
    inner: T,
    credentials: Arc<dyn CredentialProvider>,
    max_retries: u32,  // default: 1
}
```

On auth error (HTTP 401/403 or error message containing "Unauthorized"/"Forbidden"):
1. Calls `credentials.refresh()`
2. Retries the original request
3. Up to `max_retries` times

## Auxiliary Client (`auxiliary.rs`)

Lightweight LLM wrapper for background tasks that aren't part of the main conversation.

```rust
pub struct AuxiliaryClient {
    client: Arc<dyn LlmClient>,
    config: AuxiliaryConfig,
}

pub struct AuxiliaryConfig {
    pub timeout_secs: u64,       // default: 30
    pub max_tokens: usize,       // default: 2048
    pub max_response_chars: usize, // default: max_tokens * 4
}
```

### Built-in Tasks

| Method | Purpose | Default timeout |
|--------|---------|-----------------|
| `summarize(text)` | Compress text for context management | 30s |
| `generate_title(conversation)` | Create session title from messages | 15s |
| `validate_tool_call(tool_name, args)` | Safety check before execution | 10s |
| `review_code(file_content)` | Lint/suggest improvements | 30s |

### AuxiliaryBuilder

Allows configuring a separate "fast model" for cheap tasks:

```rust
let auxiliary = AuxiliaryBuilder::new(main_client)
    .with_fast_model(fast_client)  // uses this for simple tasks
    .with_timeout(20)
    .build();
```

## Fallback Chain (`fallback.rs`)

Priority-ordered provider chain with health tracking.

```rust
pub struct FallbackChain {
    providers: Vec<ProviderEntry>,
}

pub struct ProviderEntry {
    client: Arc<dyn LlmClient>,
    health: RwLock<HealthStatus>,
    fail_count: AtomicU32,
    max_failures: u32,  // default: 3
}

pub enum HealthStatus { Healthy, Degraded, Failed }
```

### Behavior

1. Tries providers in order
2. On error: increments `fail_count`, marks `Failed` after `max_failures` consecutive errors
3. Skips `Failed` providers
4. On success: resets `fail_count`, marks `Healthy`
5. `mark_failed()` / `mark_healthy()` for external health updates

### `LlmClient` Implementation

Delegates to the first healthy provider. If all fail, returns the last error.

## Provider Registry (`registry.rs`)

Role-based model routing.

```rust
pub enum ModelRole {
    Default,   // primary model for general tasks
    Smol,      // fast/cheap model for simple tasks
    Slow,      // powerful model for complex reasoning
    Plan,      // planning and analysis
    Commit,    // commit message generation
}

pub struct ProviderRegistry {
    chains: HashMap<ModelRole, Arc<FallbackChain>>,
}

pub struct RoutingClient {
    registry: ProviderRegistry,
    active_role: RwLock<ModelRole>,
}
```

### Configuration

`ProviderRegistry::from_config(config)` maps all roles to the same primary client by default. Custom role mappings are configured programmatically:

```rust
let registry = ProviderRegistry::from_config(&config)?;
registry.add_to_role(ModelRole::Smol, cheap_client);
registry.add_to_role(ModelRole::Slow, powerful_client);
```

### `RoutingClient`

Implements `LlmClient` by delegating to the active role's chain:

```rust
let routing = RoutingClient::new(registry);
routing.set_role(ModelRole::Slow);  // switch to powerful model
let response = routing.chat(&messages).await?;
```

- `cycle_role()` — rotates through roles for debugging
- Lock-safe: clones `Arc<FallbackChain>` before `.await` to avoid holding RwLock across yield points

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| Hand-rolled SSE parser | ~80 LOC, no external dependency, shared by both providers |
| Anthropic shares `SseStream` | Different event structures but same `data:` line format |
| Role routing starts simple | `from_config` maps all roles to same client; custom roles configured programmatically |
| Auxiliary implements `LlmClient` | Can be transparently swapped in/out |
| Credential refresh on 401/403 | Handles key rotation without restart |
| Tool call delta accumulation | OpenAI streams tool calls incrementally; must accumulate before yielding complete call |
