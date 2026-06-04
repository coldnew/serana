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

Widget crates define backend-neutral state. They should describe what a
component needs to render, not how a concrete backend paints it.

---

## Forbidden Patterns

<!-- Patterns that should never be used and why -->

- Do not use stdout preview strings as widget examples; examples must render
  through a backend such as `display-tui`.
- Do not put terminal glyph/layout decisions in `display-protocol-widgets`.

---

## Required Patterns

<!-- Patterns that must always be used -->

- Add new shared widget state to protocol specs first, then implement concrete
  rendering in backend crates.
- Codex-style surfaces belong in backend-neutral protocol specs with explicit
  `Kind` enums for related variants. Keep application runtime behavior outside
  `display-protocol-widgets`; backends should receive titles, rows, body
  widgets, selected indices, status text, and preview lines.

---

## Testing Requirements

<!-- What level of testing is expected -->

- When adding a widget protocol, add at least one backend renderer test or live
  backend example that consumes the protocol.

---

## Code Review Checklist

<!-- What reviewers should check -->

(To be filled by the team)
