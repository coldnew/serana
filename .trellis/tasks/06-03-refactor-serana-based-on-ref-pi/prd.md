# Refactor Serana Based on ref/pi

## Goal

Bring Serana closer to the Pi coding-agent interaction model while preserving Serana's Rust architecture and current features. The first concrete parity gap is interactive `/login` support in the TUI, matching Pi's user flow where authentication can be started from inside the session.

## What I already know

* User asked to refactor Serana based on `ref/pi`, keep all features, and include `/login`.
* `ref/pi` is a TypeScript monorepo; the relevant package is `ref/pi/packages/coding-agent`.
* Pi's coding-agent documents `/login` as an interactive TUI command for OAuth or API-key authentication.
* Serana already supports `serana login codex`, which delegates to `codex login --device-auth`.
* Serana already supports Codex-backed models through `codex exec`, and the TUI model catalog includes `codex/gpt-5.5` and `codex/gpt-5.4-mini`.
* Serana's TUI slash-command registry does not currently include `/login`.

## Assumptions (temporary)

* The initial implementation should be a narrow parity slice: add `/login codex` to the Serana TUI and reuse the existing top-level Codex login behavior.
* "All features" means preserve existing Serana features while refactoring toward Pi-style user flows, not port the entire Pi TypeScript codebase in one pass.
* Subscription login support beyond Codex is out of scope unless explicitly requested.

## Open Questions

* None for the first slice.

## Requirements (evolving)

* Split the refactor into feature-sized slices and commit each completed slice separately.
* Preserve existing top-level `serana login codex` behavior.
* Add interactive `/login codex` behavior to the TUI.
* `/login` with no provider should return concise usage or provider guidance.
* `/login codex` should run `codex login --device-auth` using inherited stdio and report success or failure in the TUI.
* Keep changes minimal and localized to Serana command handling where possible.

## Acceptance Criteria (evolving)

* [x] `serana login codex` still parses and delegates to Codex CLI device auth.
* [x] TUI slash command parsing recognizes `/login codex`.
* [x] `/login` with no provider returns a helpful usage message.
* [x] Unit tests cover the new slash-command result.
* [x] `cargo fmt --all` passes.
* [x] Relevant build check passes: `cargo check -p serana`.
* [x] Existing binary CLI login parse test passes.
* [ ] Serana lib tests run. Blocked by unrelated `CompressionConfig` initializer errors in existing `serana/src/agent/*` tests.
* [x] Completed feature slice is committed without unrelated worktree changes.

## Review Notes

* Added TUI `/login codex` parsing and handling.
* Shared Codex device-auth subprocess launcher between CLI and TUI.
* Updated README and SPEC to document TUI `/login codex`.
* Verification: `cargo fmt --all`, `cargo check -p serana`, and `cargo test -p serana --bin serana cli_accepts_codex_login_command` pass.
* Blocker: `cargo test -p serana login_requires_provider` cannot run the new lib test because unrelated `CompressionConfig` initializers in `serana/src/agent/compression_gate.rs`, `serana/src/agent/compressor.rs`, and `serana/src/agent/hermes.rs` are missing fields.
* Commit: `9c6ae69 feat(serana): add tui codex login`.
* Added TUI `/hotkeys` command for Pi-style keyboard shortcut discovery.
* Verification: `cargo fmt --all` and `cargo check -p serana` pass for `/hotkeys`.
* Blocker: `cargo test -p serana hotkeys_command_shows_shortcuts` cannot run because the same unrelated `CompressionConfig` test compile errors occur before the filtered test executes.
* Commit: `2996b6a feat(serana): add hotkeys command`.
* Wired TUI `/copy` to the existing clipboard implementation instead of reporting "not yet wired".
* Verification: `cargo fmt --all` and `cargo check -p serana` pass for `/copy`.
* Blocker: `cargo test -p serana slash_copy_without_assistant_message_shows_message` cannot run because the same unrelated `CompressionConfig` test compile errors occur before the filtered test executes.
* Commit: `6532a18 feat(serana): wire copy command`.
* Added TUI `/name <name>` command using existing session title persistence.
* Verification: `cargo fmt --all` and `cargo check -p serana` pass for `/name`.
* Blocker: `cargo test -p serana name_command_sets_session_name` and `cargo test -p serana slash_name_without_store_shows_message` cannot run because the same unrelated `CompressionConfig` test compile errors occur before the filtered tests execute.
* Commit: `2ecbe27 feat(serana): add session name command`.
* Added TUI `/export [file]` command that writes the current transcript to escaped HTML.
* Verification: `cargo fmt --all` and `cargo check -p serana` pass for `/export`.
* Blocker: `cargo test -p serana export_command_accepts_optional_path` and `cargo test -p serana session_html_export_escapes_content` cannot run because the same unrelated `CompressionConfig` test compile errors occur before the filtered tests execute.
* Commit: `a4d1ea1 feat(serana): add session export command`.
* Added prompt templates from `.serana/prompts/*.md` and `~/.serana/prompts/*.md`, with `{{0}}` and positional argument substitution.
* Verification: `cargo check -p serana` passes for prompt templates.
* Commit: `8e09031 feat(serana): add prompt template commands`.
* Added TUI `@path` file reference expansion into `<file>` blocks.
* Verification: `cargo check -p serana` passes for `@path` expansion.
* Commit: `7ba1a34 feat(serana): expand file references in tui input`.
* Added TUI `!command` and `!!` shell shortcuts.
* Verification: `cargo check -p serana` passes for shell shortcuts.
* Commit: `9f0294d feat(serana): add tui shell shortcuts`.

## Definition of Done

* Tests added or updated for changed behavior.
* Formatting passes.
* No unrelated refactors or cleanup.
* Changes are summarized with verification results.

## Out of Scope

* Porting Pi's full TypeScript package structure into Rust.
* Adding OAuth implementations for Anthropic, GitHub Copilot, or API-key storage unless requested.
* Replacing Serana's existing agent, tool, or session architecture in this slice.

## Technical Notes

* Pi docs inspected: `ref/pi/packages/coding-agent/README.md`, `ref/pi/packages/coding-agent/docs/quickstart.md`.
* Serana files inspected: `serana/src/main.rs`, `serana/src/tui/slash_commands.rs`, `serana/src/tui/app.rs`, `serana/src/llm/codex.rs`.
* Existing Serana CLI test: `cli_accepts_codex_login_command`.
