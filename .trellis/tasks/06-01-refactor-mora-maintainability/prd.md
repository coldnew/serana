# Refactor Mora for maintainability

## Goal

Make Mora easier to maintain by reducing coupling in the editor implementation while preserving current behavior. Mora is an Emacs-like editor with shared GUI/TUI core behavior and a Clojure-syntax Lisp language used like Emacs Lisp.

## What I already know

* `mora` is the application crate with editor, Lisp, GUI/TUI integration, and CLI entry points.
* GUI and TUI both use `MoraCore` from `mora/src/mora/editor_core.rs`.
* `MoraCore` owns `MoraEditor`, converts display-protocol input to Mora key events, renders via `ui_node::build_ui`, and handles resize/quit.
* `mora/src/mora/editor.rs` is the biggest maintainability hotspot at about 5,000 lines.
* `MoraEditor` currently owns editor state plus key dispatch, prefix handling, action execution, command execution, window operations, visual/normal/emacs mode behavior, search state, snippet state, LSP state, Lisp bridge state, and shell state.
* Lisp extension APIs are already split under `mora/src/mora/lisp_ext/`.
* Major and minor modes are already split under `mora/src/mora/major_mode/` and `mora/src/mora/minor_mode/`.
* Existing uncommitted changes touch `mora/src/mora/editor.rs`, `mora/src/mora/mod.rs`, `mora/src/mora/mshell.rs`, `crates/display-wgpu/src/renderer.rs`, `crates/tasks/todo.md`, and `crates/tasks/lessons.md`; source edits must preserve those changes.

## Assumptions

* This refactor should be behavior-preserving.
* The first useful slice is to extract cohesive responsibilities from `MoraEditor`, not redesign the display protocol, Lisp VM, or GUI/TUI architecture.
* The implementation should prefer mechanical moves and thin helper modules over new abstractions unless an abstraction removes clear duplication or coupling.
* The refactor should keep public APIs stable unless a small API change clearly improves the internal boundary and all call sites are updated.

## Open Questions

* None for the first implementation pass.

## Requirements

* Preserve existing editor behavior for GUI and TUI paths.
* Keep GUI/TUI sharing through `MoraCore`.
* Reduce the size and responsibility concentration of `mora/src/mora/editor.rs`.
* Extract command execution, modal key handling, and cohesive editor state groups where those moves can remain mechanical.
* Keep changes surgical and easy to review.
* Add or update tests where the moved behavior can be covered without requiring an interactive terminal or GPU window.

## Acceptance Criteria

* [x] Command execution is separated from the main editor state definition.
* [x] Modal key handling is separated from the main editor state definition.
* [x] At least one cohesive state group is separated where doing so does not require redesigning editor behavior.
* [x] Existing editor behavior is preserved for the moved code.
* [x] Targeted formatting was run on the changed editor files.
* [ ] Relevant Rust tests pass, with `cargo test` preferred unless the scope requires a narrower command first.
* [x] PRD review section records what changed and what remains.

## Definition of Done

* Tests added or updated where appropriate.
* `cargo fmt` run.
* Rust tests run and results recorded.
* No unrelated source edits or cleanup.
* Rollback remains simple: the refactor should be reviewable as moves/extractions plus minimal call-site updates.

## Out of Scope

* Rewriting Mora in a different architecture.
* Replacing the Lisp VM/evaluator.
* Changing the display protocol.
* Redesigning GUI or TUI rendering.
* Adding new editor features as part of the refactor.

## Technical Notes

* Relevant package specs are currently placeholder indexes: `.trellis/spec/mora-bin/backend/index.md` and `.trellis/spec/mora-core/backend/index.md`.
* Main candidate files inspected:
  * `mora/src/mora/editor.rs`
  * `mora/src/mora/editor_core.rs`
  * `mora/src/mora/mod.rs`
  * `mora/src/bin/mora.rs`
  * `mora/src/mora/lisp_ext/*`
* Current file-size signal:
  * `mora/src/mora/editor.rs`: about 5,068 lines.
  * `mora/src/lisp/eval.rs`: about 2,256 lines.
  * `mora/src/mora/buffer.rs`: about 1,841 lines.
  * `mora/src/lisp/vm.rs`: about 1,117 lines.

## Review

### Changes

* Split `mora/src/mora/editor.rs` into child modules:
  * `mora/src/mora/editor/commands.rs` for action execution, command execution, command helpers, Lisp state sync, snippet/LSP/git helpers, macro processing, and ace jump helpers.
  * `mora/src/mora/editor/input.rs` for modal key handling, prefix handling, visual/emacs/normal/iedit handling, and iedit state operations.
  * `mora/src/mora/editor/windows.rs` for window splitting, deletion, balancing, and sync helpers.
* Kept `mora/src/mora/editor.rs` focused on the editor data structure, initialization, basic accessors, minibuffer helpers, tests, and shared small helper functions.
* Preserved the existing `MoraEditor` API shape and used child modules so moved methods can still access editor internals without redesigning behavior.
* Reverted accidental workspace-wide `cargo fmt` churn and then ran targeted `rustfmt` only on the changed editor files.

### Verification

* `cargo check -p mora-bin` passed after the refactor.
* `rustfmt mora/src/mora/editor.rs mora/src/mora/editor/commands.rs mora/src/mora/editor/input.rs mora/src/mora/editor/windows.rs` passed.
* `cargo test -p mora-bin` compiled and ran; 259 tests passed and 1 pre-existing org-mode unit test failed:
  * `mora::org::tests::test_toggle_todo`
  * The failure is unrelated to this refactor. The test expects one heading to cycle `TODO -> DONE -> none`, but also expects an initially `DONE` heading to become `TODO`, which conflicts with the same stateless toggle behavior.

### Follow-up

* Decide the intended org TODO cycle and fix either `toggle_todo` or `test_toggle_todo`.
* A later refactor can split `commands.rs` again into narrower command families once this first mechanical extraction lands.
