# brainstorm: integrate serana crates

## Goal

Refactor Serana so the coding-agent implementation lives under the `serana/`
crate as small, understandable modules instead of being split across
`crates/serana-*`. The result should make Serana easier to evolve as the main
coding agent while keeping build behavior and existing CLI/TUI behavior intact.

## What I already know

* User wants `crates/serana*` integrated back into `serana`.
* User wants features split into small modules inside Serana.
* Existing Serana-related workspace members:
  * `serana/`
  * `crates/serana-core/`
  * `crates/serana-llm/`
  * `crates/serana-tree-sitter/`
* `serana` already owns most app-level implementation:
  * `serana/src/agent/*`
  * `serana/src/tools/*`
  * `serana/src/tui/*`
  * `serana/src/lsp/*`
* `crates/serana-core` contains shared agent traits and data types:
  * `Agent`, `AgentOutput`, `Tool`, `LlmClient`, `Message`, `Config`,
    callbacks, approval, budget, compression, token counting, verification.
* `crates/serana-llm` contains provider clients and routing:
  * OpenAI, OpenRouter, Anthropic, fallback, registry, credentials, streaming,
    auxiliary summarization.
* `crates/serana-tree-sitter` contains AST parsing, summaries, and syntax
  highlighting.
* `crates/mora-core` imports `serana_core::{Agent, Result, Tool}`.
* `mora` imports `serana_tree_sitter` from `mora/src/mora/syntax.rs`.
* `serana` depends on `mora-bin` for Mora mode in `serana/src/main.rs`.
* Because `serana -> mora-bin` already exists, making `mora-bin` depend on
  `serana` for tree-sitter would create a Cargo dependency cycle.

## Assumptions (temporary)

* "small model" means "small modules": split code into local modules with clear
  ownership rather than keeping small crates.
* The preferred direction is to remove workspace crates named `serana-*`, not
  necessarily to remove every shared non-Serana utility crate.
* Serana's Mora mode should keep working unless the user explicitly says it can
  be removed or redesigned.
* Existing dirty worktree changes are unrelated and must not be reverted.

## Open Questions

* None blocking. User confirmed the recommended MVP on 2026-06-03.

## Requirements (evolving)

* Move `serana-core` code into `serana/src/core/`.
* Move `serana-llm` code into `serana/src/llm/`.
* Replace internal imports from `serana_core` / `serana_llm` with local Serana
  modules.
* Remove `crates/serana-core` and `crates/serana-llm` from the workspace after
  imports are migrated.
* Update docs so a maintainer can understand the new Serana architecture.
* Avoid unrelated refactors while migrating crate boundaries.

## Candidate Options

### Option A: Keep Tree-Sitter Shared, Rename It

Move `crates/serana-tree-sitter` to a neutral shared crate name such as
`crates/code-tree-sitter` or `crates/code-intel-tree-sitter`. Serana and Mora
both depend on the neutral crate. This removes `crates/serana*` while avoiding
the existing `serana -> mora-bin` cycle.

Trade-off: there is still one shared crate, but it is no longer a Serana crate.

### Option B: Move Tree-Sitter Into Serana And Remove Mora Mode Coupling

Move tree-sitter into `serana/src/tree_sitter/`, then remove or redesign
Serana's direct dependency on `mora-bin` so Mora can depend on Serana without a
cycle.

Trade-off: larger behavioral/API change; touches Serana CLI Mora mode and Mora
editor integration.

### Option C: Duplicate Tree-Sitter Logic

Move tree-sitter into Serana and copy the highlighting subset into Mora.

Trade-off: minimal dependency complexity, but duplicated parser/highlighting
logic is a maintenance regression.

## Recommended MVP

Use Option A.

* Integrate `serana-core` and `serana-llm` into `serana`.
* Rename the tree-sitter crate to a neutral shared crate because Mora already
  uses it and Serana already depends on Mora.
* Update docs to explain that Serana owns agent/runtime/LLM/core concepts, while
  code parsing/highlighting is shared editor infrastructure.

## Confirmed Scope

* Integrate `serana-core` and `serana-llm` into the `serana` crate.
* Rename `crates/serana-tree-sitter` to a neutral shared crate.
* Update Serana docs so the new architecture is understandable.
* Keep Mora mode and Mora syntax highlighting working.

## Acceptance Criteria (evolving)

* [x] No workspace members named `crates/serana-core`,
  `crates/serana-llm`, or `crates/serana-tree-sitter`.
* [x] `cargo check -p serana` passes.
* [x] `cargo test -p serana --lib` passes.
* [x] Mora builds or checks with the renamed shared tree-sitter crate.
* [x] Docs describe the new Serana module layout and dependency graph.
* [x] Existing unrelated dirty changes remain untouched.

## Definition of Done (team quality bar)

* Tests added/updated if behavior changes.
* Build/check/test commands run and recorded.
* Docs updated for architecture and maintainer understanding.
* Rollback considered for crate moves.

## Out of Scope (explicit)

* Rewriting agent behavior or prompts.
* Changing LLM provider semantics.
* Removing Serana TUI behavior.
* Large cleanup of unrelated Mora/display changes in the current dirty tree.

## Technical Notes

* Inspected `Cargo.toml`, `serana/Cargo.toml`, `mora/Cargo.toml`,
  `crates/serana-core/Cargo.toml`, `crates/serana-llm/Cargo.toml`,
  `crates/serana-tree-sitter/Cargo.toml`.
* Inspected public module exports in the three Serana library crates.
* Reverse dependency search found `mora` and `crates/mora-core` currently import
  Serana library crates.
* Existing docs are stale in places and describe planned crates such as
  `serana-agent` and `serana-tools`; documentation should be rewritten around
  the actual `serana/src/*` layout.

## Implementation Results

* Moved `crates/serana-core/src/*` to `serana/src/core/`.
* Moved `crates/serana-llm/src/*` to `serana/src/llm/`.
* Renamed `crates/serana-tree-sitter` to `crates/code-tree-sitter`.
* Updated Serana, Mora, and Mora-core imports/manifests for the new module and
  crate names.
* Updated `docs/architecture.md`, `docs/serana-refactor-tutorial.md`,
  `SPEC.md`, `README.md`, `.trellis/config.yaml`, and the Serana backend
  directory-structure spec.
* Verification:
  * `rustfmt --edition 2021 ...` on affected Rust files: passed.
  * `cargo check -p serana`: passed with pre-existing warnings.
  * `cargo check -p mora-core`: passed with pre-existing warnings.
  * `cargo test -p serana --lib`: passed with pre-existing warnings.
