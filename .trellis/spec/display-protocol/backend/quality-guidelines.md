# Quality Guidelines

> Code quality standards for backend development.

---

## Overview

<!--
Document your project's quality standards here.

Questions to answer:
- What patterns are forbidden?
- What linting rules do you enforce?
- What are your testing requirements?
- What code review standards apply?
-->

The shared display layer now uses a command-stream contract:

- `collect_render_commands(node, width, height)` is the canonical way to turn a `UiNode` tree into backend-agnostic draw instructions.
- `render_commands_to_buffer(commands, width, height)` is the canonical replay step for cell-based backends.
- `paint(node, width, height)` exists only as a compatibility wrapper over the command pipeline.

---

## Forbidden Patterns

<!-- Patterns that should never be used and why -->

- Do not add new direct buffer-first rendering paths when the command stream can express the same behavior.
- Do not reintroduce separate layout logic in individual backends.

---

## Required Patterns

<!-- Patterns that must always be used -->

- Any new UI feature must be expressible through the shared command pipeline first.
- Backends should consume `RenderCommandArray` or a replayed `ScreenBuffer`, not bespoke per-widget layout code.

---

## Testing Requirements

<!-- What level of testing is expected -->

- Add/adjust unit tests for command generation and replay parity.
- If a UI widget changes output, verify the generated commands as well as the rendered cells.

---

## Code Review Checklist

<!-- What reviewers should check -->

- Check that new UI behavior can be represented in `RenderCommand`.
- Check that renderer changes do not duplicate layout or widget-specific painting.
