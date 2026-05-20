# Serana Architecture

## Overview

Serana is a personal coding agent built in Rust. It follows a modular architecture designed for extensibility and eventual migration to Hermes agent capabilities.

## Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                         CLI (main.rs)                        │
│  clap-based argument parsing, interactive REPL mode          │
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
│  - Pluggable    │  │  - write_file   │  │  - Compactor    │
│    trait        │  │  - (extensible) │  │                 │
└─────────────────┘  └─────────────────┘  └─────────────────┘
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
- Abstracts agent behavior for future Hermes migration
- Single implementation: `CodingAgent`
- Future: multiple agent strategies (planner, executor, reviewer)

### `Tool`
```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    async fn execute(&self, input: Value) -> Result<Value>;
}
```
- All file operations, shell commands, searches are tools
- JSON-based input/output for LLM compatibility
- Tools are registered in `ToolRegistry`

### `LlmClient`
```rust
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat(&self, messages: &[Message]) -> Result<String>;
}
```
- OpenAI-compatible implementation
- Supports function calling (tool use)
- Future: local models, streaming responses

## Data Flow

```
User Instruction
       │
       ▼
┌──────────────────┐
│ Context Gatherer │ ─── Find relevant files
└──────────────────┘
       │
       ▼
┌──────────────────┐
│   LLM Request    │ ─── System prompt + context + instruction
└──────────────────┘
       │
       ▼
┌──────────────────┐
│  Parse Response  │ ─── Extract tool calls or final answer
└──────────────────┘
       │
       ▼
┌──────────────────┐
│  Execute Tools   │ ─── Run tools, collect results
└──────────────────┘
       │
       ▼
┌──────────────────┐
│  Iterate/Respond │ ─── Loop back to LLM with tool results
└──────────────────┘
       │
       ▼
   Final Output
```

## Module Responsibilities

| Module | Purpose |
|--------|---------|
| `config` | Configuration structs, default values, environment variables |
| `agent` | Agent trait, planning logic, task execution |
| `tools` | Tool trait, file/shell/search implementations |
| `llm` | LLM client trait, OpenAI implementation, prompt templates |
| `context` | File gathering, context compaction for long sessions |

## Extension Points

### Adding a New Tool
```rust
pub struct MyTool;

#[async_trait]
impl Tool for MyTool {
    fn name(&self) -> &'static str { "my_tool" }
    fn description(&self) -> &'static str { "Does something useful" }
    async fn execute(&self, input: Value) -> Result<Value> {
        // Implementation
    }
}

// Register in main.rs or agent setup
registry.register(Box::new(MyTool));
```

### Adding a New LLM Provider
```rust
pub struct MyLlmClient { /* ... */ }

#[async_trait]
impl LlmClient for MyLlmClient {
    async fn chat(&self, messages: &[Message]) -> Result<String> {
        // Call your LLM API
    }
}
```

### Adding a New Agent Strategy (Hermes Migration)
```rust
pub struct HermesAgent { /* ... */ }

#[async_trait]
impl Agent for HermesAgent {
    fn name(&self) -> &'static str { "hermes" }
    async fn execute(&self, instruction: &str) -> Result<AgentOutput> {
        // Different planning/execution strategy
    }
}
```

## Future Work

1. **Tool execution loop** - Currently stubbed; needs full implementation with structured output parsing
2. **Context compaction** - Summarize long conversations to stay within token limits
3. **RAG integration** - Index codebase for semantic search
4. **Streaming responses** - Real-time output from LLM
5. **Hermes capabilities** - Multi-agent collaboration, self-reflection, reasoning traces
