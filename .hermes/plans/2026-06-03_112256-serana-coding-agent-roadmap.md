# Serana Coding Agent Roadmap Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Make Serana a dependable coding agent: project-aware, tool-disciplined, safe when editing, able to verify work, and useful inside both CLI/TUI and Mora.

**Architecture:** Treat Serana as an agent runtime with five reinforcing loops: context acquisition, planning/task tracking, surgical code editing, execution/verification, and learning. The current code already has an agent engine, prompt builder, session store, tool registry, LSP/tree-sitter tools, subagents, memory, skills, checkpoints, and TUI; the main work is to turn those pieces into a strict coding-agent workflow instead of a generic chat-with-tools loop.

**Tech Stack:** Rust workspace, `serana` binary crate, `serana-core`, `serana-llm`, tree-sitter, LSP, Tokio, ratatui/TUI, existing `ToolRegistry` profiles, SQLite-backed sessions/memory.

---

## Current context from repo inspection

- `README.md` says `serana` is the personal agent in this workspace.
- `AGENTS.md` already defines the desired behavior: think before coding, minimal changes, plan first, verify before completion, sub-agent strategy, lessons capture.
- `serana/src/agent/engine.rs` has the main loop: build system prompt, call LLM with tools, validate/execute tool calls, compress context, handle checkpoints/rewind, record session.
- `serana/src/agent/prompt_builder.rs` injects AGENTS.md and context, but its core prompt is still broad and weak for serious coding-agent behavior.
- `serana/src/tools/mod.rs` already includes the right primitive tools: file read/write/edit, AST edit/grep, search, LSP, git, shell, todo, memory, rules, review, browser, MCP, DAP, etc.
- `serana/src/agent/factory.rs` builds powered Hermes agents with LSP and skill tools by default.
- `serana/src/agent/subagent.rs` exists, but is not yet exposed as an agent tool in the registry.
- `serana/src/agent/coding.rs` is only a deprecated alias for `HermesAgent`, so “coding agent” currently means prompt/tool policy, not a distinct workflow.
- Verification check run: `cargo test -p serana --lib -q` passed: `234 passed; 0 failed`.

## Product north star

Serana should feel like a senior coding teammate with local superpowers:

1. **Reads before editing** — never guesses file structure or APIs.
2. **Plans non-trivial work** — maintains explicit TODOs and success criteria.
3. **Edits surgically** — minimal blast radius, exact diffs, no opportunistic refactors.
4. **Runs the work** — builds/tests/reproduces issues rather than asserting success.
5. **Uses code intelligence** — tree-sitter/LSP for symbols, references, diagnostics, rename/format.
6. **Can delegate safely** — subagents for exploration/review, not uncontrolled nested chaos.
7. **Learns locally** — project rules, lessons, skills, and memory feed future turns.
8. **Works inside Mora** — eventually chat/edit/apply-patch/project-agent commands in editor buffers.

## Proposed phased approach

### Phase 1: Strengthen the default coding-agent prompt and workflow policy

**Objective:** Make the existing agent behave like a disciplined coding agent before adding new machinery.

**Files:**
- Modify: `serana/src/agent/prompt_builder.rs`
- Test: `serana/src/agent/prompt_builder.rs` tests
- Possibly modify: `docs/spec/serana-agent.md`

**Changes:**
- Replace the generic core prompt with coding-agent-specific rules aligned with `AGENTS.md`:
  - inspect before answering/editing;
  - write a plan/TODO for non-trivial tasks;
  - one `in_progress` task at a time;
  - prefer `read_file`, `search`, `ast_*`, and LSP before shell spelunking;
  - prefer surgical `edit_file`/AST edits over whole-file rewrites;
  - run verification commands before final response;
  - state blockers honestly.
- Add explicit “forbidden shortcuts”: no fabricated command output, no claiming tests passed without tool evidence, no unrelated cleanup.
- Teach it when to ask clarification vs use obvious defaults.

**Validation:**
- Update/add tests that assert prompt contains key workflow rules.
- Run: `cargo test -p serana prompt_builder -q`.

### Phase 2: Make planning and TODO tracking first-class

**Objective:** Ensure Serana can decompose multi-step coding tasks and track progress without relying only on model memory.

**Files:**
- Modify: `serana/src/tools/todo_write.rs`
- Modify: `serana/src/agent/prompt_builder.rs`
- Possibly create: `serana/src/agent/planning.rs`
- Test: relevant unit tests under `serana/src/tools/todo_write.rs` or new module tests

**Changes:**
- Define a small task-state contract: `pending`, `in_progress`, `completed`, `cancelled`.
- Prompt rule: exactly one `in_progress` task during execution.
- Add todo validation: reject multiple `in_progress` items if the tool currently accepts that.
- Add final-response guidance: summarize completed tasks and exact verification command/output.

**Validation:**
- Unit tests for TODO state validation.
- Integration-style mock LLM test if existing test harness supports it.
- Run: `cargo test -p serana todo -q`.

### Phase 3: Expose subagents as a real coding tool

**Objective:** Let Serana delegate bounded research/review tasks while keeping the parent context clean.

**Files:**
- Create or modify: `serana/src/tools/subagent.rs`
- Modify: `serana/src/tools/mod.rs`
- Modify: `serana/src/agent/subagent.rs` if API needs task constraints
- Test: `serana/src/tools/subagent.rs` tests

**Changes:**
- Add a `delegate_task` tool with a constrained schema:
  - `goal: string`
  - `context: string`
  - `tool_profile: core|workspace|hermes` defaulting to workspace
  - maybe `max_iterations`
- Leaf subagents should not recursively delegate by default.
- Require subagent final summaries to include evidence handles: paths, commands run, test output snippets.
- Parent prompt should use subagents for exploration/review, not for tiny edits.

**Validation:**
- Mock LLM tests for spawning and collecting results.
- Ensure tool appears only in intended profiles.
- Run: `cargo test -p serana subagent -q`.

### Phase 4: Add a coding task runner around `AgentEngine`

**Objective:** Move important behavior out of prompt-only policy into code-level scaffolding.

**Files:**
- Create: `serana/src/agent/coding_workflow.rs`
- Modify: `serana/src/agent/mod.rs`
- Modify: `serana/src/agent/hermes.rs`
- Possibly modify: `serana/src/main.rs`

**Changes:**
- Introduce an optional `CodingWorkflow` wrapper that can:
  - classify task type: question, small edit, large feature, debug, review;
  - require initial context-gathering for code edits;
  - enforce plan/TODO before multi-step changes;
  - enforce verification attempt before completion when files changed;
  - record changed files and commands run.
- Keep this lightweight: the LLM still decides details, but the runtime checks minimum discipline.

**Validation:**
- Mock-agent tests for “file changed but no verification” producing a warning or enforced follow-up.
- Run focused tests plus `cargo test -p serana --lib -q`.

### Phase 5: Improve edit safety and diff ergonomics

**Objective:** Make edits reliable, reviewable, and reversible.

**Files:**
- Modify: `serana/src/tools/fs.rs`
- Modify: `serana/src/tools/hashline.rs`
- Modify: `serana/src/tools/checkpoint.rs`
- Modify: `serana/src/tui/diff.rs`
- Test: edit/hashline/checkpoint tests

**Changes:**
- Before first write/edit in a turn, auto-create a checkpoint or require explicit checkpoint tool use.
- Return unified diffs from write/edit tools, not just success.
- Add stale-anchor diagnostics with suggested reread when `edit_file` fails.
- Add “dry run” option for risky edits.
- Preserve secret redaction behavior already present in `fs.rs`.

**Validation:**
- Tests for diff output and checkpoint creation.
- Tests for stale anchor failure messages.
- Run: `cargo test -p serana fs hashline checkpoint -q`.

### Phase 6: Make verification a first-class artifact

**Objective:** The final answer should be grounded in real command/test/lint output.

**Files:**
- Modify: `serana/src/tools/shell.rs`
- Modify: `serana/src/agent/session_recorder.rs`
- Create or modify: `serana/src/agent/verification_summary.rs`
- Test: shell/session/verification tests

**Changes:**
- Track commands run during the turn: command, cwd, exit code, truncated stdout/stderr.
- Add a final summary helper that exposes “verification evidence” to the prompt.
- Encourage common project commands from `AGENTS.md`, here `cargo test` and `cargo fmt` before commits.
- If verification cannot run, final response must say exactly why.

**Validation:**
- Unit tests for command recording and redaction/truncation.
- Run: `cargo test -p serana verification shell session -q`.

### Phase 7: Project context and rules system polish

**Objective:** Make Serana excellent in unfamiliar repos by quickly finding the right instructions.

**Files:**
- Modify: `serana/src/agent/gatherer.rs`
- Modify: `serana/src/tools/rules.rs`
- Modify: `serana/src/agent/prompt_builder.rs`
- Test: gatherer/rules tests

**Changes:**
- Gather common agent instruction files: `AGENTS.md`, `CLAUDE.md`, `.cursorrules`, `.github/copilot-instructions.md`, `.trellis/workflow.md` if present.
- Keep byte caps and avoid huge prompt injection.
- Make `.trellis/spec/.../quality-guidelines.md` discoverable by file path when editing matching crates.
- Inject only concise summaries by default; let agent read details on demand.

**Validation:**
- Tests with temp workspaces containing nested instruction files.
- Run: `cargo test -p serana gatherer rules prompt_builder -q`.

### Phase 8: Code intelligence defaults

**Objective:** Prefer semantic navigation and diagnostics over blind regex.

**Files:**
- Modify: `serana/src/tools/code_intel.rs`
- Modify: `crates/serana-tree-sitter/src/summary.rs`
- Modify: `serana/src/agent/prompt_builder.rs`
- Test: code-intel tests

**Changes:**
- Add or improve tools for:
  - symbol outline;
  - document symbols;
  - references;
  - diagnostics;
  - code actions;
  - rename;
  - format.
- Prompt guidance: use AST/LSP before editing APIs or public symbols.
- Add fallback behavior if no language server is configured.

**Validation:**
- Unit tests for tree-sitter summaries.
- Mock LSP tests where possible.
- Run: `cargo test -p serana code_intel -q` and `cargo test -p serana-tree-sitter -q`.

### Phase 9: TUI and Mora integration for coding workflows

**Objective:** Make Serana useful where the user codes, not only as a CLI command.

**Files:**
- Modify: `serana/src/tui/*`
- Modify: `serana/src/tui/slash_commands.rs`
- Modify: `mora/src/...` later when wiring editor commands
- Possibly create docs under `docs/`

**Changes:**
- Add slash commands for:
  - `/plan`
  - `/review`
  - `/fix-test`
  - `/explain-file`
  - `/apply-patch`
  - `/delegate`
- Display tool calls, diffs, TODO state, and verification commands cleanly.
- Later expose Mora commands:
  - explain region;
  - edit region with Serana;
  - apply patch;
  - generate commit message;
  - project-aware chat buffer.

**Validation:**
- Unit tests for slash command parsing.
- Manual TUI smoke test.
- Run: `cargo test -p serana slash_commands -q`.

### Phase 10: Regression suite for agent behavior

**Objective:** Prevent prompt/workflow regressions as Serana evolves.

**Files:**
- Create: `serana/tests/agent_workflow.rs` or similar
- Create fixtures under: `serana/tests/fixtures/`
- Modify mock LLM helpers as needed

**Scenarios:**
- Question-only task should read relevant files before answering.
- Small edit should read file, edit minimal lines, run focused test.
- Multi-step feature should create TODO, progress items, run verification.
- Failed test should trigger debugging loop, not final success.
- Tool failure should be reported honestly.
- Subagent review should not mutate files unless explicitly allowed.

**Validation:**
- Run: `cargo test -p serana agent_workflow -q`.

## Priority recommendation

Start with these three PR-sized slices:

1. **Prompt/workflow hardening** (`prompt_builder.rs` + tests). Highest leverage, smallest risk.
2. **Verification evidence tracking** (record commands and changed files; final summary rules). This makes Serana trustworthy.
3. **Subagent tool exposure** (bounded delegation for exploration/review). This makes Serana more capable on larger coding tasks.

After those, invest in edit safety and code intelligence polish.

## Risks and tradeoffs

- **Too much prompt text can reduce model focus.** Keep rules short, test prompt length, and move details into skills/rules where possible.
- **Runtime enforcement can become annoying.** Start with warnings and structured reminders before hard-blocking final responses.
- **Subagents can waste tokens or make conflicting edits.** Default them to research/review and require explicit mutation permissions.
- **LSP setup varies by project.** Always provide tree-sitter/search fallback.
- **Mora integration may distract from core agent quality.** Do CLI/TUI coding-agent reliability first, then editor UX.

## Open questions

1. Should Serana remain “HermesAgent with coding policy”, or should there be a distinct `CodingAgent` type again?
   - My recommendation: keep one `HermesAgent`, add optional `CodingWorkflow` policy and factory presets.
2. Should Serana auto-run formatters/tests after edits?
   - My recommendation: auto-suggest and strongly prompt; only auto-run safe project-local commands unless configured.
3. Should subagents share memory/session state?
   - My recommendation: share read-only project context and parent-provided task context; summarize back into parent, do not share full mutable state initially.
4. Should “good coding agent” target CLI first or Mora first?
   - My recommendation: CLI/TUI first because it is easier to test; Mora commands after the workflow is stable.

## Done definition for the roadmap

Serana can be considered a good coding agent when it reliably demonstrates this loop on real repos:

1. Reads project instructions and relevant source.
2. Creates a concise plan for non-trivial changes.
3. Makes minimal edits with visible diffs.
4. Runs focused verification.
5. Uses diagnostics/search/LSP to debug failures.
6. Produces a final answer with files changed and exact verification evidence.
7. Records useful lessons/skills without polluting memory with stale task progress.
