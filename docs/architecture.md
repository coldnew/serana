# Serana Architecture

## Overview

Serana is the coding agent crate. The agent runtime, shared contracts, LLM
providers, tools, LSP client, and TUI live inside `serana/` as modules so the
system can evolve as one product instead of being split across small
Serana-specific crates.

One parsing crate remains outside Serana:

* `crates/code-tree-sitter` is shared editor/code-intelligence infrastructure.
  Serana uses it for AST tools. Mora uses it for syntax highlighting. Keeping it
  neutral avoids a Cargo cycle because Serana also depends on `mora-bin` for
  Mora mode.

## Workspace Shape

```
serana/
  src/
    main.rs          CLI entrypoint
    lib.rs           Public Serana library modules
    core/            Agent, Tool, LlmClient, Message, Config, callbacks,
                     approval, budgets, compression, verification
    llm/             OpenAI, OpenRouter, Anthropic, routing, fallback,
                     credentials, streaming, auxiliary tasks
    agent/           HermesAgent runtime, turn loop, prompts, sessions,
                     compaction, subagents, checkpoints
    tools/           Agent tools: fs, shell, git, search, AST, LSP, MCP,
                     memory, browser, review, skills, etc.
    lsp/             LSP client, transport, and protocol types
    tui/             Terminal app, rendering, events, dialogs, markdown

crates/code-tree-sitter/
  src/
    lib.rs           ParserManager, LanguageId, SyntaxTree, symbol queries
    highlight.rs     Tree-sitter highlighting used by Mora
    summary.rs       Source summary helpers
```

## Dependency Direction

```
serana
  ├── code-tree-sitter
  ├── display-protocol / display-tui
  └── mora-bin        (Mora mode only)

mora-bin
  └── code-tree-sitter

mora-core
  └── serana::core    (Agent, Tool, Result contracts)
```

Rules:

* Serana-owned agent behavior stays in `serana/src/*`.
* Shared parser/highlighting support stays in `crates/code-tree-sitter`.
* Do not recreate `crates/serana-*`; add small Serana modules instead.
* If Mora and Serana both need the same parser/highlighter logic, put it in
  `code-tree-sitter`, not in Serana.

## Main Module Responsibilities

| Module | Responsibility | Key Exports |
| --- | --- | --- |
| `serana::core` | Shared contracts and low-level agent data types | `Agent`, `Tool`, `LlmClient`, `Config`, `Message`, `AgentCallbacks`, `CancelToken`, `IterationBudget`, `CompressionConfig` |
| `serana::llm` | LLM provider clients and routing | `OpenAiClient`, `OpenRouterClient`, `AnthropicClient`, `FallbackChain`, `RoutingClient`, `AuxiliaryClient` |
| `serana::agent` | The coding-agent runtime | `HermesAgent`, `AgentFactory`, `AgentRuntimeConfig`, `SessionStore`, `ContextCompressor` |
| `serana::tools` | Tool implementations and registry | `ToolRegistry`, `ToolProfile`, individual tool structs |
| `serana::lsp` | Language-server integration | `LspManager`, `LspClient`, `Position`, `Location` |
| `serana::tui` | Terminal interface | `run`, `App`, render/event/dialog helpers |
| `code_tree_sitter` | Shared AST parsing and syntax highlighting | `ParserManager`, `LanguageId`, `SyntaxTree`, `FunctionDef`, `HighlightToken` |

## Core Execution Flow

```
User input (CLI or TUI)
  │
  ▼
AgentFactory builds HermesAgent
  │
  ├── Config and runtime options from serana::core
  ├── LLM client from serana::llm
  └── ToolRegistry from serana::tools
  │
  ▼
HermesAgent::execute
  │
  ├── Build prompt and context
  ├── Call LLM with tool definitions
  ├── Validate and execute tool calls
  ├── Append tool results as messages
  ├── Compact context when pressure is high
  └── Return AgentOutput
```

## LLM Provider Flow

```
AgentFactory
  │
  ▼
serana::llm::RoutingClient
  │
  ├── role -> provider chain
  ▼
serana::llm::FallbackChain
  │
  ├── OpenAiClient
  ├── OpenRouterClient
  └── AnthropicClient
```

`serana::core::LlmClient` is the boundary. Agent code depends on the trait;
provider-specific HTTP, streaming, credentials, and fallback logic stays in
`serana::llm`.

## Code Intelligence Flow

```
Tool call
  │
  ├── LSP tools
  │     └── serana::lsp::LspManager
  │
  ├── AST tools
  │     └── code_tree_sitter::ParserManager
  │
  └── Search tools
        └── ripgrep / native search
```

Tree-sitter is shared with Mora. LSP is Serana-specific because it is part of
the agent tool runtime.

## Why This Refactor Exists

The old layout had small crates named `serana-core`, `serana-llm`, and
`serana-tree-sitter`. That made the code look more modular, but it scattered
one product across crate boundaries and made Serana harder to reason about.

The new rule is simpler:

* Serana product logic lives in `serana/`.
* Shared infrastructure with more than one real consumer gets a neutral crate.
* Module boundaries stay small, but crate boundaries are reserved for genuine
  workspace sharing.
