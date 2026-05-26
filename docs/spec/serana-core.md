# serana-core — Core Types & Traits

## Overview

`serana-core` is the foundational crate of the Serana workspace. It defines all shared traits, types, and utilities that every other crate depends on. It has zero internal crate dependencies — it is the leaf of the dependency tree.

**Crate path:** `crates/serana-core/`

## Dependencies

External only: `tokio`, `async-trait`, `serde`, `serde_json`, `anyhow`, `thiserror`, `futures`, `chrono`, `tracing`, `pin-project-lite`, `toml`, `dirs`.

## Module Map

| Module | File | Purpose |
|--------|------|---------|
| `agent` | `agent.rs` | Core `Agent` trait and output types |
| `tool` | `tool.rs` | Core `Tool` trait for agent-callable operations |
| `llm_client` | `llm_client.rs` | Core `LlmClient` trait for LLM providers |
| `message` | `message.rs` | Universal message format for LLM chat |
| `config` | `config.rs` | TOML-based configuration loader |
| `callbacks` | `callbacks.rs` | Event system for real-time progress updates |
| `compression` | `compression.rs` | Context window compression configuration |
| `iteration_budget` | `iteration_budget.rs` | Atomic iteration counter for loop prevention |
| `interruptible` | `interruptible.rs` | Cancellation support for async operations |
| `token_counter` | `token_counter.rs` | Character-based token estimation |
| `tool_approval` | `tool_approval.rs` | Tool safety classification and approval |
| `verification` | `verification.rs` | Build+test verification for self-modification |
| `meta_cognition` | `meta_cognition.rs` | Self-evolution tracking and lessons learned |
| `context` | `context.rs` | Workspace context structure |

## Core Traits

### `Agent` trait (`agent.rs`)

The central abstraction for any agent implementation.

```rust
#[async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, instruction: &str) -> Result<AgentOutput>;
    async fn chat(&self, message: &str) -> Result<String>;
}
```

- `execute()` — full agent loop with tool calling, returns structured output
- `chat()` — simplified single-turn chat, returns text only

**Associated types:**
- `AgentOutput { text: String, tool_calls: Vec<ToolCall>, iterations: u32 }`
- `ToolCall { name: String, arguments: Value, result: Option<Value> }`

### `Tool` trait (`tool.rs`)

Interface for all agent-callable tools.

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;  // JSON Schema
    async fn execute(&self, input: Value) -> Result<Value>;
}
```

- `parameters()` returns a JSON Schema describing the tool's input format
- `execute()` accepts and returns `serde_json::Value` for flexibility

### `LlmClient` trait (`llm_client.rs`)

Interface for LLM provider implementations.

```rust
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat(&self, messages: &[Message]) -> Result<Message>;
    async fn chat_with_tools(&self, messages: &[Message], tools: &[ToolDefinition]) -> Result<Message>;
    async fn chat_stream(&self, messages: &[Message]) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>>;
    async fn chat_with_tools_stream(&self, messages: &[Message], tools: &[ToolDefinition]) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>>;
}
```

- `chat_stream` has a default implementation that wraps `chat` in a single-item stream
- `ToolDefinition` contains `name`, `description`, and `parameters` (JSON Schema)

## Core Types

### `Message` enum (`message.rs`)

The universal message format used across all LLM interactions.

```rust
pub enum Message {
    System { content: String },
    User { content: String },
    Assistant { content: String },
    ToolCall { id: String, function: FunctionCall },
    ToolResult { tool_call_id: String, content: String },
}
```

- Implements `Serialize`/`Deserialize` for direct JSON marshaling
- `FunctionCall { name: String, arguments: String }`

### `Config` struct (`config.rs`)

Loads configuration from `~/.serana/config.toml`.

```rust
pub struct Config {
    pub providers: HashMap<String, ProviderConfig>,
    pub default_provider: String,
    pub default_model: String,
    pub workspace: Option<String>,
}

pub struct ProviderConfig {
    pub api_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
}
```

- `api_url(provider)` — resolves provider URL (supports env var override via `SERANA_<PROVIDER>_API_URL`)
- `api_key(provider)` — resolves API key (supports env var override via `SERANA_<PROVIDER>_API_KEY` or `<PROVIDER>_API_KEY`)
- `model(provider)` — resolves model name (supports env var override via `SERANA_<PROVIDER>_MODEL`)
- Default providers: `openai`, `anthropic`, `openrouter`
- Env var overrides take precedence over config file values

### `AgentCallbacks` (`callbacks.rs`)

Event system for real-time progress updates from the agent.

```rust
pub struct AgentCallbacks {
    pub on_tool_progress: Option<Arc<dyn Fn(&str, &str) + Send + Sync>>,
    pub on_thinking: Option<Arc<dyn Fn() + Send + Sync>>,
    pub on_streaming: Option<Arc<dyn Fn(&str) + Send + Sync>>,
    pub on_status_change: Option<Arc<dyn Fn(AgentStatus) + Send + Sync>>,
}
```

- Builder pattern: `AgentCallbacks::new().on_streaming(|delta| ...).on_thinking(|| ...)`
- `AgentStatus` enum: `Idle`, `Thinking`, `Running`, `ExecutingTool(String)`, `Compressing`, `Done`

### `IterationBudget` (`iteration_budget.rs`)

Atomic counter preventing infinite agent loops.

```rust
pub struct IterationBudget {
    max_iterations: AtomicU32,
    current: AtomicU32,
}
```

- `can_continue() -> bool` — checks if budget remains
- `increment()` — atomically increments counter
- `is_exhausted() -> bool` — true when current >= max
- `progress() -> f32` — returns 0.0..1.0
- Default: 50 parent iterations, 20 subagent iterations

### `CancelToken` (`interruptible.rs`)

Atomic boolean for cooperative cancellation.

```rust
pub struct CancelToken {
    cancelled: AtomicBool,
}
```

- `cancel()` — sets flag to true
- `is_cancelled() -> bool` — checks flag
- `InterruptibleApiCall::new(token, future)` — wraps a future, returns `Err(Cancelled)` if token fires mid-execution

### `TokenCounter` (`token_counter.rs`)

Heuristic token estimation without BPE.

```rust
pub struct TokenCounter;
impl TokenCounter {
    pub fn estimate(text: &str) -> usize {
        text.len() / 4  // ~4 chars per token
    }
}
```

- Conservative estimate, fast — suitable for budget decisions, not billing

### `ToolApproval` (`tool_approval.rs`)

Tool safety classification and approval system.

```rust
pub enum RiskLevel { Safe, Low, Medium, High }
pub enum ApprovalMode { Auto, Interactive, Whitelist, Smart }

pub struct ToolApproval {
    pub mode: ApprovalMode,
    pub whitelist: HashSet<String>,
    pub blacklist: HashSet<String>,
}
```

- `classify_risk(tool_name) -> RiskLevel` — maps tool names to risk levels
- `requires_approval(tool_name) -> bool` — checks mode + risk + lists
- Default risk levels: file read = Safe, file write = Low, shell = High, git commit = Medium

### `MetaCognition` (`meta_cognition.rs`)

Tracks self-modification history and lessons learned.

```rust
pub struct MetaCognition {
    pub records: Vec<ModificationRecord>,
    pub lessons: Vec<String>,
}

pub struct ModificationRecord {
    pub timestamp: DateTime<Utc>,
    pub kind: ModificationKind,
    pub description: String,
    pub success: bool,
    pub lessons: Vec<String>,
}
```

- `record(modification)` — adds to history
- `add_lesson(lesson)` — adds to lessons list
- `get_stats() -> ModificationStats` — aggregates success/failure counts
- `get_recent_failures(n)` — returns last N failed modifications

### `VerificationSystem` (`verification.rs`)

Build+test runner for safe self-modification.

```rust
pub struct VerificationSystem {
    pub workspace_root: PathBuf,
}

pub struct VerificationResult {
    pub build_success: bool,
    pub test_success: bool,
    pub build_output: String,
    pub test_output: String,
}
```

- `verify() -> Result<VerificationResult>` — runs `cargo build --release` + `cargo test`
- `create_snapshot() -> Result<StateSnapshot>` — creates git stash
- `rollback(snapshot) -> Result<()>` — restores from snapshot

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| `anyhow::Result` everywhere | No custom error enum at this level — domain errors go in consuming crates |
| `Message` is untagged serde | Direct serialization to OpenAI/Anthropic JSON without conversion |
| Callbacks are `Arc<dyn Fn>` | Avoids tokio dependency for callers wanting sync callbacks |
| `chat_stream` has default impl | Wraps `chat` in single-item stream — implementors opt in to true streaming |
| `TokenCounter` is heuristic | 4 chars/token is fast and good enough for budget decisions |
| `CompressionConfig` protects last N | Avoids summarizing the most recent conversation turns |
| `Tool` takes/returns `Value` | Maximum flexibility — tools define their own schemas |

## Usage Patterns

### Creating a custom tool

```rust
use serana_core::{Tool, Result};
use serde_json::{json, Value};
use async_trait::async_trait;

struct MyTool;

#[async_trait]
impl Tool for MyTool {
    fn name(&self) -> &str { "my_tool" }
    fn description(&self) -> &str { "Does something useful" }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "input": { "type": "string" }
            },
            "required": ["input"]
        })
    }
    async fn execute(&self, input: Value) -> Result<Value> {
        let param = input["input"].as_str().unwrap_or("");
        Ok(json!({ "result": format!("processed: {}", param) }))
    }
}
```

### Using CancelToken

```rust
use serana_core::CancelToken;
use std::sync::Arc;

let token = CancelToken::new();
let token_clone = token.clone();

// In another task:
tokio::spawn(async move {
    tokio::time::sleep(Duration::from_secs(5)).await;
    token_clone.cancel();
});

// In the main task:
if token.is_cancelled() {
    println!("Operation was cancelled");
}
```

### Loading config

```rust
use serana_core::Config;

let config = Config::load()?;  // reads ~/.serana/config.toml
let api_key = config.api_key("openai")?;
let model = config.model("anthropic")?;
```
