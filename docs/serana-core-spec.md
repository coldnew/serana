# serana-core — Core Types & Traits

## Purpose

Holds all foundational types, traits, and utilities shared across every crate in the Serana workspace. No other crate should depend on `serana-core` in reverse — it is the zeroth dependency.

## Dependencies

- `tokio`, `async-trait`, `serde`, `serde_json`, `anyhow`, `thiserror`, `futures`, `chrono`, `tracing`, `pin-project-lite`, `toml`, `dirs`

## Exports

### Core Traits

| Trait | File | Purpose |
|-------|------|---------|
| `Agent` | `agent.rs` | Agent interface: `execute(&str) -> AgentOutput`, `chat(&str) -> String` |
| `Tool` | `tool.rs` | Tool interface: `name()`, `description()`, `parameters()`, `execute(Value) -> Value` |
| `LlmClient` | `llm_client.rs` | LLM interface: `chat`, `chat_with_tools`, `chat_stream`, `chat_with_tools_stream` |

### Core Types

| Type | File | Purpose |
|------|------|---------|
| `Config` | `config.rs` | ~/.serana/config.toml loader, env var overrides, provider URL resolution |
| `Message` | `message.rs` | Roles: system/user/assistant/tool, supports tool calls and results |
| `ToolCallData`, `FunctionCall` | `message.rs` | LLM tool call structure (id, type, function name + args) |
| `AgentOutput`, `ToolCall` | `agent.rs` | Agent execution results |
| `ToolDefinition`, `FunctionDefinition` | `llm_client.rs` | Tool schemas sent to LLM |
| `Context` | `context.rs` | Workspace root, relevant files, conversation history |
| `Result<T>` | `lib.rs` | `anyhow::Result<T>` alias |
| `AgentStatus` | `callbacks.rs` | State machine: Idle, Thinking, Running, ExecutingTool, etc. |
| `AgentCallbacks`, `CallbackState` | `callbacks.rs` | Fire-and-forget callback surfaces for tool progress, streaming, status |
| `CancelToken`, `InterruptibleApiCall` | `interruptible.rs` | Atomic-bool cancellation for async HTTP calls |
| `IterationBudget`, `TokenCost` | `iteration_budget.rs` | Iteration counting with configurable limits (default: 50 parent, 20 subagent) |
| `CompressionConfig`, `CompressionThresholds`, `CompressionDecision` | `compression.rs` | Context window pressure monitoring (preflight 50%, gateway 85%) |
| `ApprovalMode`, `ApprovalDecision`, `RiskLevel`, `ToolApproval` | `tool_approval.rs` | Tool permission system with Auto/Interactive/Whitelist/Smart modes |
| `TokenCounter` | `token_counter.rs` | Character-based token estimation (~4 chars/token) |
| `MetaCognition`, `ModificationRecord`, `ModificationKind`, `ModificationStats` | `meta_cognition.rs` | Self-evolution tracking: records modifications, collects lessons |
| `VerificationSystem`, `VerificationResult`, `StateSnapshot` | `verification.rs` | Build+test runner, git snapshot creation and rollback |

## Design Decisions

- **`anyhow` for `Result`**: All public APIs use `crate::Result<T>` = `anyhow::Result<T>`. No custom error enum at this level — domain-specific errors go in consuming crates.
- **`Message` is untagged serde**: Allows direct serialization to OpenAI/Anthropic JSON formats without conversion.
- **Callbacks are `Arc<dyn Fn>`**: Not channels — avoids tokio dependency for callers that want sync callbacks.
- **`LlmClient` default-streaming**: `chat_stream` falls back to wrapping `chat` in a single-item stream, so implementors can opt in to true streaming.
- **`TokenCounter` is heuristic**: 4 chars/token is a conservative estimate. Not BPE — fast and good enough for budget decisions.
- **`CompressionConfig` protects last N messages**: Avoids summarizing the most recent conversation turns.
