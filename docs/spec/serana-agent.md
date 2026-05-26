# serana-agent — Agent Orchestration Engine

## Overview

`serana-agent` is the agent orchestration layer. It implements `HermesAgent` (the primary agent type) along with all machinery for running agent loops: turn execution, tool handling, context compression, session persistence, subagent spawning, and prompt building.

**Crate path:** `crates/serana-agent/`

## Dependencies

- **Internal:** `serana-core`, `serana-llm`, `serana-tools`
- **External:** `tokio`, `async-trait`, `serde`, `serde_json`, `anyhow`, `futures`, `tracing`, `chrono`, `rusqlite`, `uuid`, `dirs`, `regex`, `toml`

## Module Map

| Module | File | Exports | Purpose |
|--------|------|---------|---------|
| `hermes` | `hermes.rs` | `HermesAgent` | Primary agent implementation |
| `engine` | `engine.rs` | `AgentEngine` | Core execution loop |
| `factory` | `factory.rs` | `AgentFactory` | Agent construction with proper tool profiles |
| `turn_runner` | `turn_runner.rs` | `TurnRunner` | Streams LLM response, accumulates deltas |
| `tool_executor` | `tool_executor.rs` | `execute_tools_concurrent()` | Executes tool calls |
| `tool_turn` | `tool_turn.rs` | `handle_tool_turn()` | Orchestrates tool execution with meta-cognition |
| `tool_call_validator` | `tool_call_validator.rs` | `ToolCallValidator` | Validates tool call IDs, names, arguments |
| `prompt_builder` | `prompt_builder.rs` | `PromptBuilder` | Builds system prompt from workspace, skills, tools |
| `run_state` | `run_state.rs` | `AgentRunState` | Message list and tool call history during execution |
| `lifecycle` | `lifecycle.rs` | `AgentLifecycle` | Iteration budget and cancellation checks |
| `compressor` | `compressor.rs` | `ContextCompressor` | Token estimation and message compression |
| `compression_gate` | `compression_gate.rs` | `CompressionGate` | Wraps compressor with LLM/auxiliary client |
| `session` | `session.rs` | `SessionStore` | SQLite-backed session persistence |
| `session_recorder` | `session_recorder.rs` | `SessionRecorder` | Records messages and tool calls during execution |
| `stream_rules` | `stream_rules.rs` | `StreamRuleEngine` | Time-Traveling Stream Rules (TTSR) |
| `message_validation` | `message_validation.rs` | `validate_message_alternation()`, `fix_message_alternation()` | Role sequencing |
| `runtime_config` | `runtime_config.rs` | `AgentRuntimeConfig`, `AgentPromptConfig` | Configuration structs |
| `subagent` | `subagent.rs` | `SubagentSpawner`, `SubagentTask`, `SubagentResult` | Child agent spawning |
| `coding` | `coding.rs` | `CodingAgent` | Deprecated alias for `HermesAgent` |
| `compactor` | `compactor.rs` | `ContextCompactor` | (Stub) Future context compaction |
| `gatherer` | `gatherer.rs` | `ContextGatherer` | (Stub) Future relevant file gathering |

## HermesAgent (`hermes.rs`)

The primary agent implementation. Uses builder pattern.

```rust
pub struct HermesAgent {
    llm: Arc<dyn LlmClient>,
    tools: ToolRegistry,
    callbacks: AgentCallbacks,
    budget: IterationBudget,
    session: Option<SessionStore>,
    compressor: Option<ContextCompressor>,
    cancel_token: Option<CancelToken>,
    skills: Vec<Skill>,
    auxiliary: Option<AuxiliaryClient>,
}
```

### Builder Methods

```rust
let agent = HermesAgent::hermes(llm)
    .with_tools(registry)
    .with_callbacks(callbacks)
    .with_budget(IterationBudget::new(50))
    .with_session(session_store)
    .with_compressor(compressor)
    .with_cancel_token(cancel_token)
    .with_skills(skills)
    .with_auxiliary(auxiliary);
```

### Agent Trait Implementation

```rust
impl Agent for HermesAgent {
    fn name(&self) -> &str { "hermes" }

    async fn execute(&self, instruction: &str) -> Result<AgentOutput> {
        let engine = AgentEngine::new(/* ... */);
        engine.execute(instruction).await
    }

    async fn chat(&self, message: &str) -> Result<String> {
        let output = self.execute(message).await?;
        Ok(output.text)
    }
}
```

## AgentEngine (`engine.rs`)

The core execution loop.

```rust
pub struct AgentEngine {
    llm: Arc<dyn LlmClient>,
    tools: ToolRegistry,
    messages: Vec<Message>,
    budget: IterationBudget,
    compressor: Option<ContextCompressor>,
    cancel_token: Option<CancelToken>,
    callbacks: AgentCallbacks,
    auxiliary: Option<AuxiliaryClient>,
}
```

### Execution Flow

```
1. Build system prompt (PromptBuilder)
2. Prepare messages: system + user instruction
3. Convert tools to ToolDefinition[]
4. Iteration loop:
   a. Check cancel token → return if cancelled
   b. Check iteration budget → fire exhausted callback if done
   c. LLM call (chat_with_tools_stream)
   d. Stream response via TurnRunner
   e. If tool calls in response:
      - Validate tool calls (ToolCallValidator)
      - Execute tools (handle_tool_turn)
      - Inject tool results as messages
      - Check compression (CompressionGate)
      - Increment budget, continue loop
   f. If text response:
      - Return AgentOutput { text, iterations }
5. Return AgentOutput
```

## AgentFactory (`factory.rs`)

Constructs `HermesAgent` with proper tool profiles.

```rust
pub struct AgentFactory {
    config: AgentRuntimeConfig,
}

impl AgentFactory {
    pub fn hermes(config: AgentRuntimeConfig) -> Self { /* ... */ }

    pub fn build(self, llm: Arc<dyn LlmClient>) -> HermesAgent { /* ... */ }

    pub fn build_tools(&self) -> ToolRegistry { /* ... */ }
}
```

### Factory Methods

- `hermes(config)` — creates factory for full Hermes agent
- `custom(config)` — creates factory with custom configuration
- `with_lsp_tools(manager)` — enables LSP tools
- `with_skill_tool(workspace)` — enables skill creation tool
- `build(llm)` — constructs the agent

## PromptBuilder (`prompt_builder.rs`)

Assembles system prompt from multiple sources.

```rust
pub struct PromptBuilder {
    workspace: PathBuf,
    personality: Option<String>,
    memory: Option<String>,
    skills: Vec<Skill>,
    tool_descriptions: String,
}
```

### Prompt Sections (in order)

1. **Core prompt** — Serana identity, workspace name, code editing rules
2. **Personality** — loaded from `<workspace>/.serana/personality.md`
3. **Project memory** — from `<workspace>/.serana/memory.md`
4. **User memory** — from `~/.serana/user-memory.md`
5. **Skills** — injected as bullet list with descriptions
6. **Context files** — `.serana/context/` directory contents
7. **Tool descriptions** — from `ToolRegistry::describe_all()`
8. **Tool guidance** — JSON tool call format instructions

## ContextCompressor (`compressor.rs`)

Monitors token usage and compresses old messages.

```rust
pub struct ContextCompressor {
    thresholds: CompressionThresholds,
    protect_last_n: usize,  // default: 10
}
```

### Compression Thresholds

| Threshold | Default | Action |
|-----------|---------|--------|
| Preflight | 50% | Start background compression |
| Gateway | 85% | Block next LLM call until compression completes |

### Compression Process

1. Estimate total tokens via `TokenCounter`
2. If threshold exceeded:
   - Split messages at `protect_last_n` boundary
   - Keep system messages untouched
   - Summarize old messages via LLM (or AuxiliaryClient)
   - Insert summary as `Message::user("[Previous conversation summary]\n...")`
   - Keep recent messages unchanged

### CompressionGate (`compression_gate.rs`)

Wraps compressor with LLM client:

```rust
pub enum CompressionResult {
    Compressed(Vec<Message>),
    Unchanged,
}

pub struct CompressionGate {
    compressor: ContextCompressor,
    client: Arc<dyn LlmClient>,
}
```

## SessionStore (`session.rs`)

SQLite-backed session persistence.

```rust
pub struct SessionStore {
    conn: rusqlite::Connection,
}
```

### Database Schema

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    title TEXT,
    created_at TEXT,
    updated_at TEXT,
    workspace TEXT,
    model TEXT
);

CREATE TABLE messages (
    id INTEGER PRIMARY KEY,
    session_id TEXT REFERENCES sessions(id),
    role TEXT,
    content TEXT,
    created_at TEXT
);

CREATE TABLE tool_calls (
    id INTEGER PRIMARY KEY,
    message_id INTEGER REFERENCES messages(id),
    tool_name TEXT,
    arguments TEXT,
    result TEXT,
    created_at TEXT
);
```

### Methods

| Method | Purpose |
|--------|---------|
| `new(db_path)` | Open/create database |
| `default_location()` | `~/.local/share/serana/sessions.db` |
| `init()` | Create tables if not exist |
| `create_session(workspace, model)` | Create new session, return ID |
| `save_message(session_id, role, content)` | Persist message |
| `save_tool_call(message_id, tool_name, args, result)` | Persist tool call |
| `list_sessions(limit)` | Recent sessions |
| `get_session(id)` | Load session with messages |
| `search_messages(query)` | SQL LIKE search across messages |

## SubagentSpawner (`subagent.rs`)

Spawns independent child agents for parallel tasks.

```rust
pub struct SubagentSpawner {
    llm: Arc<dyn LlmClient>,
}

pub struct SubagentTask {
    pub instruction: String,
    pub tools: Option<Vec<String>>,
    pub budget: Option<u32>,
}

pub struct SubagentResult {
    pub task_id: String,
    pub output: Result<AgentOutput>,
}
```

### Usage

```rust
let spawner = SubagentSpawner::new(llm);

let tasks = vec![
    SubagentTask { instruction: "Find all TODO comments".into(), .. },
    SubagentTask { instruction: "Analyze test coverage".into(), .. },
];

let results = spawner.execute_tasks(tasks).await;
// Each task runs as independent HermesAgent with own budget (default: 20)
```

## StreamRuleEngine (`stream_rules.rs`)

Time-Traveling Stream Rules (TTSR). Monitors streaming output for regex patterns and injects system reminders.

```rust
pub struct StreamRuleEngine {
    rules: Vec<StreamRule>,
    injected: HashSet<String>,
}

pub struct StreamRule {
    pub name: String,
    pub pattern: Regex,
    pub injection: String,
}
```

### Behavior

1. Monitors accumulated `chat_stream` output
2. On regex match: aborts current request, injects rule's `injection` as system message, retries
3. Prevents double-injection via `injected` HashSet
4. Rules loaded from `~/.serana/stream-rules.toml` or workspace `.serana/stream-rules.toml`

## Message Validation (`message_validation.rs`)

Ensures proper role sequencing in message lists.

**Valid sequence:** `system → user → assistant → (tool → assistant) → user → ...`

- `validate_message_alternation(messages)` — returns errors if sequence is invalid
- `fix_message_alternation(messages)` — merges consecutive same-role messages, injects placeholders for missing roles

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| Sequential tool execution | `execute_tools_concurrent` iterates sequentially; true concurrency (join_all) is safe but not yet enabled |
| AgentCallbacks is pass-by-reference | Not cloned per iteration; lives in-memory |
| Compression at 50%/85% | Preflight starts background compression; gateway blocks until done |
| Session store is secondary | Messages flow through agent first, persisted after response |
| Subagents get own budgets | Default 20 iterations; prevents runaway child agents |
| Stream rules abort + retry | Allows mid-stream correction without restarting entire conversation |
| Message validation on every turn | Prevents LLM API errors from malformed role sequences |
