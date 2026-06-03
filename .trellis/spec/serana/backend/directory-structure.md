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

## Scenario: Codex CLI LLM Provider

### 1. Scope / Trigger

Trigger: adding or changing a Serana LLM provider that delegates to the
installed Codex CLI and its ChatGPT login state.

### 2. Signatures

* TUI command: `/model codex/<model>`.
* Rust provider: `CodexClient::new(model).with_workspace(workspace)`.
* External command:
  `codex exec --model <model> --sandbox read-only --skip-git-repo-check --ephemeral --output-last-message <tempfile> --color never -`.

### 3. Contracts

* Provider-qualified model strings split once on `/`.
* `provider` becomes the prefix (`codex`); `llm.model` becomes the suffix.
* Prompt text is written to Codex stdin.
* Serana reads the final assistant response from `--output-last-message`, not
  from Codex's transcript output.

### 4. Validation & Error Matrix

* `codex` missing from `PATH` -> surface an installation/login hint.
* Codex exits non-zero -> surface exit status plus stderr.
* Last-message file missing or empty -> fall back to trimmed stdout/stderr.
* Unsupported Codex model -> let Codex's API error propagate to the user.

### 5. Good/Base/Bad Cases

* Good: `/model codex/gpt-5.5` sets `provider=codex`, `model=gpt-5.5`, and
  subsequent requests run through Codex CLI.
* Base: `/model gpt-4o-mini` keeps the current provider and changes only the
  model.
* Bad: `/model codex/unsupported` should not silently fall back to OpenAI.

### 6. Tests Required

* Unit test provider-qualified parsing.
* Unit test bare model parsing keeps current provider.
* Unit test `/model` completion includes supported Codex model entries.
* Unit test Codex exec arguments include model, read-only sandbox, ephemeral
  mode, and output-last-message capture.
* Smoke test live Codex CLI with a supported model when auth behavior changes.

### 7. Wrong vs Correct

Wrong:

```text
/model codex/gpt-5.5 only updates app.model display text
```

Correct:

```text
/model codex/gpt-5.5 updates app.provider and app.model, then the active LLM
client routes future calls through CodexClient.
```
