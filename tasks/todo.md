# Refactor Serana Based on ref/pi

- [x] Create Trellis task.
- [x] Inspect `ref/pi` coding-agent docs and command surface.
- [x] Inspect Serana CLI and TUI slash-command handling.
- [x] Confirm scope for the first implementation slice.
- [x] Implement `/login codex` TUI parity.
- [x] Run formatting and relevant tests.
- [x] Commit `/login codex` slice.
- [x] Implement `/hotkeys` TUI command.
- [x] Run formatting and relevant checks for `/hotkeys`.
- [x] Commit `/hotkeys` slice.
- [x] Wire `/copy` to clipboard.
- [x] Run formatting and relevant checks for `/copy`.
- [x] Commit `/copy` slice.
- [x] Implement `/name <name>` session title command.
- [x] Run formatting and relevant checks for `/name`.
- [x] Commit `/name` slice.
- [x] Implement `/export [file]` HTML export.
- [x] Run formatting and relevant checks for `/export`.
- [x] Commit `/export` slice.
- [x] Record results.
- [x] Implement auto-retry with exponential backoff on LLM failures.
- [x] Add context overflow detection and recovery (compact + retry).
- [x] Add `@path` file attachment in TUI input.
- [x] Add `!command` bash quick-execute.
- [x] Add `!!` re-execute last command.
- [x] Add prompt template system (`{{N}}` arg substitution from .serana/prompts/*.md).
- [x] Add session tree branching (fork_session, list_children, parent_id).
- [x] Add `/fork`, `/tree`, `/new`, `/logout` slash commands.
- [x] Fix pre-existing CompressionConfig test initializer errors.
- [x] Run cargo fmt, cargo check, cargo test.

## Results

- **Auto-Retry**: Configurable via `config.toml` `[retry]` section (enabled, max_retries=3, base_delay_ms=2000, max_delay_ms=60000). Detects rate limits (429), server errors (500/502/503/529), network errors. Exponential backoff with jitter.
- **Context Overflow**: Detects "context_length_exceeded", "prompt is too long", etc. Triggers compression gate + single retry.
- **@path**: Resolves `~`, relative, and absolute paths. Wraps content in `<file>` tags.
- **!cmd**: Runs `sh -c <cmd>` with workspace cwd. Truncates output >8000 chars.
- **!!**: Re-executes last `!command`.
- **Prompt Templates**: `.serana/prompts/*.md` loaded as slash commands with `{{1}}..{{N}}` arg substitution. `{{0}}` = full args.
- **Session Tree**: SQLite `parent_id` column with migration. `fork_session()` copies messages. `list_children()` returns forks.
- **New Commands**: `/fork`, `/tree`, `/new`, `/logout` registered and wired in TUI.
- `cargo fmt -p serana`: passed.
- `cargo check -p serana`: passed (0 errors, 6 pre-existing warnings).
- `cargo test -p serana --lib`: 298 passed, 1 failed (pre-existing `model_command_shows_argument_completions`).
- `cargo test -p serana --bin serana`: passed.
- Fixed 5 pre-existing CompressionConfig test initializer errors.