# Hermes Agent Refactor

Success criteria for this slice:
- [x] Extract tool-turn execution out of `CodingAgent::execute`.
- [x] Preserve existing tool execution, persistence, and meta-cognition behavior.
- [x] Run formatting and targeted tests.
- [x] Restore workspace test coverage blocked by stale temp-dir and self-evolution tests.
- [x] Add shared runtime configuration for parent agents and subagents.
- [x] Route TUI parent agent construction through shared runtime configuration.
- [x] Add subagent regression coverage for inherited runtime prompt context.
- [x] Add explicit tool registry profiles for core, coding, and Hermes modes.
- [x] Route TUI and subagents through the Hermes tool profile explicitly.
- [x] Add `AgentFactory` to centralize runtime config, tool profile, LSP, and skill-tool construction.
- [x] Route TUI and subagents through `AgentFactory`.
- [x] Extract session message/tool-call persistence and title generation into `SessionRecorder`.
- [x] Extract streaming LLM turn execution into `TurnRunner`.
- [x] Extract mutable per-run messages and accumulated tool calls into `AgentRunState`.
- [x] Replace empty preflight tool-call hook with real `ToolCallValidator`.
- [x] Extract context compression threshold handling into `CompressionGate`.
- [x] Extract cancellation and iteration budget handling into `AgentLifecycle`.
- [x] Move the remaining execution loop into `AgentEngine`.
- [x] Move model tool-schema construction into `ToolRegistry`.
- [x] Move shared runtime policy out of `CodingAgent`.
- [x] Add a named Hermes factory path for powered agent construction.
- [x] Route subagents and public naming through Hermes.
- [x] Make `HermesAgent` the canonical concrete agent type.
- [x] Move the canonical implementation into the `hermes` module.
- [x] Update self-evolution examples to target the Hermes module.
- [x] Make the Hermes factory enforce Hermes power defaults.
- [x] Add named registry constructors for tool power levels.
- [x] Make agent tests request explicit tool power levels.
- [x] Split prompt config from tool-power runtime policy.
- [x] Add explicit direct constructors for powered and custom-tool Hermes agents.
- [x] Add a named Hermes runtime-config constructor.
- [x] Make the core prompt identify Serana as a Hermes agent.
- [x] Keep the `CodingAgent` alias out of the canonical Hermes module.
- [x] Add a named custom factory path beside powered Hermes construction.
- [x] Make tool-registry tests use named power-level constructors.
- [x] Add a direct `HermesAgent::hermes` constructor.
- [x] Deprecate compatibility constructors and aliases inside the crates.
- [x] Rename the middle tool profile to workspace terminology.
- [x] Deprecate the `coding` compatibility module.
- [x] Update parent binary and docs outside `crates` to use Hermes naming.

Review:
- Extracted tool-turn execution into `serana-agent/src/tool_turn.rs`.
- `CodingAgent::execute` now delegates tool execution and meta-cognition recording to `handle_tool_turn`, while keeping session persistence at the agent boundary.
- Replaced `serana-agent` unit-test `tempfile` usage with a crate-local test helper so package tests can run without adding a new workspace lockfile update.
- Replaced equivalent `serana-tools` test `tempfile` usage, fixed self-evolution root resolution to point at the workspace root, and aligned registry tests with LSP tools being opt-in.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target` and `env CARGO_TARGET_DIR=/tmp/serana-target cargo test -q --workspace`.
- `rustfmt --edition 2021 --check serana-agent/src/coding.rs serana-agent/src/tool_turn.rs` passed. Crate-wide rustfmt still reports unrelated pre-existing formatting drift in other files.
- Added `AgentRuntimeConfig` so workspace, skills, and compression policy can travel together.
- `SubagentSpawner` now accepts and applies runtime config before budget/callback overrides.
- Verification passed again with `cargo test -q -p serana-agent --target-dir /tmp/serana-target` and `env CARGO_TARGET_DIR=/tmp/serana-target cargo test -q --workspace`.
- Added `ToolProfile::{Core, Coding, Hermes}` and `ToolRegistry::for_profile`.
- `ToolRegistry::new()` remains compatible, but parent/subagent construction now names `ToolProfile::Hermes` explicitly.
- Added profile boundary tests proving core/coding profiles exclude high-power Hermes tools while Hermes includes self-evolution and external tools.
- Verification passed with `cargo test -q -p serana-tools --target-dir /tmp/serana-target`, `cargo test -q -p serana-agent --target-dir /tmp/serana-target`, and `env CARGO_TARGET_DIR=/tmp/serana-target cargo test -q --workspace`.
- Added `AgentFactory` with explicit opt-ins for LSP tools and skill creation.
- TUI no longer assembles registry extensions directly; it builds a Hermes factory and asks it to construct the agent.
- Subagents now build through the same factory path, preserving runtime policy and tool profile.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target`, `cargo test -q -p serana-tui --target-dir /tmp/serana-target`, and `env CARGO_TARGET_DIR=/tmp/serana-target cargo test -q --workspace`.
- Added `SessionRecorder` so `CodingAgent::execute` delegates user/assistant message persistence, tool-call persistence, and async title generation.
- `CodingAgent::with_session` still exposes the same external API while storing a recorder internally.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target` and `env CARGO_TARGET_DIR=/tmp/serana-target cargo test -q --workspace`.
- Added `TurnRunner` so streaming model calls and delta accumulation are isolated from lifecycle orchestration.
- Added a unit test covering streamed text accumulation and stream-delta callbacks.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target` and `env CARGO_TARGET_DIR=/tmp/serana-target cargo test -q --workspace`.
- Added `AgentRunState` so message history and accumulated tool calls are managed outside the main lifecycle loop.
- `CodingAgent::execute` now updates state through methods instead of directly mutating message/tool-call vectors.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target` and `env CARGO_TARGET_DIR=/tmp/serana-target cargo test -q --workspace`.
- Removed the no-op `validate_tool_calls_before_execution` method from `CodingAgent`.
- Added `ToolCallValidator` to reject unknown tools, duplicate tool-call IDs, and invalid JSON arguments before execution.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target` and `env CARGO_TARGET_DIR=/tmp/serana-target cargo test -q --workspace`.
- Added `CompressionGate` to own compression threshold checks and primary-vs-auxiliary summarizer selection.
- `CodingAgent::execute` now asks the gate whether state messages should be replaced, rather than owning compression logic directly.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target` and `env CARGO_TARGET_DIR=/tmp/serana-target cargo test -q --workspace`.
- Added `AgentLifecycle` to own cancellation checks, budget continuation, and budget-exhausted status emission.
- `CodingAgent::execute` now uses named lifecycle operations instead of open-coding cancellation and iteration budget checks.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target` and `env CARGO_TARGET_DIR=/tmp/serana-target cargo test -q --workspace`.
- Added `AgentEngine` to own the runtime orchestration loop across lifecycle, compression, turn execution, validation, tool execution, and session recording.
- `CodingAgent::execute` now delegates to `AgentEngine`, leaving the agent struct focused on construction and runtime configuration.
- Verification passed with `rustfmt --edition 2021 --check serana-agent/src/coding.rs serana-agent/src/engine.rs`, `cargo test -q -p serana-agent --target-dir /tmp/serana-target`, and `env CARGO_TARGET_DIR=/tmp/serana-target cargo test -q --workspace`.
- Added `ToolRegistry::definitions()` as the canonical conversion from registered tools to model tool schemas.
- `AgentEngine` now asks the registry for tool definitions instead of building schemas locally.
- Verification passed with `cargo test -q -p serana-tools --target-dir /tmp/serana-target` and `cargo test -q -p serana-agent --target-dir /tmp/serana-target`.
- Moved `AgentRuntimeConfig` into `runtime_config.rs`, so shared Hermes runtime policy no longer lives inside `coding.rs`.
- Kept the public `serana_agent::AgentRuntimeConfig` export stable and added coverage for the Hermes default profile.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target`.
- Added `AgentFactory::hermes` to bundle Hermes profile defaults with LSP and skill-tool extensions.
- TUI parent agent construction now uses the named Hermes factory instead of manually repeating profile and extension choices.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target` and `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.
- `SubagentSpawner` now defaults to `AgentFactory::hermes` and preserves Hermes defaults when runtime config is supplied.
- Added the public `HermesAgent` alias while keeping `CodingAgent` compatible, and the agent now reports its runtime name as `hermes-agent`.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target` and `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.
- Renamed the concrete runtime struct to `HermesAgent`; `CodingAgent` remains as a compatibility type alias.
- `AgentFactory::build` now returns `HermesAgent`, and agent tests construct the canonical type directly.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target`.
- Moved the concrete agent implementation from `coding.rs` to `hermes.rs`.
- `coding.rs` is now only a compatibility re-export for old imports.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target`.
- Updated self-evolution tool descriptions to reference `crates/serana-agent/src/hermes.rs` instead of stale coding-agent paths.
- Added a regression test so self-evolution examples keep pointing at the Hermes module.
- Verification passed with `cargo test -q -p serana-tools --target-dir /tmp/serana-target`.
- `AgentFactory::hermes` now forces `ToolProfile::Hermes` even if the supplied runtime config requested a narrower profile.
- `AgentFactory::default` now constructs the powered Hermes path with Hermes profile, LSP tools, and skill-tool support.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target`.
- Added `ToolRegistry::{core,coding,hermes}` so tool power levels can be requested directly instead of only through `ToolProfile`.
- `ToolRegistry::new()` now delegates to `ToolRegistry::hermes()`, and `AgentFactory` routes profile selection through the named constructors.
- Verification passed with `cargo test -q -p serana-tools --target-dir /tmp/serana-target` and `cargo test -q -p serana-agent --target-dir /tmp/serana-target`.
- Tightened `AgentFactory` extension coverage so raw factories prove LSP/skill tools are absent until explicitly enabled.
- Updated agent tests to use `ToolRegistry::hermes()` or `ToolRegistry::core()` based on the power level they actually need.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target`.
- Added `AgentPromptConfig` for workspace, skills, and compression fields that can be applied directly to `HermesAgent`.
- `AgentFactory::build` now applies prompt config after it has already resolved tool power, keeping tool-profile policy in the factory.
- Added coverage proving `HermesAgent::with_runtime_config` does not silently rebuild tools from `tool_profile`.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target`.
- Added `HermesAgent::powered` for direct construction with the full Hermes toolset and `HermesAgent::with_tools` for explicit custom registries.
- `HermesAgent::new` remains compatible by delegating to `with_tools`, while factory construction now uses the explicit custom-tool path.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target`.
- `AgentRuntimeConfig::hermes` now names the default Hermes profile policy, while `AgentRuntimeConfig::new` remains compatible by delegating to it.
- TUI parent-agent construction now uses the named Hermes runtime config before building `AgentFactory::hermes`.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target` and `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.
- The core prompt now introduces Serana as a Hermes agent, and prompt-builder coverage asserts the Hermes wording.
- The `CodingAgent` compatibility alias now lives in `coding.rs`; `hermes.rs` contains only the canonical `HermesAgent` implementation.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target`, targeted rustfmt checks for touched files, and `env CARGO_TARGET_DIR=/tmp/serana-target cargo test -q --workspace`.
- Added `AgentFactory::custom` for the raw no-extension factory path; `AgentFactory::new` remains compatibility glue.
- Factory tests now use `custom` when they intentionally avoid powered Hermes workspace extensions and assert `new` keeps that compatibility behavior.
- Verification passed with `cargo test -q -p serana-agent factory --target-dir /tmp/serana-target`, `rustfmt --edition 2021 --check serana-agent/src/factory.rs`, and `env CARGO_TARGET_DIR=/tmp/serana-target cargo test -q --workspace`.
- Updated `serana-tools` registry tests to use `ToolRegistry::core` or `ToolRegistry::hermes` according to the tool power they assert.
- `ToolRegistry::new` is now exercised only by the compatibility test that proves it still maps to powered Hermes tools.
- Verification passed with `cargo test -q -p serana-tools --target-dir /tmp/serana-target`, `rustfmt --edition 2021 --config skip_children=true --check serana-tools/src/lib.rs`, and `env CARGO_TARGET_DIR=/tmp/serana-target cargo test -q --workspace`.
- Added `HermesAgent::hermes` as the named full-power direct constructor, and kept `HermesAgent::powered` as compatibility glue.
- Agent tests now use `HermesAgent::hermes` for canonical construction and cover `powered` only as compatibility.
- Verification passed with `cargo test -q -p serana-agent hermes --target-dir /tmp/serana-target`, `rustfmt --edition 2021 --check serana-agent/src/hermes.rs`, and `env CARGO_TARGET_DIR=/tmp/serana-target cargo test -q --workspace`.
- Deprecated crate-level compatibility APIs: `CodingAgent`, `HermesAgent::new`, `HermesAgent::powered`, `AgentFactory::new`, `AgentRuntimeConfig::new`, and `ToolRegistry::new`.
- Internal defaults now use canonical Hermes constructors, with `new`/`powered` references retained only in compatibility tests.
- Parent-level files originally still referenced `CodingAgent` after an interrupted elevated edit; they were later migrated with `apply_patch`.
- Added `ToolProfile::Workspace` and `ToolRegistry::workspace` for the non-Hermes workspace tool profile.
- `ToolProfile::Coding` and `ToolRegistry::coding` remain compatibility names, while factory and tests now use workspace terminology for the middle power level.
- Verification passed with `cargo test -q -p serana-tools --target-dir /tmp/serana-target`, `cargo test -q -p serana-agent factory --target-dir /tmp/serana-target`, targeted rustfmt checks, and `env CARGO_TARGET_DIR=/tmp/serana-target cargo test -q --workspace`.
- Deprecated the public `coding` module so downstream users are pointed at the `hermes` module and `HermesAgent`.
- Verification passed with `cargo test -q -p serana-agent --target-dir /tmp/serana-target`, `rustfmt --edition 2021 --config skip_children=true --check serana-agent/src/lib.rs`, and `env CARGO_TARGET_DIR=/tmp/serana-target cargo test -q --workspace`.
- Updated the parent CLI binary to import `HermesAgent`, describe itself as a Hermes agent, and construct direct runs with `HermesAgent::hermes`.
- Updated parent architecture/spec docs to describe `HermesAgent` and the Hermes factory path instead of the old `CodingAgent` runtime path.
- Updated the parent package description and README summary to describe Serana as a personal Hermes agent.
- Workspace verification now passes without Hermes migration deprecation warnings from `src/main.rs`; only pre-existing unrelated warnings remain.

# TUI Rewrite Toward oh-my-pi

Success criteria for this slice:
- [x] Derive the first visual slice from `ref/oh-my-pi` docs and theme/status-line components.
- [x] Port the default TUI palette to the reference Titanium colors.
- [x] Start status-line parity with a compact `π` identity segment, dot separators, and shorter path/git/token labels.
- [x] Run focused `serana-tui` tests.

Review:
- Ported `serana-tui` default theme constants and JSON token defaults to the reference Titanium palette.
- Added a `Pi` status segment and made built-in presets start with the compact identity/model/status shape.
- Changed status-line rendering to use dot separators, shorter mode/context/session labels, `@workspace`, and cleaner git dirty markers.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.
- `rustfmt --edition 2021 --check serana-tui/src/theme.rs serana-tui/src/status_line.rs serana-tui/src/ui.rs` reports formatting changes in pre-existing nearby code; I left that churn out of this slice.

Success criteria for status-line layout slice:
- [x] Split built-in status presets into left and right segment groups like `ref/oh-my-pi`.
- [x] Add a right-side session segment.
- [x] Render a width-aware filled gap between left and right status groups.
- [x] Drop overflowing status segments from the right, then the left, to keep the footer within terminal width.
- [x] Run focused `serana-tui` tests.

Review:
- Status presets now model left/right segment groups instead of one linear list.
- The default preset now follows the reference shape: identity/model/mode/path/git/context/cost on the left and session/runtime metadata on the right.
- Status-line rendering joins visible segments, fills the center gap with the theme border style, and prunes overflow.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for welcome panel slice:
- [x] Move the welcome title into the welcome component border like `ref/oh-my-pi`.
- [x] Hide the separate global header while the welcome screen is active.
- [x] Keep the two-column welcome layout inside one bordered panel with a center divider.
- [x] Show real recent session metadata in the welcome panel.
- [x] Run focused `serana-tui` tests.

Review:
- Welcome now renders as one bordered panel with embedded `Serana v...` title.
- The welcome screen no longer consumes a separate top header band.
- The right column now lists up to three stored sessions with timestamps instead of always showing a placeholder.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for editor status-border slice:
- [x] Move status content from a standalone footer row into the input editor top border.
- [x] Reuse the existing left/right status layout as border title content.
- [x] Remove the extra status row from the main layout.
- [x] Run focused `serana-tui` tests.

Review:
- `render_status_line` became `build_status_line`, returning border content instead of drawing a footer widget.
- The input block now uses that status line as its top-border title.
- The main layout now has header/content/input areas only, matching the reference editor/status integration more closely.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for tool execution frame slice:
- [x] Render tool calls as a distinct framed component like the reference tool execution UI.
- [x] Include tool status in the tool header.
- [x] Preserve existing per-tool body rendering for file, diff, command, LSP, AST, and generic tools.
- [x] Run focused `serana-tui` tests.

Review:
- Tool-call rendering now wraps existing tool bodies in a rounded title/footer frame.
- The frame title includes status labels (`pending`, `running`, `done`, `error`) alongside the existing status icon and tool name.
- Existing tool-specific body renderers now accept styled header lines, preserving header color.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for message rendering slice:
- [x] Render user messages as padded Markdown blocks with the configured user-message background.
- [x] Remove the extra assistant name header so assistant text reads like the reference component.
- [x] Render thinking traces as simple italic text instead of a nested box.
- [x] Run focused `serana-tui` tests.

Review:
- User messages now use the Markdown renderer and `USER_MSG_BG` styling instead of raw indented text.
- Assistant messages start directly with rendered Markdown content, closer to `AssistantMessageComponent`.
- Thinking content no longer adds box chrome that the reference assistant component does not use.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for width-safe conversation slice:
- [x] Sanitize tabs in rendered conversation lines.
- [x] Clamp rendered conversation lines to the terminal content width.
- [x] Preserve span styling while clamping.
- [x] Run focused `serana-tui` tests.

Review:
- Added a span-preserving `clamp_line` helper for the conversation surface.
- Message, tool, todo, status, and BTW lines are clamped after composition before rendering.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for autocomplete selector slice:
- [x] Render autocomplete rows with an accent cursor prefix like the reference select list.
- [x] Use accent styling for selected values and muted styling for descriptions.
- [x] Truncate value and description text to fit the popup.
- [x] Show a muted count line when autocomplete results are clipped.
- [x] Run focused `serana-tui` tests.

Review:
- Command autocomplete now uses `›` selection prefixes and selected-text styling instead of relying on list highlight state.
- Item values and descriptions are width-limited before rendering.
- The popup grows by one row for a scroll/count indicator when more results exist than are visible.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for dialog selector slice:
- [x] Render dialog rows with the same explicit cursor-prefix structure as the reference select list.
- [x] Derive the highlighted row from the selected item inside the filtered list.
- [x] Truncate labels and descriptions to the available dialog width.
- [x] Show no-match and clipped-result count lines with muted styling.
- [x] Run focused `serana-tui` tests.

Review:
- Dialog selectors now render explicit `›` cursor rows instead of relying on Ratatui list highlight state.
- Filtered selections are mapped from the selected original item to the selected filtered row before rendering.
- Labels and descriptions are sanitized and truncated to the popup width.
- Empty filters show muted no-match text, and clipped lists show a muted selected/total count line.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for editor chrome slice:
- [x] Render the input editor with rounded border symbols like the reference editor.
- [x] Add horizontal editor padding inside the border.
- [x] Keep status content embedded in the top border.
- [x] Preserve existing input cursor, wrapping, and scrolling behavior.
- [x] Run focused `serana-tui` tests.

Review:
- The input editor block now uses rounded border glyphs.
- Horizontal padding is applied inside the input border, matching the reference editor's bordered layout.
- The existing top-border status line, cursor drawing, wrapping, and scroll behavior remain in place.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for Markdown wrapping slice:
- [x] Wrap styled Markdown paragraph lines to the renderer width instead of relying on outer clipping.
- [x] Preserve blockquote borders on wrapped quote lines.
- [x] Align wrapped list-item continuation lines under the item text.
- [x] Add focused tests for long paragraph, blockquote, and list wrapping.
- [x] Run focused `serana-tui` tests.

Review:
- Markdown flushing now wraps styled spans to the renderer width before conversation-level clamping.
- Blockquote wraps repeat the quote border prefix on continuation rows.
- List item wraps insert continuation padding aligned under the item text.
- Added tests for long paragraph, blockquote, and list wrapping.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for tool metadata slice:
- [x] Render concise tool argument summaries instead of raw JSON for generic tool calls.
- [x] Show concise LSP/AST argument metadata when available.
- [x] Render generic tool results with status and line-count metadata before previews.
- [x] Add focused tests for generic, LSP, AST, and result summary behavior.
- [x] Run focused `serana-tui` tests.

Review:
- Generic tool calls now render compact key/value argument summaries and skip private `_` fields.
- LSP and AST renderers show concise argument metadata before results when useful fields are present.
- Generic result previews now include success/error status and line-count metadata before expanded preview lines.
- Added focused tests for generic arg summaries, LSP/AST summaries, and multi-line result summaries.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for todo checklist slice:
- [x] Render todos as checklist rows using reference status markers.
- [x] Show incomplete/total todo counts in the todo header.
- [x] Keep status-specific styling for pending, in-progress, done, and abandoned todos.
- [x] Add focused tests for todo header/counts and status markers.
- [x] Run focused `serana-tui` tests.

Review:
- Todo rendering now uses checklist rows with `[ ]`, `[/]`, `[x]`, and `[-]` status markers.
- The todo block header shows incomplete and total todo counts.
- Pending, in-progress, done, and abandoned todos keep distinct styling.
- Added focused tests for todo counts and marker rendering.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for BTW panel slice:
- [x] Render BTW notes in a rounded custom panel instead of ad hoc sharp lines.
- [x] Make the panel width-aware so borders and note text fit the content area.
- [x] Preserve warning/accent label styling and muted note text.
- [x] Add focused tests for panel borders and width limits.
- [x] Run focused `serana-tui` tests.

Review:
- BTW notes now render in a rounded, width-aware panel with muted borders.
- The panel title remains warning-styled and note text remains muted.
- Added focused tests for rounded panel rendering and width bounds.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for processing loader slice:
- [x] Render pending assistant work as a bordered loader panel like reference execution loaders.
- [x] Keep spinner frame and pending message text visible.
- [x] Make the loader panel width-aware, including narrow widths.
- [x] Add focused tests for loader panel structure and width limits.
- [x] Run focused `serana-tui` tests.

Review:
- Pending assistant work now renders inside a sharp bordered loader panel.
- The panel includes the active spinner frame, cancel hint, and pending message preview.
- Added focused tests for loader structure and width limits.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for status notice slice:
- [x] Render status messages as labeled notice rows instead of flat arrow lines.
- [x] Make status notices width-aware and safe on narrow terminals.
- [x] Preserve informational styling for labels and muted styling for message text.
- [x] Add focused tests for notice labels and width limits.
- [x] Run focused `serana-tui` tests.

Review:
- Status messages now render as `[status]` notice rows instead of arrow-prefixed lines.
- Notice text is truncated to the available width and remains muted.
- Added focused tests for label rendering and width limits.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for thinking Markdown slice:
- [x] Render assistant thinking through the Markdown renderer instead of raw lines.
- [x] Preserve thinking color/italic styling across rendered spans.
- [x] Keep thinking text width-aware and compatible with final conversation clamping.
- [x] Add focused tests for formatted and wrapped thinking text.
- [x] Run focused `serana-tui` tests.

Review:
- Assistant thinking text now renders through the Markdown renderer.
- Rendered thinking spans are patched with the existing thinking style.
- Added focused tests for Markdown formatting and wrapping behavior.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for dialog chrome slice:
- [x] Render selector dialogs with rounded border chrome.
- [x] Keep the selector help/footer text width-safe.
- [x] Preserve existing filter, list, and count behavior.
- [x] Add focused tests for width-safe dialog help text.
- [x] Run focused `serana-tui` tests.

Review:
- Selector dialogs now use rounded popup borders.
- Dialog footer help is truncated to the available width.
- Existing filter, select-list rows, and clipped count behavior were left intact.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for autocomplete scrolling slice:
- [x] Keep the selected autocomplete item visible when selection moves past the first five rows.
- [x] Reserve popup space for the count line instead of letting list rows overlap it.
- [x] Preserve existing row styling, cursor marker, and selected/total metadata.
- [x] Add focused tests for autocomplete visible-range behavior.
- [x] Run focused `serana-tui` tests.

Review:
- Autocomplete now renders a scrolling visible range so the selected command remains visible after moving beyond the first five rows.
- The popup reserves the final inner row for selected/total metadata instead of letting command rows overlap it.
- Existing row cursor marker, selected styling, and muted descriptions were preserved.
- Added focused tests for top, scrolled, empty, and clamped visible-range behavior.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for tool width-safety slice:
- [x] Clamp rendered tool-execution lines at the tool component boundary.
- [x] Preserve existing styles while truncating long spans.
- [x] Keep image-detection notices width-safe too.
- [x] Add focused tests for long command/result output.
- [x] Run focused `serana-tui` tests.

Review:
- Tool execution output is now clamped at the `render_tool_call` boundary, including image-detection notices appended after renderer-specific content.
- Span styles are preserved while long spans are truncated.
- The shared truncation helper now counts characters instead of bytes.
- Added focused tests for long tool output and style-preserving line clamps.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for editor display sanitation slice:
- [x] Render editor text through a dedicated display-line helper.
- [x] Replace tabs and strip control characters before editor text reaches the terminal.
- [x] Keep cursor marker and cursor-character styling on the current editor row.
- [x] Clamp editor display lines to the input content width.
- [x] Add focused tests for sanitation, cursor styling, and width limits.
- [x] Run focused `serana-tui` tests.

Review:
- Input text rendering now flows through `render_editor_display_lines` instead of being assembled inline in `render_input`.
- Editor display text replaces tabs with spaces and strips control characters before rendering.
- Current-row cursor marker and reversed cursor-character styling were preserved.
- Editor display lines are clamped to the input content width.
- Added focused tests for sanitation, cursor styling, and width limits.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for editor wrapping slice:
- [x] Wrap long editor display rows into multiple width-safe lines instead of truncating them.
- [x] Preserve sanitized text and cursor styling across wrapped display lines.
- [x] Keep placeholder rendering as a single width-safe line.
- [x] Add focused tests for wrapping and cursor preservation.
- [x] Run focused `serana-tui` tests.

Review:
- Editor display rows now wrap into multiple width-safe lines instead of truncating user text.
- Sanitized text, cursor marker, and reversed cursor-character styling are preserved through wrapping.
- Empty-editor placeholder rendering stays a single clamped line.
- Added focused tests for wrapped long lines and cursor preservation after wrapping.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for editor word-wrap slice:
- [x] Prefer wrapping editor display rows at whitespace boundaries.
- [x] Fall back to character-level splitting for words longer than the content width.
- [x] Preserve cursor marker and reversed cursor-character styling through word wrapping.
- [x] Add focused tests for word wrapping and long-word fallback.
- [x] Run focused `serana-tui` tests.

Review:
- Editor display wrapping now groups whitespace and word tokens so rows wrap at word boundaries when possible.
- Long words still split at the content width as a fallback.
- Cursor marker and reversed cursor-character styling are preserved as styled characters through wrapping.
- Added focused tests for word-boundary wrapping and long-word splitting.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for editor wrap helper slice:
- [x] Extract styled editor wrap token construction from `wrap_editor_display_line`.
- [x] Extract token application logic so long-word fallback and whitespace wrapping are named.
- [x] Preserve existing editor wrapping behavior and tests.
- [x] Run focused `serana-tui` tests.

Review:
- Editor wrapping now builds styled wrap tokens through `editor_wrap_tokens`.
- Normal token wrapping and long-token fallback are split into named helpers.
- Existing cursor-preserving word-wrap behavior was preserved.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for Markdown table slice:
- [x] Collect Markdown table headers and rows instead of ignoring table tags.
- [x] Render tables with rounded borders, a header separator, and padded cells.
- [x] Keep rendered table lines width-safe on narrow terminals.
- [x] Add focused tests for table structure and width bounds.
- [x] Run focused `serana-tui` tests.

Review:
- Markdown tables now collect header and body rows from pulldown table events.
- Tables render with rounded borders, a header separator, and padded cells.
- Narrow tables fall back to truncated pipe-separated rows when there is not enough border/cell width.
- Added focused tests for table border structure and width safety.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for Markdown image slice:
- [x] Track Markdown image destination and alt text instead of treating the alt text as plain text.
- [x] Render image markdown as a compact width-safe placeholder line.
- [x] Preserve link rendering behavior outside image tags.
- [x] Add focused tests for image path/alt rendering and width bounds.
- [x] Run focused `serana-tui` tests.

Review:
- Markdown image tags now collect destination URL/path and alt text.
- Images render as compact `[image: alt] target` placeholders styled like links.
- Image placeholders are truncated before the normal markdown wrapper sees them.
- Added focused tests for image alt/path rendering and width limits.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for Markdown strikethrough slice:
- [x] Give `MarkdownTheme` an explicit strikethrough style.
- [x] Apply that style to `~~text~~` spans instead of rendering them as normal text.
- [x] Preserve existing bold/italic/link stack behavior.
- [x] Add focused tests for strikethrough modifier rendering.
- [x] Run focused `serana-tui` tests.

Review:
- Markdown theme now exposes an explicit strikethrough style.
- `~~text~~` spans now render with Ratatui's crossed-out modifier.
- Existing style-stack behavior for other inline styles was left intact.
- Added a focused span-level strikethrough test.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for Markdown link de-duplication slice:
- [x] Track visible link text while rendering inline link spans.
- [x] Avoid appending the link target when visible text already equals the href.
- [x] Treat `mailto:` targets as duplicates when visible text equals the email address.
- [x] Preserve current target suffix behavior for descriptive link text.
- [x] Add focused tests for duplicate and descriptive links.
- [x] Run focused `serana-tui` tests.

Review:
- Markdown link rendering now tracks visible link text while inside link spans.
- Link targets are omitted when visible text already equals the href or the `mailto:` email address.
- Descriptive link text still appends the target suffix.
- Added focused tests for duplicate URL links, duplicate mailto links, and descriptive links.
- Live worktree tests are blocked by unrelated dirty `serana-llm` export edits; clean-checkout verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target-link-verify`.

Success criteria for Markdown HTML slice:
- [x] Render inline HTML as plain text instead of dropping it.
- [x] Render HTML blocks as plain text through the normal wrapping path.
- [x] Keep HTML output width-safe.
- [x] Add focused tests for inline HTML, block HTML, and width bounds.
- [x] Run focused `serana-tui` tests.

Review:
- Inline and block HTML events now render as plain text through the normal Markdown text path.
- HTML output uses existing wrapping/clamping behavior.
- Added focused tests for inline HTML, block HTML, and width limits.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for Markdown link underline slice:
- [x] Render Markdown links with underline styling as well as link color.
- [x] Keep target suffix spans visually consistent with link text.
- [x] Preserve existing link target de-duplication behavior.
- [x] Add focused tests for link underline styling.
- [x] Run focused `serana-tui` tests.

Review:
- Markdown link style now combines aquamarine color with underline styling.
- Link target suffixes use the same link style, so they remain visually consistent.
- Existing link target de-duplication tests still pass.
- Added a focused span-level underline test for link text.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for Markdown code block width slice:
- [x] Wrap raw code block lines through the markdown width wrapper.
- [x] Wrap highlighted code block spans through the markdown width wrapper.
- [x] Preserve code fence lines and code indentation.
- [x] Add focused tests for long raw and highlighted code block lines.
- [x] Run focused `serana-tui` tests.

Review:
- Code block content lines now flow through `wrap_spans` before being appended to markdown output.
- Both raw and syntax-highlighted code block lines are width-safe.
- Code fence lines and the existing two-space code indentation were preserved.
- Added focused tests for long raw and highlighted fenced code blocks.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for Markdown task-list marker slice:
- [x] Render checked markdown task markers as `[x]`.
- [x] Render unchecked markdown task markers as `[ ]`.
- [x] Keep wrapped task-list continuation lines aligned under the item text.
- [x] Add focused tests for checked, unchecked, and wrapped task items.
- [x] Run focused `serana-tui` tests.

Review:
- Markdown task-list parsing is now enabled in pulldown options.
- Checked and unchecked task markers render as `[x]` and `[ ]`.
- Task-list continuation prefixes include both the list bullet and checkbox marker.
- Added focused tests for checked, unchecked, and wrapped task list items.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for Markdown footnote slice:
- [x] Enable pulldown footnote parsing.
- [x] Render inline footnote references as visible `[^label]` markers.
- [x] Render footnote definitions with their `[^label]:` prefix.
- [x] Keep rendered footnote content width-safe.
- [x] Add focused tests for references, definitions, and width bounds.
- [x] Run focused `serana-tui` tests.

Review:
- Markdown footnote parsing is now enabled in pulldown options.
- Inline footnote references render as visible styled `[^label]` markers.
- Footnote definitions render with a styled `[^label]:` prefix.
- Added focused tests for references, definitions, and width safety.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.

Success criteria for Markdown math slice:
- [x] Enable pulldown math parsing if supported locally.
- [x] Render inline math as visible `$...$` text instead of dropping it.
- [x] Render display math as visible `$$...$$` text instead of dropping it.
- [x] Keep rendered math width-safe.
- [x] Add focused tests for inline math, display math, and width bounds.
- [x] Run focused `serana-tui` tests.

Review:
- Markdown math parsing is now enabled in pulldown options.
- Inline math renders as visible styled `$...$` text.
- Display math renders as visible styled `$$...$$` text.
- Added focused tests for inline math, display math, and width safety.
- Verification passed with `cargo test -q -p serana-tui --target-dir /tmp/serana-target`.
