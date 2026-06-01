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

The GPU backend should consume the shared command stream indirectly, via the same cell buffer replay used by the terminal backend.

- Keep GPU-specific work limited to atlas, surfaces, and presentation.
- Treat `RenderCommandArray` as the upstream UI contract.

---

## Forbidden Patterns

<!-- Patterns that should never be used and why -->

- Do not reimplement layout or widget painting inside `display-wgpu`.
- Do not add alternate UI trees that bypass the shared display protocol.

---

## Required Patterns

<!-- Patterns that must always be used -->

- Build the screen buffer from shared render commands before handing it to the GPU renderer.
- Keep the GPU renderer focused on `ScreenBuffer` presentation.

---

## Testing Requirements

<!-- What level of testing is expected -->

- Update tests when the command stream or replay changes visible output.
- Verify the windowed path with a live screenshot after changing rendering behavior.

---

## Code Review Checklist

<!-- What reviewers should check -->

- Confirm the WGPU path uses the shared render pipeline end to end.
