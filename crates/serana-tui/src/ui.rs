//! UI rendering — vertical flow like oh-my-pi.

use crate::app::{App, AppMode, ChatMessage, MessageRole, TodoItem, ToolCallStatus};
use crate::component::Container;
use crate::components::{BoxWidget, Markdown, Spacer, Text};
use crate::style::{Colors, Style};
use crate::tool_execution::{ToolExecution, ToolState};
use crate::tui::Tui;

/// Theme colors matching oh-my-pi dark theme.
pub struct Theme {
    pub accent: Style,
    pub success: Style,
    pub warning: Style,
    pub error: Style,
    pub dim: Style,
    pub muted: Style,
    pub info: Style,
    pub user_fg: Style,
    pub agent_fg: Style,
    pub thinking: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: Style::new().fg(Colors::BRIGHT_MAGENTA).bold(),
            success: Style::new().fg(Colors::BRIGHT_GREEN).bold(),
            warning: Style::new().fg(Colors::BRIGHT_YELLOW).bold(),
            error: Style::new().fg(Colors::BRIGHT_RED).bold(),
            dim: Style::new().fg(Colors::GRAY).dim(),
            muted: Style::new().fg(Colors::GRAY),
            info: Style::new().fg(Colors::BRIGHT_CYAN),
            user_fg: Style::new().fg(Colors::BRIGHT_GREEN),
            agent_fg: Style::new().fg(Colors::BRIGHT_BLUE),
            thinking: Style::new().fg(Colors::GRAY).italic(),
        }
    }
}

/// PI logo block characters (oh-my-pi style)
const PI_LOGO: &[&str] = &[
    "▀████████████▀",
    " ╘███    ███  ",
    "  ███    ███  ",
    "  ███    ███  ",
    " ▄███▄  ▄███▄ ",
];

/// Spinner frames for waiting indicator
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Get current spinner frame based on tick count
fn spinner_frame(tick: u64) -> &'static str {
    SPINNER_FRAMES[(tick % SPINNER_FRAMES.len() as u64) as usize]
}

/// Apply magenta→cyan gradient to logo (matching oh-my-pi)
fn gradient_logo(lines: &[&str]) -> Vec<String> {
    // 256-color gradient: bright magenta → bright cyan
    let colors = [
        "\x1b[38;5;199m", // bright magenta
        "\x1b[38;5;171m", // magenta-purple
        "\x1b[38;5;135m", // purple
        "\x1b[38;5;99m",  // purple-blue
        "\x1b[38;5;75m",  // cyan-blue
        "\x1b[38;5;51m",  // bright cyan
    ];
    let reset = "\x1b[0m";

    lines
        .iter()
        .map(|line| {
            let mut result = String::new();
            let color_count = colors.len();
            let step = (line.len() / color_count).max(1);
            let mut color_idx = 0;

            for (i, char) in line.chars().enumerate() {
                if i > 0 && i % step == 0 && color_idx < color_count - 1 {
                    color_idx += 1;
                }
                if char != ' ' {
                    result.push_str(colors[color_idx]);
                    result.push(char);
                    result.push_str(reset);
                } else {
                    result.push(char);
                }
            }
            result
        })
        .collect()
}

/// Render welcome screen with true two-column layout (oh-my-pi style).
fn render_welcome(container: &mut Container, version: &str, model: &str, provider: &str) {
    let theme = Theme::default();

    // Get terminal width (assume 80 minimum)
    let width = 80;
    let max_width = 100;
    let box_width = width.min(max_width).max(4);

    // Column layout
    let dual_content_width = box_width - 3; // │ + │ + │
    let preferred_left_col = 26;
    let min_left_col = 14; // logo width (updated to match oh-my-pi)
    let min_right_col = 20;

    let left_min_content_width = min_left_col
        .max(visible_width("Welcome back!"))
        .max(visible_width(model))
        .max(visible_width(provider));
    let desired_left_col = preferred_left_col
        .min((dual_content_width as f64 * 0.35) as usize)
        .max(min_left_col);

    let dual_left_col = if dual_content_width >= min_right_col + 1 {
        desired_left_col.min(dual_content_width - min_right_col)
    } else {
        (dual_content_width - 1).max(1)
    };
    let dual_right_col = (dual_content_width - dual_left_col).max(1);

    let show_right_column =
        dual_left_col >= left_min_content_width && dual_right_col >= min_right_col;
    let left_col = if show_right_column {
        dual_left_col
    } else {
        box_width - 2
    };
    let right_col = if show_right_column { dual_right_col } else { 0 };

    // Logo with gradient
    let logo_colored = gradient_logo(PI_LOGO);

    // Left column - centered content
    let mut left_lines: Vec<String> = vec![
        "".to_string(),
        center_text(&theme.accent.apply("Welcome back!"), left_col),
        "".to_string(),
    ];

    for line in &logo_colored {
        left_lines.push(center_text(line, left_col));
    }

    left_lines.push("".to_string());
    left_lines.push(center_text(&theme.muted.apply(model), left_col));
    left_lines.push(center_text(&theme.dim.apply(provider), left_col));
    left_lines.push("".to_string());

    // Right column content
    let mut right_lines: Vec<String> = Vec::new();

    if show_right_column {
        let separator_width = (right_col - 2).max(0);
        let separator = format!(" {}", theme.dim.apply(&"─".repeat(separator_width)));

        // Tips section
        right_lines.push(format!(" {}", theme.accent.apply("Tips")));
        right_lines.push(format!(
            " {}{}",
            theme.dim.apply("?"),
            theme.muted.apply(" for keyboard shortcuts")
        ));
        right_lines.push(format!(
            " {}{}",
            theme.dim.apply("#"),
            theme.muted.apply(" for prompt actions")
        ));
        right_lines.push(format!(
            " {}{}",
            theme.dim.apply("/"),
            theme.muted.apply(" for commands")
        ));
        right_lines.push(format!(
            " {}{}",
            theme.dim.apply("!"),
            theme.muted.apply(" to run bash")
        ));
        right_lines.push(format!(
            " {}{}",
            theme.dim.apply("$"),
            theme.muted.apply(" to run python")
        ));
        right_lines.push(separator.clone());

        // LSP Servers section
        right_lines.push(format!(" {}", theme.accent.apply("LSP Servers")));
        right_lines.push(format!("  {}", theme.dim.apply("○ No LSP servers")));
        right_lines.push(separator);

        // Recent sessions section
        right_lines.push(format!(" {}", theme.accent.apply("Recent sessions")));
        right_lines.push(format!(" {}", theme.dim.apply(" • No recent sessions")));
        right_lines.push("".to_string());
    }

    // Border characters
    let h = theme.dim.apply("─");
    let v = theme.dim.apply("│");
    let tl = theme.dim.apply("┌");
    let tr = theme.dim.apply("┐");
    let bl = theme.dim.apply("└");
    let br = theme.dim.apply("┘");
    let _tee = theme.dim.apply("┬");
    let tee_up = theme.dim.apply("┴");

    let mut lines: Vec<String> = Vec::new();

    // Top border with embedded title
    let title = format!(" Serana v{} ", version);
    let title_prefix = "───";
    let title_styled = format!(
        "{}{}",
        theme.dim.apply(title_prefix),
        theme.muted.apply(&title)
    );
    let title_vis_len = title_prefix.chars().count() + title.chars().count();
    let title_space = box_width - 2;

    if title_vis_len >= title_space {
        lines.push(format!(
            "{}{}{}",
            tl,
            truncate_to_width(&title_styled, title_space),
            tr
        ));
    } else {
        let after_title = title_space - title_vis_len;
        lines.push(format!(
            "{}{}{}{}",
            tl,
            title_styled,
            theme.dim.apply(&"─".repeat(after_title)),
            tr
        ));
    }

    // Content rows
    let max_rows = if show_right_column {
        left_lines.len().max(right_lines.len())
    } else {
        left_lines.len()
    };

    for i in 0..max_rows {
        let left = fit_to_width(left_lines.get(i).cloned().unwrap_or_default(), left_col);
        if show_right_column {
            let right = fit_to_width(right_lines.get(i).cloned().unwrap_or_default(), right_col);
            lines.push(format!("{}{}{}{}{}", v, left, v, right, v));
        } else {
            lines.push(format!("{}{}{}", v, left, v));
        }
    }

    // Bottom border
    if show_right_column {
        lines.push(format!(
            "{}{}{}{}{}",
            bl,
            h.repeat(left_col),
            tee_up,
            h.repeat(right_col),
            br
        ));
    } else {
        lines.push(format!("{}{}{}", bl, h.repeat(left_col), br));
    }

    container.push(Spacer::new(1));
    for line in lines {
        container.push(Text::styled(line, Style::default()));
    }
    container.push(Spacer::new(1));
}

/// Calculate visible width (strips ANSI escapes)
fn visible_width(text: &str) -> usize {
    strip_ansi_escapes::strip_str(text).chars().count()
}

/// Center text within a given width
fn center_text(text: &str, width: usize) -> String {
    let vis_len = visible_width(text);
    if vis_len >= width {
        return truncate_to_width(text, width);
    }
    let left_pad = (width - vis_len) / 2;
    let right_pad = width - vis_len - left_pad;
    format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
}

/// Fit string to exact width with ANSI-aware truncation/padding
fn fit_to_width(text: String, width: usize) -> String {
    let vis_len = visible_width(&text);
    if vis_len > width {
        truncate_to_width(&text, width)
    } else if vis_len < width {
        format!("{}{}", text, " ".repeat(width - vis_len))
    } else {
        text
    }
}

/// Truncate string to width, preserving ANSI codes
fn truncate_to_width(text: &str, width: usize) -> String {
    let vis_len = visible_width(text);
    if vis_len <= width {
        return text.to_string();
    }

    let ellipsis = "…";
    let max_width = width.saturating_sub(1);
    let mut result = String::new();
    let mut current_width = 0;
    let mut in_escape = false;

    for char in text.chars() {
        if char == '\x1b' {
            in_escape = true;
        }

        if in_escape {
            result.push(char);
            if char == 'm' {
                in_escape = false;
            }
        } else if current_width < max_width {
            result.push(char);
            current_width += 1;
        }
    }

    format!("{}{}", result, ellipsis)
}

/// Render chat messages with oh-my-pi styling.
fn render_messages(container: &mut Container, messages: &[ChatMessage]) {
    let theme = Theme::default();

    for msg in messages {
        match msg.role {
            MessageRole::User => {
                container.push(Spacer::new(1));
                // User prefix with checkmark
                container.push(Text::styled("  ✓ You", theme.success));
                for line in msg.content.lines() {
                    container.push(Text::styled(format!("    {}", line), Style::default()));
                }
            }
            MessageRole::Agent => {
                container.push(Spacer::new(1));
                // Agent prefix with star
                container.push(Text::styled("  ★ Serana", theme.accent));

                // Thinking block (if present)
                if let Some(thinking) = &msg.thinking {
                    container.push(Text::styled("  ┌─ thinking ─", theme.thinking));
                    for line in thinking.lines() {
                        container.push(Text::styled(format!("  │ {}", line), theme.thinking));
                    }
                    container.push(Text::styled("  └─", theme.thinking));
                    container.push(Spacer::new(1));
                }

                for line in msg.content.lines() {
                    container.push(Markdown::new(format!("    {}", line)));
                }

                // Inline tool calls using ToolExecution component
                for tool in &msg.tool_calls {
                    let mut tool_ui = ToolExecution::new(&tool.name);
                    tool_ui.set_state(match tool.status {
                        ToolCallStatus::Pending => ToolState::Pending,
                        ToolCallStatus::Running => ToolState::Running,
                        ToolCallStatus::Success => ToolState::Success,
                        ToolCallStatus::Error => ToolState::Error,
                    });
                    if !tool.args.is_empty() {
                        tool_ui.set_args(&tool.args);
                    }
                    if let Some(result) = &tool.result {
                        tool_ui.set_output(result);
                    }
                    container.push(tool_ui);
                }
            }
            MessageRole::System => {
                container.push(Spacer::new(1));
                container.push(Text::styled(
                    format!("  ⚠ System: {}", msg.content),
                    theme.warning,
                ));
            }
        }
    }
}

/// Render pending/streaming messages with animated waiting indicator.
fn render_pending(container: &mut Container, pending: &[String], tick: u64, is_streaming: bool) {
    let theme = Theme::default();

    if pending.is_empty() {
        return;
    }

    container.push(Spacer::new(1));

    if is_streaming {
        // Active streaming - show spinner and "Responding..."
        let spinner = spinner_frame(tick);
        container.push(Text::styled(
            format!("  {} Responding...", spinner),
            theme.accent,
        ));
    } else {
        // Waiting for response - show different indicator
        let spinner = spinner_frame(tick);
        container.push(Text::styled(
            format!("  {} Waiting for response...", spinner),
            theme.warning,
        ));
    }

    // Show streaming content
    for msg in pending {
        container.push(Text::styled(format!("    {}", msg), theme.dim));
    }
}

/// Render status messages.
fn render_status(container: &mut Container, status: &[String]) {
    let theme = Theme::default();

    if status.is_empty() {
        return;
    }

    container.push(Spacer::new(1));
    for msg in status {
        container.push(Text::styled(format!("  → {}", msg), theme.info));
    }
}

/// Render todo list.
fn render_todo(container: &mut Container, todos: &[TodoItem]) {
    let theme = Theme::default();

    if todos.is_empty() {
        return;
    }

    container.push(Spacer::new(1));
    container.push(Text::styled("  Tasks", theme.accent));

    for todo in todos {
        let check = if todo.done { "✓" } else { "○" };
        let style = if todo.done {
            theme.dim
        } else {
            Style::default()
        };
        container.push(Text::styled(
            format!("    {} {}", check, todo.content),
            style,
        ));
    }
}

/// Render btw notes.
fn render_btw(container: &mut Container, notes: &[String]) {
    let theme = Theme::default();

    if notes.is_empty() {
        return;
    }

    container.push(Spacer::new(1));
    container.push(Text::styled("  By the way", theme.warning));

    for note in notes {
        container.push(Text::styled(format!("    • {}", note), theme.dim));
    }
}

/// Render input box at bottom with dynamic border.
fn render_input(container: &mut Container, app: &App) {
    let theme = Theme::default();

    let border_style = match app.mode {
        AppMode::Normal => Style::new().fg(Colors::GRAY),
        AppMode::Input => Style::new().fg(Colors::BRIGHT_BLUE),
        AppMode::Processing => Style::new().fg(Colors::BRIGHT_YELLOW),
    };

    let title = match app.mode {
        AppMode::Input => " Input ",
        AppMode::Processing => " Working ",
        AppMode::Normal => " Input ",
    };

    let mut box_widget = BoxWidget::new()
        .with_title(title)
        .with_border_style(border_style);

    let content = if app.input.is_empty() {
        match app.mode {
            AppMode::Processing => Text::styled("Waiting for AI response...", theme.dim),
            AppMode::Input => Text::styled("Type your message...", theme.dim),
            AppMode::Normal => Text::styled("Type to start...", theme.dim),
        }
    } else {
        Text::new(&app.input)
    };

    box_widget.add_child(Box::new(content));
    container.push(box_widget);
}

/// Render status line at the very bottom (oh-my-pi style).
fn render_status_line(container: &mut Container, app: &App) {
    let theme = Theme::default();

    let mut left_segments = Vec::new();
    let mut right_segments = Vec::new();

    // Left: mode, model, git branch, workspace, token usage
    let (mode_text, mode_style) = match app.mode {
        AppMode::Normal => ("NORMAL", Style::new().fg(Colors::BRIGHT_GREEN)),
        AppMode::Input => ("INPUT", Style::new().fg(Colors::BRIGHT_BLUE)),
        AppMode::Processing => ("BUSY", Style::new().fg(Colors::BRIGHT_YELLOW).bold()),
    };
    left_segments.push(mode_style.apply(mode_text));
    left_segments.push(" ┆ ".to_string());
    left_segments.push(theme.info.apply(&app.model));

    if let Some(branch) = &app.git_branch {
        left_segments.push(" ┆ ".to_string());
        left_segments.push(theme.dim.apply(&format!("git:{}", branch)));
        if app.git_staged > 0 || app.git_unstaged > 0 || app.git_untracked > 0 {
            let mut parts = Vec::new();
            if app.git_staged > 0 {
                parts.push(format!("+{}", app.git_staged));
            }
            if app.git_unstaged > 0 {
                parts.push(format!("!{}", app.git_unstaged));
            }
            if app.git_untracked > 0 {
                parts.push(format!("?{}", app.git_untracked));
            }
            left_segments.push(theme.warning.apply(&format!(" ({})", parts.join(" "))));
        }
    }

    let workspace_name = app
        .workspace
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("workspace");
    left_segments.push(" ┆ ".to_string());
    left_segments.push(theme.dim.apply(workspace_name));

    if app.tokens_input > 0 || app.tokens_output > 0 {
        let total_input = app.tokens_input + app.tokens_cache_read;
        if total_input > 0 || app.tokens_output > 0 {
            left_segments.push(" ┆ ".to_string());
            let usage = format!("in:{} out:{}", total_input, app.tokens_output);
            left_segments.push(theme.dim.apply(&usage));
        }
        // Context window percentage (oh-my-pi style)
        if app.context_window > 0 {
            let total_tokens = total_input + app.tokens_output;
            let pct = (total_tokens as f64 / app.context_window as f64 * 100.0) as u32;
            if pct > 0 {
                left_segments.push(" ┆ ".to_string());
                let ctx_str = format!("ctx:{}%", pct);
                // Color based on usage level
                let ctx_style = if pct >= 90 {
                    theme.error
                } else if pct >= 70 {
                    theme.warning
                } else if pct >= 50 {
                    Style::new().fg(Colors::BRIGHT_MAGENTA)
                } else {
                    theme.dim
                };
                left_segments.push(ctx_style.apply(&ctx_str));
            }
        }
    }

    // Iteration progress
    if app.iterations_max > 0 {
        left_segments.push(" ┆ ".to_string());
        let iter_text = format!("iter:{}/{}", app.iterations_used, app.iterations_max);
        let iter_style = if app.iterations_used >= app.iterations_max {
            theme.error
        } else if app.iterations_used as f64 / app.iterations_max as f64 > 0.8 {
            theme.warning
        } else {
            theme.dim
        };
        left_segments.push(iter_style.apply(&iter_text));
    }

    // Right: help
    right_segments.push("Ctrl+D: quit".to_string());

    let left = left_segments.join("");
    let right = right_segments.join(" ");
    let status = format!(" {} {}", left, right);
    container.push(Text::styled(&status, theme.dim));
}

/// Draw the complete UI.
pub fn draw(tui: &mut Tui, app: &App) -> serana_core::Result<()> {
    let root = tui.root_mut();
    root.clear();

    // 1. Welcome or messages (oh-my-pi order)
    if app.show_welcome && app.messages.is_empty() {
        render_welcome(root, &app.version, &app.model, &app.provider);
    } else {
        render_messages(root, &app.messages);
    }

    // 2. Pending messages (streaming) - with animated indicator
    let is_streaming = !app.pending_messages.is_empty() && app.mode == AppMode::Processing;
    render_pending(root, &app.pending_messages, app.tick_count, is_streaming);
    // 3. Status messages
    render_status(root, &app.status_messages);
    // 4. Todo list
    render_todo(root, &app.todo_items);
    // 5. BTW notes
    render_btw(root, &app.btw_notes);
    // 6. Spacer
    root.push(Spacer::new(1));
    // 7. Input area
    render_input(root, app);
    // 8. Status line at bottom
    render_status_line(root, app);

    Ok(())
}
