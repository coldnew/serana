# Mora Architecture

## Goal

Mora should be an Emacs-like editor with a Rust kernel and a Mora Lisp
extension surface. Rust owns the fast, unsafe, platform-facing, and
consistency-critical parts. Mora Lisp owns editor behavior, user workflows, and
customization policy.

The project should avoid two failure modes:

- A Rust-only editor with a Lisp scripting add-on. This is fast, but not
  Emacs-like because most behavior cannot be replaced by users.
- A Lisp-only editor core. This is flexible, but it puts text storage, display,
  process I/O, and rendering hot paths in the wrong layer.

The target design is:

```text
Rust kernel     = primitives, state, persistence, display, backends, VM
Mora Lisp       = commands, modes, keymaps, hooks, UI policy, packages
Display protocol = backend-neutral retained UI tree + render commands + inspector
```

## Architectural Rule

If a feature defines editor behavior or user workflow, implement it in Mora Lisp
unless it needs Rust for performance, safety, OS access, or backend access.

Rust APIs should expose small primitives. Mora Lisp should compose those
primitives into user-facing commands.

For example, `find-file` should be Lisp behavior backed by Rust primitives:

```lisp
(defcommand find-file [path]
  (interactive "FFind file: ")
  (switch-to-buffer (find-file-noselect path)))
```

The primitive `find-file-noselect` may be Rust because it touches decoding,
file I/O, path handling, buffer allocation, and file metadata. The command
prompt, buffer switching policy, hooks, recent-file integration, and error
presentation belong in Mora Lisp.

## Layer Responsibilities

### Rust Kernel

Rust owns:

- Buffer storage, cursor primitives, undo data structures, markers, overlays,
  registers, kill ring storage, and window tree state.
- File I/O, process I/O, shell execution, TRAMP transports, LSP transports,
  timers, async runtime integration, and OS events.
- Syntax engines such as tree-sitter and fast search primitives.
- Display protocol, layout, render commands, inspector metadata, TUI and WGPU
  backends, glyph caches, and input normalization.
- Mora Lisp reader, evaluator, bytecode/VM, namespaces, native primitive
  registration, and package loading.
- Minimal boot code that creates the first frame, first buffer, evaluator, and
  standard library load path.

Rust should not own:

- The default meaning of most `M-x` commands.
- Major/minor mode behavior unless the mode is only a thin Rust adapter over a
  native engine.
- Default keymaps beyond the bootstrap needed to invoke Lisp commands.
- Modeline, menu bar, echo area, completion UI, or theme policy.
- Package-level workflows such as project navigation, grep presentation, or
  git porcelain.

### Mora Lisp Standard Library

Mora Lisp owns:

- `defcommand`, `interactive`, command aliases, command discovery, and help
  metadata.
- Global/local/keymap definitions and prefix maps.
- Major modes and minor modes.
- Hooks, advice, custom variables, faces, themes, and package configuration.
- Default Emacs-like commands such as `find-file`, `save-buffer`,
  `switch-buffer`, `kill-buffer`, `execute-extended-command`, `goto-line`,
  `query-replace`, `describe-*`, and `customize-*`.
- Modeline, menu bar, echo area, minibuffer, completion, buffer list, help
  buffers, and other editor UI policies.
- User init loading and package loading.

The standard library should live in versioned source files loaded at startup,
not as long Rust `match` arms.

## Proposed Directory Shape

```text
mora/
  lisp/                 ; Mora Lisp standard library source
    core.mora           ; defcommand, interactive, hook/advice helpers
    files.mora          ; find-file, save-buffer, write-file
    buffers.mora        ; switch-buffer, kill-buffer, buffer-list
    windows.mora        ; split/delete/select windows
    minibuffer.mora     ; read-from-minibuffer, completion policy
    keymap.mora         ; global-map, ctl-x-map, esc-map
    ui.mora             ; frame/menu/modeline/echo builders
    modes/
      fundamental.mora
      text.mora
      lisp.mora
      rust.mora
    packages/
      grep.mora
      project.mora
      vc.mora
  src/
    lisp/                ; Rust implementation of the Mora Lisp runtime
    mora/
      kernel/            ; editor state and primitive services
      primitives/        ; Rust functions exported to Mora Lisp
      ui/                ; fallback/prebuilt UI components
      display/           ; legacy display adapters until fully retired
```

`mora/lisp/` is intentionally distinct from `mora/src/lisp/`: the first is
Mora Lisp standard-library code, while the second is Rust code implementing the
language runtime.

`mora/src/mora/editor/commands.rs` should shrink over time. Its long `M-x`
dispatch table should be replaced by registered Lisp commands plus a small Rust
fallback for primitive-only bootstrap commands.

## Runtime Data Flow

Startup:

```text
mora binary
  -> create Runtime
  -> register Rust primitives
  -> load mora/lisp/core.mora
  -> load standard command/mode/UI files
  -> load user init file
  -> enter event loop
```

Input:

```text
backend input event
  -> normalized display_protocol::InputEvent
  -> Rust key decoder
  -> Lisp keymap lookup
  -> command invocation
  -> primitive calls mutate kernel state
  -> render frame
```

Rendering:

```text
kernel snapshot
  -> Lisp UI builder or Rust fallback component
  -> display_protocol::UiNode
  -> layout tree with ElementId/UiNodeKind inspector metadata
  -> RenderCommandArray
  -> TUI or WGPU renderer
```

This makes the UI inspectable and replaceable while keeping rendering
backend-neutral.

## Primitive Boundary

Rust primitives should be namespaced and narrow:

```text
mora.buffer/*
mora.window/*
mora.file/*
mora.process/*
mora.display/*
mora.ui/*
mora.keymap/*
mora.command/*
```

Prefer primitives like:

- `mora.buffer/insert`
- `mora.buffer/delete-region`
- `mora.buffer/current-name`
- `mora.file/find-file-noselect`
- `mora.window/switch-to-buffer`
- `mora.command/register`
- `mora.ui/set-component-builder`

Avoid primitives like:

- `mora.editor/do-find-file-command`
- `mora.editor/run-grep-workflow`
- `mora.editor/build-default-modeline`

The first group exposes capability. The second group hardcodes editor policy.

## Migration Plan

1. Introduce a `mora/lisp/` loader and load it before user init files.
2. Move command registration defaults from Rust arrays and `match` arms into
   Lisp `defcommand` files.
3. Split Rust primitives from policy-heavy `lisp_ext` modules. Keep primitive
   modules small and namespaced.
4. Move default UI builders into `mora/lisp/ui.mora`, leaving Rust UI
   components as fallback and test fixtures.
5. Move major/minor mode defaults into Lisp, with Rust adapters only for
   performance engines such as tree-sitter.
6. Convert command completion, `M-x`, help, key descriptions, and mode metadata
   to read from the Lisp command registry.
7. Keep TUI/WGPU rendering strictly behind display-protocol render commands.

Each step should preserve a usable editor and include a screenshot or terminal
render check for Mora UI regressions.

## Current Gaps

The current codebase already has useful pieces:

- `MoraLispBridge` registers primitives and can build UI from Lisp builders.
- `display-protocol` has backend-neutral UI nodes, render commands, and
  inspector metadata.
- Mora UI is split into menu/modeline/editor/echo Rust components.

But the architecture is still too Rust-policy-heavy:

- `MoraEditor` is a large state object with many workflow flags.
- `editor/commands.rs` contains hardcoded `M-x` behavior and command names.
- `editor/input.rs` owns much of the key behavior instead of delegating through
  Lisp keymaps.
- Default UI policy is Rust-first with Lisp override hooks, instead of
  Lisp-first with Rust fallback.
- Many `lisp_ext` modules expose workflow-shaped functions rather than narrow
  primitives.

## Design Decision

Mora should become Lisp-first at the behavior layer, not Lisp-only. The Rust
kernel remains authoritative for state, safety, performance, and platform
integration. Mora Lisp becomes authoritative for editor semantics.

This matches the Emacs power model while still using Rust where Rust gives Mora
real advantages.
