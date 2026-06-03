# Directory Structure

> How Serana backend code is organized.

---

## Overview

Serana is one coding-agent product. Serana-owned behavior belongs in the
`serana` crate as small modules, not in separate `crates/serana-*` workspace
crates.

Use a separate workspace crate only for neutral infrastructure with multiple
real consumers. Current example: `crates/code-tree-sitter` is shared by Serana
AST tools and Mora syntax highlighting.

## Directory Layout

```
serana/src/
├── main.rs      # CLI entrypoint
├── lib.rs       # public module root
├── core/        # Agent, Tool, LlmClient, Message, Config, callbacks, budgets
├── llm/         # provider clients, routing, fallback, streaming, credentials
├── agent/       # HermesAgent runtime, turns, sessions, compaction, subagents
├── tools/       # Tool implementations and ToolRegistry
├── lsp/         # LSP client, transport, protocol types
└── tui/         # terminal app, events, rendering, dialogs

crates/code-tree-sitter/
└── src/         # shared ParserManager, LanguageId, highlighting, summaries
```

## Module Organization

Add new Serana features by ownership:

| Feature | Location |
| --- | --- |
| Shared agent/tool/LLM contract | `serana/src/core/` |
| New LLM provider or provider routing | `serana/src/llm/` |
| Agent loop, prompts, compaction, sessions | `serana/src/agent/` |
| New tool | `serana/src/tools/` |
| LSP protocol/runtime behavior | `serana/src/lsp/` |
| Terminal UI behavior | `serana/src/tui/` |
| Parser/highlighter shared with Mora | `crates/code-tree-sitter/` |

## Naming Conventions

* Use module names that describe the feature boundary: `core`, `llm`, `agent`,
  `tools`, `lsp`, `tui`.
* Do not create packages named `serana-core`, `serana-llm`,
  `serana-tree-sitter`, or other `crates/serana-*`.
* Inside `serana/src`, import shared contracts with `crate::core`.
* Inside `serana/src`, import provider helpers with `crate::llm`.
* Outside Serana, import shared contracts with `serana::core`.
* Import parser/highlighter support with `code_tree_sitter`.

## Examples

```rust
// Serana library module
use crate::core::{Result, Tool};
use crate::llm::AuxiliaryClient;
```

```rust
// Serana binary entrypoint
use serana::core::{Agent, Config, LlmClient};
use serana::llm::OpenAiClient;
```

```rust
// Mora or other workspace crate
use code_tree_sitter::ParserManager;
use serana::core::{Agent, Result, Tool};
```

## Wrong vs Correct

Wrong:

```text
crates/serana-new-feature/
crates/serana-llm-extra/
```

Correct:

```text
serana/src/<owning-module>/new_feature.rs
```

If the feature is shared infrastructure, choose a neutral crate name that does
not imply Serana ownership, then update `.trellis/config.yaml`, `Cargo.toml`,
and docs in the same change.
