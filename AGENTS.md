# AGENTS.md

## Core Coding Principles

### 1. Think Before Coding
**Don’t assume. Don’t hide confusion. Present trade-offs.**

Before implementing:
- Clearly state your assumptions. If unsure, ask.
- If multiple interpretations exist, present them — don’t silently choose one.
- If there’s a simpler approach, say so. Raise objections when appropriate.
- If something is unclear, stop. Point out the points of confusion. Ask.

### 2. Prioritize Conciseness
**Solve the problem with the least amount of code. Don’t over-speculate.**

- Don’t add features beyond what was requested.
- Don’t create abstractions for one-off code.
- Don’t add unrequested “flexibility” or “configurability.”
- Don’t add error handling for scenarios that can’t happen.
- If you wrote 200 lines when 50 would suffice, rewrite it.

**Self-check:** Would a senior engineer think this is overly complex? If yes, simplify.

### 3. Precise Modifications
**Only touch what must be touched. Only clean up the mess you created.**

When editing existing code:
- Don’t “improve” nearby code, comments, or formatting.
- Don’t refactor things that aren’t broken.
- Match the existing style, even if you prefer a different one.
- If you notice unrelated dead code, mention it — don’t delete it.

When your changes create orphan code:
- Remove imports/variables/functions that became unused *because of your changes*.
- Do not delete pre-existing dead code unless explicitly asked.

**Acceptance criterion:** Every modified line should be directly traceable to the user’s request.

### 4. Goal-Driven Execution
**Define success criteria. Loop and verify until achieved.**

Turn tasks into verifiable goals:
- “Add validation” → “Write tests for invalid inputs, then make them pass”
- “Fix bug” → “Write a test that reproduces the bug, then make it pass”
- “Refactor X” → “Ensure tests pass before and after the refactor”

For multi-step tasks, provide a short plan with verification steps.

### Workflow Orchestration
- **Planning Mode is Default** for any non-trivial task.
- **Sub-Agent Strategy** — delegate research/exploration to keep main context clean.
- **Self-Improvement Closed Loop** — after corrections, update `tasks/lessons.md`.
- **Must Verify Before Completion** — never mark done until proven (tests, logs, comparison).
- **Pursue Elegance (in Moderation)** — ask for more elegant solutions on non-trivial changes.
- **Autonomous Bug Fixing** — fix directly, point to evidence, don’t shift burden.

### Task Management
1. **Plan First** → Write into `tasks/todo.md` as checkable items.
2. **Confirm Plan** with user.
3. **Track Progress** — check off items.
4. **Explain Changes** — high-level summary per step.
5. **Record Results** — append review section.
6. **Capture Lessons** — update `tasks/lessons.md` after corrections.

### Supplementary Core Principles
- **No Laziness** — locate root causes, deliver senior-developer quality.
- **Minimal Blast Radius** — only change what needs changing.

**Signs these principles are working:** Fewer unnecessary changes in diffs, fewer rewrites, clarification questions *before* implementation.

## Build & Test

```bash
cargo build --release          # builds CLI + NAPI
cargo test                     # all Rust tests
cargo test test_org_to_ast     # single test
```

No `npm test`. Format with `cargo fmt` before commits. Use Conventional Commits.

## Coding Principles (Project-specific)
- State assumptions.
- Minimal, surgical edits.
- Match style.
- Verify with `cargo test`.
- Define success criteria upfront.

## Docs Worth Knowing
- `README.md`

## Command Output

Protect context usage. **Any command with unknown or potentially large output must be byte-capped.**

Default pattern:

```bash
COMMAND 2>&1 | head -c 4000
```
