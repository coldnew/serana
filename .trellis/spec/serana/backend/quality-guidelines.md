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

Serana backend and TUI code should preserve command input exactly where
whitespace has semantic meaning.

---

## Forbidden Patterns

<!-- Patterns that should never be used and why -->

- Do not call `trim()` on slash command input before deciding whether argument
  autocomplete should run; `/model ` must remain distinct from `/model`.

---

## Required Patterns

<!-- Patterns that must always be used -->

- Use `trim_start()` when slash command matching should tolerate leading spaces
  but still preserve trailing spaces for argument completion.

---

## Testing Requirements

<!-- What level of testing is expected -->

- Add or update slash command tests when changing command parsing,
  autocomplete, or completion application behavior.

---

## Code Review Checklist

<!-- What reviewers should check -->

(To be filled by the team)
