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
