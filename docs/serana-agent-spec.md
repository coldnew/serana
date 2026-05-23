# serana-agent — Agent Implementation

## Purpose

The main agent that orchestrates LLM calls, tool execution, context management, and session persistence. Implements the `Agent` trait from `serana-core`.

## Dependencies

- `serana-core`, `serana-llm`, `serana-tools`
- `rusqlite`, `uuid`, `regex`

## Module Map

| Module | Exports | Purpose |
|--------|---------|---------|
| `coding` | `CodingAgent` | Main agent implementation: execute loop, system prompt, tool integration |
| `prompt_builder` | `PromptBuilder` | Assembles system prompt from personality, memory, skills, context files |
| `tool_executor` | `execute_tools_concurrent`, `ToolExecutionResult` | Iterates tool calls, invokes via ToolRegistry |
| `compressor` | `ContextCompressor` | Checks token usage, summaries old messages to stay within budget |
| `compactor` | `ContextCompactor` | (Stub) Future context compaction |
| `gatherer` | `ContextGatherer` | (Stub) Gathers relevant files for context |
| `session` | `SessionStore`, `Session`, `StoredMessage`, `SearchResult` | SQLite-backed session persistence |
| `subagent` | `SubagentSpawner`, `SubagentTask`, `SubagentResult` | Spawn independent child agents for parallel tasks |
| `message_validation` | `validate_message_alternation`, `fix_message_alternation` | Ensures correct user/assistant/tool role alternation |
| `stream_rules` | `StreamRuleEngine`, `StreamRule` | TTSR: pattern-match streaming output, inject system reminders mid-stream |

## CodingAgent

### Execute Loop

1. Build system prompt (via `PromptBuilder`)
2. Convert tools to `ToolDefinition[]`
3. Prepare messages: system + user instruction
4. **Iteration loop** (until budget exhausted or done):
   a. LLM call (`chat_with_tools`)
   b. If tool call response:
      - Execute tools (sequential, via `execute_tools_concurrent`)
      - Inject tool results as messages
      - Check compression (preflight/gateway)
      - Check iteration budget
   c. If text response: return as `AgentOutput`
5. Fire status callbacks throughout

### Builder Methods

- `with_callbacks()` — attach `AgentCallbacks`
- `with_budget()` — custom `IterationBudget` (default: 50 iterations)
- `with_workspace()` — set workspace root for `PromptBuilder`
- `with_session()` — attach `SessionStore` for persistence
- `with_compressor()` — custom `ContextCompressor`
- `with_cancel_token()` — attach `CancelToken` for cancellation
- `with_skills()` — inject skill descriptions into system prompt
- `with_auxiliary()` — attach `AuxiliaryClient` for compression

## PromptBuilder

Assembles system prompt from sections (in order):

1. **Core prompt** — Serana identity, workspace name, code editing rules
2. **Personality** — loaded from `<workspace>/.serana/personality.md`
3. **Project memory** — from `<workspace>/.serana/memory.md`
4. **User memory** — from `~/.serana/user-memory.md`
5. **Skills** — injected as a bullet list with descriptions
6. **Context files** — `.serana/context/` directory contents
7. **Tool descriptions** — from `ToolRegistry::describe_all()`
8. **Tool guidance** — JSON tool call format instructions

## SessionStore

- SQLite database at `~/.local/share/serana/sessions.db`
- Tables: `sessions`, `messages`, `tool_calls`
- Methods: `create_session()`, `save_message()`, `save_tool_call()`, `list_sessions()`, `get_session()`, `search_messages()`
- Search via SQL `LIKE` (not FTS)

## SubagentSpawner

- Spawns independent `CodingAgent` instances as `tokio::spawn` tasks
- Each subagent gets its own `IterationBudget` (default: 20)
- Results collected as `JoinHandle<SubagentResult>`
- Uses shared `Arc<dyn LlmClient>` for the LLM

## ContextCompressor

- Two paths: direct LLM summarization or `AuxiliaryClient` summarization
- Splits messages at `protect_last_n` boundary (default: 10)
- Keeps system messages untouched
- Inserts a summary as a `Message::user("[Previous conversation summary]\n...")`

## StreamRuleEngine (TTSR)

- Monitors accumulated `chat_stream` output against compiled regex patterns
- On match: aborts current request, injects rule's `injection` as system message, retries
- Prevents double-injection (tracks already-fired rules in `HashSet`)
- Rules loaded from `~/.serana/stream-rules.toml` or workspace `.serana/stream-rules.toml`

## Message Validation

- `validate_message_alternation`: Enforces `system → user → assistant → (tool → assistant) → user → ...`
- `fix_message_alternation`: Merges consecutive same-role messages, injects placeholders for missing roles

## Design Decisions

- **Sequential tool execution**: `execute_tools_concurrent` iterates sequentially. True concurrency (tokio::join_all) is safe but not yet enabled.
- **Sysbox agent lives in-memory**: `AgentCallbacks` is pass-by-reference, not cloned per iteration.
- **Compression triggers at 50%/85%**: Preflight at 50% context usage, gateway at 85%. Gateway blocks next LLM call until compression completes.
- **Session store is secondary storage**: Messages flow through agent first, persisted after response.
