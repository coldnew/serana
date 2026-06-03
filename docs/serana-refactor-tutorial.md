# Serana Refactor Guide

This guide explains the current Serana layout after the Serana-specific helper
crates were integrated back into the main `serana` crate.

## Mental Model

Serana is one coding-agent product. Keep product logic in `serana/src` and split
it into small modules. Do not add new `crates/serana-*` crates for ordinary
Serana features.

Use a workspace crate only when the code is real shared infrastructure with
another consumer. Today that applies to `crates/code-tree-sitter`, which is used
by both Serana and Mora.

## Current Layout

| Path | Purpose |
| --- | --- |
| `serana/src/main.rs` | CLI entrypoint. Loads config and selects one-shot, TUI, config, or Mora mode. |
| `serana/src/lib.rs` | Public Serana module root. Exposes `agent`, `core`, `llm`, `lsp`, `tools`, and `tui`. |
| `serana/src/core/` | Shared contracts and low-level types: `Agent`, `Tool`, `LlmClient`, `Message`, `Config`, callbacks, cancellation, budgets, compression, approval, verification. |
| `serana/src/llm/` | Provider implementations and routing: OpenAI, OpenRouter, Anthropic, fallback chains, credentials, streaming, auxiliary tasks. |
| `serana/src/agent/` | Runtime orchestration: `HermesAgent`, prompt building, turn running, tool execution, compaction, sessions, subagents, checkpoints. |
| `serana/src/tools/` | Agent tools and `ToolRegistry`: file I/O, shell, git, search, AST tools, LSP tools, MCP, browser, review, skills, memory, etc. |
| `serana/src/lsp/` | LSP manager/client/transport and protocol types. |
| `serana/src/tui/` | Terminal application state, events, rendering, dialogs, markdown, syntax display. |
| `crates/code-tree-sitter/` | Neutral shared parser/highlighter crate used by Serana AST tools and Mora syntax highlighting. |

## Dependency Rules

* `serana::agent`, `serana::tools`, `serana::tui`, and `serana::lsp` import
  shared contracts through `crate::core`.
* `serana::agent` imports providers through `crate::llm`.
* Serana AST tools import parser support through `code_tree_sitter`.
* Mora syntax highlighting imports parser support through `code_tree_sitter`.
* `mora-core` imports agent/tool contracts through `serana::core`.
* Do not move `code-tree-sitter` into Serana unless Serana's direct dependency
  on `mora-bin` is also redesigned; otherwise the workspace can form a Cargo
  dependency cycle.

## Main Execution Flow

1. `serana/src/main.rs` parses CLI arguments and loads `serana::core::Config`.
2. `AgentFactory` builds a `HermesAgent` from:
   * an `LlmClient` implementation from `serana::llm`;
   * a `ToolRegistry` from `serana::tools`;
   * runtime options from `AgentRuntimeConfig`.
3. `HermesAgent::execute` builds context and prompts, calls the LLM, validates
   tool calls, executes tools, appends tool results, and repeats until the turn
   is complete or the iteration budget is exhausted.
4. Context compression runs through `serana::agent::compaction` and the
   contracts in `serana::core::compression`.
5. Sessions and checkpoints are handled by `serana::agent` modules.

## Where To Add Things

| New work | Put it here |
| --- | --- |
| New LLM provider | `serana/src/llm/<provider>.rs`, then export from `serana/src/llm/mod.rs`. |
| Provider routing/fallback change | `serana/src/llm/registry.rs` or `serana/src/llm/fallback.rs`. |
| New agent tool | `serana/src/tools/<tool>.rs`, then register it in `serana/src/tools/mod.rs`. |
| Agent loop behavior | `serana/src/agent/turn_runner.rs`, `tool_turn.rs`, `engine.rs`, or `hermes.rs` depending on the boundary. |
| Prompt/context behavior | `serana/src/agent/prompt_builder.rs` or `gatherer.rs`. |
| Compression/compaction behavior | `serana/src/agent/compressor.rs`, `compression_gate.rs`, or `agent/compaction/*`; shared structs live in `serana/src/core/compression.rs`. |
| Shared trait/data contract | `serana/src/core/`. |
| AST parsing/highlighting used by both Serana and Mora | `crates/code-tree-sitter/`. |
| TUI rendering/state | `serana/src/tui/`. |

## Import Examples

Inside Serana library modules:

```rust
use crate::core::{Result, Tool};
use crate::llm::AuxiliaryClient;
```

Inside `serana/src/main.rs`:

```rust
use serana::core::{Agent, Config, LlmClient};
use serana::llm::OpenAiClient;
```

Inside Mora or other crates:

```rust
use code_tree_sitter::ParserManager;
use serana::core::{Agent, Result, Tool};
```

## Verification Commands

Use focused checks while working:

```bash
cargo check -p serana
cargo check -p mora-core
cargo test -p serana --lib
```

Run `cargo fmt --all` before committing.

## Refactor Policy

The preferred shape is small modules inside Serana, not small Serana crates.
Keep edits surgical:

* move code only when the ownership boundary is clear;
* update all imports and manifests in the same change;
* update `docs/architecture.md` when dependency direction changes;
* leave unrelated Mora/display cleanup alone unless the refactor requires it.
