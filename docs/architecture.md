# Serana Architecture

## Overview

Serana is a personal coding agent built in Rust. It uses a workspace of 7 crates providing LLM integration, code intelligence (LSP + tree-sitter), tool execution, agent orchestration, and a TUI interface.

## Crate Dependency Graph

```
serana (binary root)
  ├── serana-core          (foundational traits & types)
  ├── serana-llm           (LLM client implementations)
  ├── serana-lsp           (LSP protocol client)
  ├── serana-tree-sitter   (AST parser)
  ├── serana-tools         (tool implementations)
  ├── serana-agent         (agent orchestration)
  └── serana-tui           (terminal UI)
```

Dependency flow (only one direction):

```
serana-core
  ├── serana-llm
  ├── serana-lsp
  ├── serana-tree-sitter
  ├── serana-tools (also depends on serana-lsp, serana-tree-sitter)
  ├── serana-agent (depends on serana-llm, serana-tools)
  └── serana-tui   (depends on serana-llm, serana-agent, serana-tools)
```

No circular dependencies. Lower-level crates never depend on higher-level ones.

## High-Level Data Flow

```
User Input (TUI or CLI)
  │
  ▼
┌──────────────────────────────────────────────────────────┐
│                    serana-agent                           │
│  CodingAgent::execute(instruction)                       │
│                                                          │
│  1. Build system prompt (prompt_builder)                 │
│  2. Call LLM (serana-llm → LlmClient)                    │
│  3. Parse tool calls from response                       │
│  4. Execute tools (serana-tools → ToolRegistry)          │
│  5. Feed results back to LLM                             │
│  6. Repeat until done or budget exhausted                │
│                                                          │
│  Optional: compress context if nearing token limit       │
│  Optional: persist session (SessionStore → SQLite)       │
└──────────────────────────────────────────────────────────┘
  │
  ├── serana-core  (Message, Tool, LlmClient traits)
  ├── serana-llm   (OpenAI, Anthropic, fallback, routing)
  ├── serana-tools (24 tools: fs, shell, search, git, LSP, ...)
  └── serana-lsp   (LSP server management)
  └── serana-tree-sitter (AST parsing)
```

## Crate Responsibilities

| Crate | Responsibility | Key Exports |
|-------|---------------|-------------|
| `serana-core` | Shared types, traits (`Agent`, `Tool`, `LlmClient`), config, callbacks, cancellation, budgets, compression, meta-cognition, verification | `Agent`, `Tool`, `LlmClient`, `Config`, `Message`, `AgentCallbacks`, `IterationBudget`, `MetaCognition` |
| `serana-llm` | `LlmClient` implementations: OpenAI, Anthropic; credential management; fallback chains; role-based routing; auxiliary tasks | `OpenAiClient`, `AnthropicClient`, `FallbackChain`, `RoutingClient`, `AuxiliaryClient` |
| `serana-lsp` | LSP client protocol: spawn language servers, JSON-RPC 2.0, go-to-def, references, hover | `LspManager`, `LspClient`, `Position`, `Location` |
| `serana-tree-sitter` | AST parsing and syntax queries for Rust, JS/TS, Python, Go | `ParserManager`, `FunctionDef`, `StructDef`, `Import` |
| `serana-tools` | 24 `Tool` implementations: file ops, shell, search, git, GitHub, SSH, eval, browser, debugger, memory, MCP, skills, etc. | `ToolRegistry`, `ShellSession`, `DapSession`, `MemoryStore`, `McpConnection` |
| `serana-agent` | Agent orchestration: execute loop, context compression, session persistence, subagent spawning, stream rules, message validation | `CodingAgent`, `SessionStore`, `SubagentSpawner`, `ContextCompressor`, `StreamRuleEngine` |
| `serana-tui` | Terminal UI: ratatui rendering, event loop, markdown, syntax highlighting, editor, dialogs | `run()`, `App`, `Tui` |

## Core Execution Loop (CodingAgent)

```
┌─────────────────────┐
│   Build system      │
│   prompt            │
└────────┬────────────┘
         ▼
┌─────────────────────┐
│   LLM call          │
│   chat_with_tools   │
└────────┬────────────┘
         ▼
   ┌───────────┐
   │ Tool call │←── No ───┐
   └─────┬─────┘          │
         │ Yes             │
         ▼                 │
┌─────────────────────┐   │
│   Execute tools     │   │
│   (sequential)      │   │
└────────┬────────────┘   │
         ▼                │
┌─────────────────────┐   │
│   Inject results    │   │
│   as messages       │   │
└────────┬────────────┘   │
         ▼                │
┌─────────────────────┐   │
│   Check budget      │───┘
│   Check compression │
└────────┬────────────┘
         ▼ (no more tool calls)
┌─────────────────────┐
│   Return            │
│   AgentOutput       │
└─────────────────────┘
```

## LLM Provider Architecture

```
                        ┌──────────────────┐
                        │  RoutingClient    │  (serana-llm::registry)
                        │  role→chain map  │
                        └────────┬─────────┘
                                 │
                    ┌────────────┴────────────┐
                    │    FallbackChain         │
                    │  provider1 → provider2   │
                    └────────────┬─────────────┘
                                 │
               ┌─────────────────┼─────────────────┐
               ▼                 ▼                   ▼
        ┌────────────┐   ┌──────────────┐   ┌──────────────┐
        │OpenAIClient│   │AnthropicClient│   │Refreshable...│
        └────────────┘   └──────────────┘   └──────────────┘
```

## LSP + Tree-Sitter Code Intelligence

```
Tool Call (e.g., "find references of X in main.rs")
  │
  ├── LspDefinitionTool
  │     └── serana-lsp::LspManager
  │           ├── LspClient (rust-analyzer)
  │           └── LspTransport (stdio, JSON-RPC 2.0)
  │
  ├── AstOutlineTool
  │     └── serana-tree-sitter::ParserManager
  │           └── SyntaxTree (language-specific queries)
  │
  └── SearchTool (ripgrep fallback)
```

## Self-Evolution

Serana can modify her own source code through `self_evolve` tools:

```
ReadSelfTool  → read source files
EditSelfTool  → edit source files (hashline format)
CargoTool     → `cargo build`, `cargo test`
GitTool       → `git add`, `git commit`, etc.
SearchCodeTool → ripgrep across own codebase
RecordModificationTool → log to MetaCognition
ModificationStatsTool  → query modification history
ReflectModificationTool → extract lessons from modifications
VerificationSystem (serana-core) → build + test before committing
```

All modifications are tracked in `MetaCognition` with timestamps, pass/fail status, and extracted lessons for continuous improvement.
