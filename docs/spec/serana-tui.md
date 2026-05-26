# serana-tui — Terminal User Interface

## Overview

`serana-tui` provides a ratatui-based terminal interface for interactive chat with the agent. It handles rendering, keyboard input, streaming response display, markdown formatting, syntax highlighting, slash commands, and session management.

**Crate path:** `crates/serana-tui/`

## Dependencies

- **Internal:** `serana-core`, `serana-llm`, `serana-agent`, `serana-tools`
- **External:** `ratatui`, `crossterm`, `pulldown-cmark`, `syntect`, `strip-ansi-escapes`, `tokio`, `serde`, `serde_json`, `anyhow`, `tracing`, `dirs`

## Module Map

| Module | File | Purpose |
|--------|------|---------|
| `lib` | `lib.rs` | Entry point: `run()` function |
| `app` | `app.rs` | Application state machine |
| `tui` | `tui.rs` | Terminal setup/teardown |
| `ui` | `ui.rs` | Widget layout and rendering |
| `event` | `event.rs` | Event loop (keyboard, resize, tick) |
| `editor` | `editor.rs` | Multi-line text editor |
| `dialog` | `dialog.rs` | Modal dialog system |
| `slash_commands` | `slash_commands.rs` | Slash command dispatch |
| `markdown` | `markdown.rs` | Markdown rendering |
| `syntax` | `syntax.rs` | Syntax highlighting |
| `diff` | `diff.rs` | Diff rendering |
| `image` | `image.rs` | Image display (sixel/kitty) |
| `symbols` | `symbols.rs` | Unicode/ASCII symbols |
| `theme` | `theme.rs` | Color themes |
| `status_line` | `status_line.rs` | Bottom status bar |
| `tool_execution` | `tool_execution.rs` | Tool call progress display |
| `terminal` | `terminal.rs` | Terminal utilities |

## Entry Point (`lib.rs`)

```rust
pub fn run(
    workspace: PathBuf,
    model: Option<String>,
    provider: Option<String>,
    config: Config,
) -> Result<()>
```

### Startup Sequence

1. Create `Tui` (terminal raw mode, alternate screen, mouse capture)
2. Create `EventHandler` + mpsc channels
3. Build `HermesAgent` through `AgentFactory::hermes` with:
   - `AgentCallbacks` wired to `stream_tx` for streaming deltas
   - Self-evolution tools
   - LSP tools (if language servers detected)
   - Skill tools
4. Initialize `SessionStore` + `SkillStore`
5. Create `App` with initial state
6. Enter main event loop

## App State Machine (`app.rs`)

```rust
pub struct App {
    messages: Vec<ChatMessage>,
    editor: Editor,
    mode: AppMode,
    session: SessionStore,
    pending_response: Option<String>,
    autocomplete: AutocompleteState,
    todos: Vec<TodoItem>,
    btw_notes: Vec<String>,
    slash_commands: SlashCommandRegistry,
    tokens_used: TokenUsage,
    start_time: Instant,
}
```

### App Modes

```
Normal ←──────────────────────────┐
  │ (type message, press Enter)   │
  ▼                               │
Processing                        │
  │ (agent executes,              │
  │  streaming response)          │
  │                               │
  └───────────────────────────────┘
```

Additional modes: `Input`, `Dialog`, `Editor`, `FileBrowser`

### ChatMessage

```rust
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Vec<ToolCallDisplay>,
    pub thinking: Option<String>,
}

pub enum MessageRole { User, Agent, System }
```

### Key Methods

| Method | Purpose |
|--------|---------|
| `handle_key_event(event)` | Process keyboard input based on current mode |
| `tick()` | Periodic update (check streaming, update stats) |
| `session_elapsed()` | Duration since session start |
| `cost_estimate()` | Estimated cost based on tokens used |
| `token_rate()` | Tokens per minute |
| `set_pending_response(content)` | Update streaming content |
| `clear_pending_response()` | Finalize response as ChatMessage |
| `todo_reminder_message()` | Generate TODO reminder for system prompt |

## Tui (`tui.rs`)

Terminal setup and teardown.

```rust
pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}
```

- `new()` — enables raw mode, alternate screen, mouse capture
- `restore()` — disables raw mode, returns to main screen
- `terminal()` — returns mutable reference for drawing

## Event Loop (`event.rs`)

```rust
pub struct EventHandler {
    rx: mpsc::Receiver<AppEvent>,
}

pub enum AppEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    Tick,
    StreamDelta(String),
    ResponseComplete(String),
}
```

- **Tick rate:** 60 FPS (~16ms)
- **Crossterm events:** keyboard, resize
- **mpsc channels:** streaming deltas and completed responses from agent
- Non-blocking `try_recv` to avoid lock-step rendering

## Editor (`editor.rs`)

Multi-line text editor widget.

```rust
pub struct Editor {
    lines: Vec<String>,
    cursor: (usize, usize),  // (row, col)
    scroll: usize,
    undo_stack: Vec<EditorState>,
}
```

### Features

- Multi-line editing with cursor movement
- Undo/redo via state stack
- Path completion (Tab key)
- History (up/down arrows)

## Slash Commands (`slash_commands.rs`)

```rust
pub struct SlashCommandRegistry {
    commands: HashMap<String, SlashCommand>,
}
```

### Built-in Commands

| Command | Purpose |
|---------|---------|
| `/quit` | Exit application |
| `/model` | Switch LLM model |
| `/help` | Show available commands |
| `/stats` | Show token usage and session stats |
| `/session` | Session management (list, load, new) |
| `/todo` | Add/list/complete TODO items |
| `/btw` | Add "by the way" notes |
| `/copy` | Copy last response to clipboard |
| `/skill` | List/create/load skills |
| `/theme` | Switch color theme |
| `/thinking` | Toggle thinking/reasoning display |
| `/compact` | Force context compression |
| `/clear` | Clear conversation |
| `/reload` | Reload configuration |
| `/queue` | Queue messages for batch processing |

## Markdown Rendering (`markdown.rs`)

Uses `pulldown-cmark` for parsing, converts to ratatui `Paragraph`.

### Supported Elements

- Headings (H1-H4)
- Code blocks with syntax highlighting
- Inline code
- Bold, italic, strikethrough
- Unordered and ordered lists
- Links (displayed as `[text](url)`)
- Blockquotes
- Horizontal rules

## Syntax Highlighting (`syntax.rs`)

Uses `syntect` for code block highlighting.

- Configurable theme (via `/theme` command)
- Supports all languages in syntect's default syntax set
- Falls back to plain text for unknown languages

## Diff Rendering (`diff.rs`)

Colored diff display for git diffs and file comparisons.

- Green for additions (`+`)
- Red for deletions (`-`)
- Gray for context lines
- Header lines highlighted differently

## Image Support (`image.rs`)

Terminal image display.

- Detects terminal capabilities (sixel, kitty, iTerm2)
- Falls back to placeholder text if not supported
- Used for displaying images in agent responses

## Dialog System (`dialog.rs`)

Modal dialogs for user interaction.

```rust
pub struct Dialog {
    pub title: String,
    pub content: DialogContent,
    pub actions: Vec<DialogAction>,
}

pub enum DialogContent {
    Confirm(String),           // Yes/No
    Input(String, String),     // Label + default
    Select(String, Vec<String>), // Label + options
    List(Vec<ListItem>),       // Scrollable list
}
```

Used for: model selection, theme selection, session selection, confirmations.

## Theme System (`theme.rs`)

```rust
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub user_msg: Color,
    pub agent_msg: Color,
    pub system_msg: Color,
    pub code_bg: Color,
    pub diff_add: Color,
    pub diff_del: Color,
    // ...
}
```

- Multiple built-in themes
- Switchable via `/theme` command
- Stored in `~/.serana/theme.toml`

## Status Line (`status_line.rs`)

Bottom bar showing:
- Current model name
- Token usage (in/out)
- Session duration
- Estimated cost
- Iteration count

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| mpsc channels for streaming | Agent sends text deltas via `unbounded_channel`; TUI polls with `try_recv` |
| Separate response channels | `stream_tx/rx` for in-flight deltas; `response_tx/rx` for complete responses |
| No async in render path | Drawing is synchronous; streaming state polled before `draw()` |
| Pending response tracking | `set_pending_response()` updates streaming content; on completion, appends as ChatMessage |
| Session persistence on each message | Both user and agent messages saved to SessionStore after fully received |
| 60 FPS tick rate | Smooth scrolling and responsive input without excessive CPU |
| Builder pattern for HermesAgent | Flexible construction with optional components |
| Crossterm over termion | Cross-platform (Windows + Unix); ratatui's recommended backend |

## Keybindings

| Key | Normal Mode | Processing Mode |
|-----|-------------|-----------------|
| Enter | Send message | (ignored) |
| Ctrl+C | Quit | Cancel current operation |
| Tab | Autocomplete | (ignored) |
| Up/Down | History | Scroll |
| `/` | Start slash command | (ignored) |
| Escape | Clear input | (ignored) |
| Ctrl+L | Clear screen | (ignored) |
