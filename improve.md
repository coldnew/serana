# TUI Feature Gap: serana vs. oh-my-pi

## Symbol System

oh-my-pi defines 150+ Unicode/Nerd Font symbols across 3 presets (`unicode`, `nerd`, `ascii`) with 2 spinner types. Serana hard-codes symbols inline.

**To match:** Create a symbol module mapping all UI symbols (status, spinners, box-drawing, etc.) matching oh-my-pi's `unicode` preset.

## Status Line

oh-my-pi has 21 configurable segment types (model, mode, git, pr, tokens, cost, context, time, session, etc.) across 7 presets with auto-overflow and powerline arrows. Serana has 6 hard-coded segments.

**To match:** Make status line segments configurable with presets; add missing segments (cost, token rate, session time, thinking level, hostname, PR link).

## Diff Rendering

oh-my-pi does word-level intra-line diff highlighting with syntax-highlighted context lines, line number gutters, and collapse markers. Serana has basic `+`/`-` per-line color.

**To match:** Implement word-level diff (split on word boundaries and color changed segments), add line numbers, show syntax-highlighted context around changes.

## Markdown

oh-my-pi renders code blocks with syntax highlighting (via native Rust `pi-natives`) and caches rendered output (256-entry LRU). Serana uses pulldown-cmark with raw text for code blocks.

**To match:** Add syntax highlighting to code blocks in the markdown renderer. Consider reusing `syntect` or `tree-sitter` highlighters already in the workspace.

## Tool Execution

oh-my-pi supports multi-file streaming diffs, 10+ tool-specific renderers, async diff computation with debouncing, and image display from tool results. Serana has single-file basic expand/collapse.

**To match:** Tool-specific output formatters (bash, python, edit, read, write, etc.), streaming diff stabilization, image rendering from tool results.

## Message Types

oh-my-pi has 10+ message types (skill, branch summary, compaction summary, BTW, todo, hook, custom). Serana has 3 (user, agent, system).

**To match:** Render BTW notes panel, todo reminders, skill execution messages, compaction summaries.

## Input

oh-my-pi has a multi-line editor with autocomplete (file paths, commands, emoji), undo stack, kill ring, and emacs keybindings. Serana has single-line input.

**To match:** Multi-line input, path completion (reuse tree-sitter/workspace-aware search), undo support.

## Images

oh-my-pi renders tool result images via Kitty/iTerm2/Sixel protocols with async PNG conversion. Serana has no image support.

**To match:** Implement terminal image protocol detection and rendering from tool results (e.g., `matplotlib` output).

## Theme System

oh-my-pi has 62 color tokens + 7 background tokens with variable references, auto dark/light detection, colorblind mode, live reload via file watcher. Serana has 25 colors and 8 styles, hard-coded.

**To match:** Move theme to a JSON-configurable format with the full oh-my-pi token set; support auto dark/light detection via terminal query.

## Interactive Dialogs

oh-my-pi has selectors for: model, theme, settings, session, plugin, OAuth, MCP, history search. Serana has none.

**To match:** Implement model selector, theme selector, session selector as ratatui popup/overlay widgets.

## Priority Order

1. **Symbol system** (low effort, high visual impact)
2. **Diff word-level highlighting** (medium effort, high functional impact)
3. **Syntax-highlighted code blocks** (medium effort, high visual impact)
4. **BTW panel + todo reminders** (low effort — state exists already)
5. **Rich tool execution rendering** (high effort, high functional impact)
6. **Configurable status line** (medium effort)
7. **Path autocomplete** (medium effort)
8. **Image support** (low effort if terminal supports it)
9. **Theme system** (high effort)
10. **Interactive dialogs** (high effort)
