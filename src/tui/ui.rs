//! UI rendering — vertical flow like oh-my-pi.

use super::app::{App, AppMode, ChatMessage, MessageRole, TodoItem, ToolCallStatus};
use super::component::Container;
use super::components::{BoxWidget, Markdown, Spacer, Text};
use super::style::{Colors, Style};
use super::tui::Tui;
use super::tool_execution::{ToolExecution, ToolState};

/// Theme colors matching oh-my-pi dark theme.
pub struct Theme {
    pub accent: Style,
    pub success: Style,
    pub warning: Style,
    pub error: Style,
    pub dim: Style,
    pub info: Style,
    pub user_fg: Style,
    pub agent_fg: Style,
    pub thinking: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: Style::new().fg(Colors::BRIGHT_BLUE).bold(),
            success: Style::new().fg(Colors::BRIGHT_GREEN).bold(),
            warning: Style::new().fg(Colors::BRIGHT_YELLOW).bold(),
            error: Style::new().fg(Colors::BRIGHT_RED).bold(),
            dim: Style::new().fg(Colors::GRAY).dim(),
            info: Style::new().fg(Colors::BRIGHT_CYAN),
            user_fg: Style::new().fg(Colors::BRIGHT_GREEN),
            agent_fg: Style::new().fg(Colors::BRIGHT_BLUE),
            thinking: Style::new().fg(Colors::GRAY).italic(),
        }
    }
}

/// Render welcome screen with true two-column layout (oh-my-pi style).
fn render_welcome(container: &mut Container, version: &str, model: &str) {
    let theme = Theme::default();
    let width = 80;

    container.push(Spacer::new(1));

    let left_col_width = (width as f64 * 0.5) as usize;
    let right_col_width = width - left_col_width - 3;

    // Left column: centered logo and welcome
    // Left column: centered logo and welcome
    let left_lines: Vec<String> = vec![
        "".to_string(),
        center_text(&theme.accent.apply("Welcome back!"), left_col_width),
        "".to_string(),
        center_text(&theme.accent.apply("███████╗███████╗██╗  ██╗ ██████╗ ██████╗ ██████╗ ███████╗"), left_col_width),
        center_text(&theme.accent.apply("██╔════╝██╔════╝██║  ██║██╔════╝██╔═══██╗██╔══██╗██╔════╝"), left_col_width),
        center_text(&theme.accent.apply("███████╗█████╗  ███████║██║     ██║   ██║██║  ██║█████╗  "), left_col_width),
        center_text(&theme.accent.apply("╚════██║██╔══╝  ██╔══██║██║     ██║   ██║██║  ██║██╔══╝  "), left_col_width),
        center_text(&theme.accent.apply("███████║███████╗██║  ██║╚██████╗╚██████╔╝██████╔╝███████╗"), left_col_width),
        center_text(&theme.accent.apply("╚══════╝╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝"), left_col_width),
        "".to_string(),
        center_text(&theme.dim.apply(model), left_col_width),
        "".to_string(),
    ];

    // Right column: tips, LSP, sessions
    let right_lines: Vec<String> = vec![
        format!(" {}", theme.accent.apply("Tips")),
        format!(" {} {}", theme.dim.apply("?"), theme.dim.apply("for keyboard shortcuts")),
        format!(" {} {}", theme.dim.apply("/"), theme.dim.apply("for commands")),
        format!(" {} {}", theme.dim.apply("!"), theme.dim.apply("to run bash")),
        format!(" {} {}", theme.dim.apply("$"), theme.dim.apply("to run python")),
        format!(" {}", theme.dim.apply(&"─".repeat(right_col_width.saturating_sub(2)))),
        format!(" {}", theme.accent.apply("LSP Servers")),
        format!("  {}", theme.dim.apply("○ No LSP servers")),
        format!(" {}", theme.dim.apply(&"─".repeat(right_col_width.saturating_sub(2)))),
        format!(" {}", theme.accent.apply("Recent sessions")),
        format!("  {}", theme.dim.apply("• No recent sessions")),
    ];

    let max_rows = left_lines.len().max(right_lines.len());
    let mut padded_left = Vec::new();
    let mut padded_right = Vec::new();
    for i in 0..max_rows {
        let left = left_lines.get(i).cloned().unwrap_or_default();
        let right = right_lines.get(i).cloned().unwrap_or_default();
        padded_left.push(pad_to_width(&left, left_col_width));
        padded_right.push(pad_to_width(&right, right_col_width));
    }

    let h = theme.dim.apply("─");
    let v = theme.dim.apply("│");
    let tl = theme.dim.apply("┌");
    let tr = theme.dim.apply("┐");
    let bl = theme.dim.apply("└");
    let br = theme.dim.apply("┘");
    let tee = theme.dim.apply("┬");
    let tee_bottom = theme.dim.apply("┴");

    let mut lines = Vec::new();
    let title = format!(" Serana v{} ", version);
    let title_styled = theme.dim.apply(&title);
    let title_len = title.chars().count();
    let top_line = format!("{}{}{}{}{}{}",
        tl,
        title_styled,
        h.repeat(left_col_width.saturating_sub(title_len + 1)),
        tee,
        h.repeat(right_col_width),
        tr
    );
    lines.push(top_line);
    for i in 0..max_rows {
        lines.push(format!("{}{}{}{}{}", v, padded_left[i], v, padded_right[i], v));
    }

    let bottom_line = format!("{}{}{}{}{}", bl, h.repeat(left_col_width), tee_bottom, h.repeat(right_col_width), br);
    lines.push(bottom_line);

    for line in lines {
        container.push(Text::styled(line, Style::default()));
    }

    container.push(Spacer::new(1));
}

fn center_text(text: &str, width: usize) -> String {
    let vis_len = strip_ansi_escapes::strip_str(text).chars().count();
    if vis_len >= width {
        return text.to_string();
    }
    let left_pad = (width - vis_len) / 2;
    let right_pad = width - vis_len - left_pad;
    format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
}

fn pad_to_width(text: &str, width: usize) -> String {
    let vis_len = strip_ansi_escapes::strip_str(text).chars().count();
    if vis_len >= width {
        return text.to_string();
    }
    format!("{}{}", text, " ".repeat(width - vis_len))
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

/// Render pending/streaming messages.
fn render_pending(container: &mut Container, pending: &[String]) {
    let theme = Theme::default();
    
    if pending.is_empty() {
        return;
    }
    
    container.push(Spacer::new(1));
    container.push(Text::styled("  ◐ Working...", theme.warning));
    
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
        let style = if todo.done { theme.dim } else { Style::default() };
        container.push(Text::styled(format!("    {} {}", check, todo.content), style));
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
            AppMode::Processing => Text::styled("Processing...", theme.dim),
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
    let mode_text = match app.mode {
        AppMode::Normal => "NORMAL",
        AppMode::Input => "INPUT",
        AppMode::Processing => "BUSY",
    };
    left_segments.push(Style::new().fg(Colors::BRIGHT_GREEN).apply(mode_text));
    left_segments.push(" ┆ ".to_string());
    left_segments.push(theme.info.apply(&app.model));

    if let Some(branch) = &app.git_branch {
        left_segments.push(" ┆ ".to_string());
        left_segments.push(theme.dim.apply(&format!("git:{}", branch)));
        if app.git_staged > 0 || app.git_unstaged > 0 || app.git_untracked > 0 {
            let mut parts = Vec::new();
            if app.git_staged > 0 { parts.push(format!("+{}", app.git_staged)); }
            if app.git_unstaged > 0 { parts.push(format!("!{}", app.git_unstaged)); }
            if app.git_untracked > 0 { parts.push(format!("?{}", app.git_untracked)); }
            left_segments.push(theme.warning.apply(&format!(" ({})", parts.join(" "))));
        }
    }

    let workspace_name = app.workspace.file_name()
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
    }

    // Right: help
    right_segments.push("Ctrl+D: quit".to_string());

    let left = left_segments.join("");
    let right = right_segments.join(" ");
    let status = format!(" {} {}", left, right);
    container.push(Text::styled(&status, theme.dim));
}

/// Draw the complete UI.
pub fn draw(tui: &mut Tui, app: &App) -> crate::Result<()> {
    let root = tui.root_mut();
    root.clear();

    // 1. Welcome or messages (oh-my-pi order)
    if app.show_welcome && app.messages.is_empty() {
        render_welcome(root, &app.version, &app.model);
    } else {
        render_messages(root, &app.messages);
    }

    // 2. Pending messages (streaming)
    render_pending(root, &app.pending_messages);

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
