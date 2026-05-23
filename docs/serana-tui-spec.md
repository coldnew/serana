# serana-tui — Terminal User Interface

## Purpose

Ratatui-based terminal interface for interactive chat with the agent. Handles rendering, keyboard input, streaming response display, markdown formatting, and syntax highlighting.

## Dependencies

- `serana-core`, `serana-llm`, `serana-agent`, `serana-tools`
- `ratatui`, `crossterm`, `pulldown-cmark`, `syntect`, `strip-ansi-escapes`

## Module Map

| Module | Purpose |
|--------|---------|
| `tui.rs` | Terminal setup/teardown (raw mode, alternate screen) |
| `app.rs` | Application state machine, message list, mode tracking |
| `ui.rs` | Widget layout and conditional rendering |
| `event.rs` | Event loop combining keyboard + resize + tick |
| `dialog.rs` | Yes/no confirmations, text input prompts |
| `diff.rs` | Diff rendering with colored +/- lines |
| `editor.rs` | Multi-line text editor widget |
| `image.rs` | Image rendering (sixel/kitty) |
| `markdown.rs` | Markdown to ratatui `Paragraph` conversion |
| `slash_commands.rs` | `/session`, `/clear`, `/model`, etc. |
| `status_line.rs` | Bottom status bar rendering |
| `symbols.rs` | Unicode symbol constants |
| `syntax.rs` | Syntect-based syntax highlighting |
| `theme.rs` | Color theme definition |
| `tool_execution.rs` | Tool call progress display |

## App State Machine

```
Normal
  │  (user types message, presses Enter)
  ▼
Processing
  │  (agent executes, streaming response received)
  ▼
Normal
```

Modes: `Normal`, `Processing`, `Input`, `Dialog`, `Editor`, `FileBrowser`

## ChatMessage

Each message stored in `app.messages`:
- `role`: User | Agent | System
- `content`: message text
- `tool_calls`: vector of tool call display data
- `thinking`: optional thinking/reasoning content

## Event Loop

- 60 FPS tick rate (~16ms)
- Crossterm events: keyboard, resize
- mpsc channels for streaming deltas and responses
- Keybindings: Enter=send, Ctrl+C=quit, Tab=autocomplete, `/`=slash commands

## Tui Entry Point

`serana_tui::run(workspace, model, provider, config)`:
1. Creates `Tui` (terminal raw mode)
2. Creates `EventHandler` + mpsc channels
3. Initializes `CodingAgent` with `AgentCallbacks` (streaming delta → `stream_tx`)
4. Creates `ToolRegistry`, registers self-evolve + LSP tools
5. Initializes `SessionStore` + `SkillStore`
6. Creates `App` and enters render/event loop

## Markdown Rendering

- Uses `pulldown-cmark` for parsing
- Supports: headings, code blocks (with syntax highlight), lists, inline code, bold/italic, links
- Syntax highlighting via `syntect` with configurable theme

## Design Decisions

- **mpsc channels for streaming**: Agent sends text deltas via `unbounded_channel`, TUI polls in main loop. Non-blocking `try_recv` to avoid lock-step rendering.
- **Separate response channel**: Streaming deltas go through `stream_tx/rx`, final response through `response_tx/rx`. This allows the TUI to distinguish "in-flight" from "complete."
- **No async in render path**: Drawing is synchronous. Streaming state is polled before `draw()`.
- **Pending response tracking**: `app.set_pending_response()` updates streaming content; on completion, appends as a `ChatMessage` and clears pending state.
- **Session persistence on each message**: Both user and assistant messages saved to `SessionStore` after they're fully received.
