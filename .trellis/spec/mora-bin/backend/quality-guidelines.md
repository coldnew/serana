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

(To be filled by the team)

---

## Forbidden Patterns

<!-- Patterns that should never be used and why -->

(To be filled by the team)

---

## Required Patterns

<!-- Patterns that must always be used -->

Mora's default startup frame should stay Emacs-like:

- The menu bar uses Emacs-style labels (`File`, `Edit`, `Options`, `Buffers`,
  `Tools`, `Help`).
- The scratch buffer starts in Lisp major mode.
- The default editor pane does not show line numbers unless the line-number
  minor mode is enabled.
- The current line is not given a special background by default; only the
  cursor cell is emphasized.

---

## Testing Requirements

<!-- What level of testing is expected -->

When changing the default frame, add at least one regression test for the
startup state or rendered modeline so the Emacs-like defaults do not drift.

---

## Code Review Checklist

<!-- What reviewers should check -->

(To be filled by the team)
