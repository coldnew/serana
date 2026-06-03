# Support Codex-backed models in Serana

## Goal

Make Serana's Codex login useful in the TUI by allowing Codex-backed model selection and ensuring `/model` completion suggests models that Serana can actually use.

## What I already know

* `serana login codex` delegates to `codex login --device-auth`.
* The Codex CLI stores auth under `~/.codex`, not `~/.serana`.
* Serana currently creates one `OpenAiClient` at TUI startup from `Config`.
* `/model` currently updates `app.model` only; it does not rebuild or switch the LLM client used by the agent.
* `/model` completion is a static local catalog and does not include Codex model names.
* The installed Codex CLI supports `codex exec -m <MODEL> <PROMPT>` for non-interactive use.

## Assumptions

* "Codex model" means models available through the logged-in Codex CLI, not API-key OpenAI models.
* The MVP can use the Codex CLI as the provider boundary instead of reading private Codex auth files.
* Tool use parity with Serana's native OpenAI client is out of scope for the first fix.

## Requirements

* Add a Codex-backed provider path that uses the installed `codex` CLI and its existing login state.
* Let `/model codex/<model>` switch Serana's active provider and model, not only the status text.
* Include Codex model suggestions in `/model` argument autocomplete.
* Keep existing OpenAI/OpenRouter-style model names working.
* Surface clear errors when Codex CLI is missing or not logged in.

## Acceptance Criteria

* [x] `/model codex/<model>` updates provider to `codex` and model to `<model>`.
* [x] Subsequent user messages are executed through the Codex provider.
* [x] `/model` autocomplete includes Codex model entries.
* [x] Tests cover parsing/switching and autocomplete catalog behavior.
* [ ] Existing Serana tests still pass for the touched package.

## Definition of Done

* Rust tests added or updated for the changed behavior.
* `cargo fmt --all` run.
* Relevant `cargo test` target run.
* Behavior and limitations documented if user-facing behavior changes.

## Out of Scope

* Reading or depending on Codex's private `~/.codex/auth.json` format.
* Full streaming/tool-call parity for Codex CLI-backed responses.
* Dynamic model discovery from OpenAI/Codex services.

## Technical Notes

* Likely files: `serana/src/tui/app.rs`, `serana/src/tui/mod.rs`, `serana/src/tui/slash_commands.rs`, `serana/src/llm/*`.
* `codex exec --help` confirms `-m, --model <MODEL>` and non-interactive prompt support.

## Review

* Added `CodexClient` using `codex exec --model <model> --sandbox read-only --ephemeral --output-last-message <tempfile>`.
* Added provider-qualified model parsing so `/model codex/gpt-5.5` changes both `provider` and `model`.
* Added active LLM client routing so the TUI agent uses the current provider/model instead of the startup-only client.
* Added Codex model suggestions to `/model` autocomplete and the model selector catalog.
* Verification: `cargo fmt --all`, `cargo check -p serana`, `cargo test -p serana --bin serana`, and a live `codex exec --model gpt-5.5` smoke test passed.
* Blocked verification: `cargo test -p serana --lib` fails on unrelated existing `CompressionConfig` test initializers missing fields in `serana/src/agent/compression_gate.rs`, `serana/src/agent/compressor.rs`, and `serana/src/agent/hermes.rs`.
