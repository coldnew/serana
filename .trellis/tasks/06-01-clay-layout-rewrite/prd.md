# brainstorm: clay-like layout rewrite

## Goal

Replace the current display layout stack with a Clay-inspired layout system so `display-protocol`, `display-tui`, and `display-wgpu` share one renderer-agnostic core that is easier to maintain and closer to Emacs-style UI composition.

## What I already know

* The user wants a rethink of the layout system based on `ref/clay`.
* The target crates are `display-protocol*`, `display-tui`, and `display-wgpu`.
* Clay uses immediate nested declarative layout, flex-like sizing, text measurement, scroll containers, and renderer-agnostic render commands.
* The repo currently has `display-protocol` as the shared UI model, `display-protocol-jsx` as a JSX DSL, `display-tui` for terminal rendering, and `display-wgpu` for GPU rendering.
* Mora UI already depends on the existing display stack.

## Assumptions (temporary)

* We should keep Mora usable during the migration instead of doing a hard break.
* The new layout core should probably preserve the current renderer split, even if the internal API changes.
* We only need to match Clay's layout model and command pipeline, not reimplement every Clay feature verbatim.

## Open Questions

* None. Preserve the current public API shape with compatibility shims first, then drop the old path after the new path is complete.

## Requirements (evolving)

* Shared layout engine must be renderer-agnostic.
* TUI and WGPU renderers must consume the same layout output.
* JSX-based UI construction should still be possible, or be replaced by an equivalent declarative builder.
* Basic scrolling, text measurement, wrapping, and box/row/column composition must work.
* The new path must coexist with the old path until parity is proven.
* The result must be verifiable in both terminal and GUI paths.

## Acceptance Criteria (evolving)

* [ ] New Clay-like layout core exists alongside the old path.
* [ ] `display-tui` and `display-wgpu` can render from the new layout output.
* [ ] Mora still starts in both TUI and GUI modes while the migration is in progress.
* [ ] At least one live screenshot / visual verification is taken for both paths after the rewrite.
* [ ] Existing editor text rendering remains usable.
* [ ] Old layout path is removed only after the new path reaches parity and tests pass.

## Definition of Done (team quality bar)

* Tests added/updated where behavior changed.
* `cargo fmt` and targeted checks/tests pass.
* Docs/spec notes updated if behavior or API changes.
* Migration risks and compatibility impact documented.

## Out of Scope (explicit)

* Rebuilding every Mora UI feature.
* Matching Clay exactly down to all internal details unless required by the repo.
* A full Emacs clone as part of this task.

## Technical Notes

* Reference repo: `ref/clay`.
* Current packages to inspect first: `crates/display-protocol`, `crates/display-protocol-jsx`, `crates/display-tui`, `crates/display-wgpu`.
* Existing UI construction already flows through JSX helpers and `UiNode` rendering.
