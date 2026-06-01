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

The terminal backend should stay thin:

- Prefer replaying `RenderCommandArray` into a `ScreenBuffer` over writing widget-specific drawing code in `display-tui`.
- Keep terminal-specific work limited to input, terminal lifecycle, and buffer presentation.

---

## Forbidden Patterns

<!-- Patterns that should never be used and why -->

- Do not add UI layout code in the terminal layer.
- Do not duplicate cell painting logic that already exists in `display-protocol`.

---

## Required Patterns

<!-- Patterns that must always be used -->

- Use `TuiTerminal::render_command_array` for command-stream rendering.
- Keep `render_screen_buffer` available only as a lower-level presentation helper.

---

## Testing Requirements

<!-- What level of testing is expected -->

- Update tests when command replay changes rendering.
- Verify the TUI path with a real terminal render when behavior changes.

---

## Code Review Checklist

<!-- What reviewers should check -->

- Confirm the TUI path consumes shared render output, not bespoke widget drawing.
