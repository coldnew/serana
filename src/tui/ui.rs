//! UI rendering — vertical flow like oh-my-pi.

use super::app::{App, AppMode, ChatMessage, MessageRole};
use super::component::Container;
use super::components::{BoxWidget, Spacer, Text};
use super::style::{Color, Colors, Style};
use super::tui::Tui;

/// Theme colors matching oh-my-pi dark theme.
pub struct Theme {
    pub accent: Style,
    pub success: Style,
    pub warning: Style,
    pub error: Style,
    pub dim: Style,
    pub info: Style,
    pub user_bg: Style,
    pub agent_bg: Style,
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
            user_bg: Style::new().bg(Color::Rgb(35, 35, 45)),
            agent_bg: Style::new().bg(Color::Rgb(25, 25, 35)),
        }
    }
}

/// Render welcome screen with ASCII art logo.
fn render_welcome(container: &mut Container, model: &str) {
    let theme = Theme::default();

    // ASCII logo
    container.push(Spacer::new(1));
    container.push(Text::styled(
        "  ____                  _       ",
        theme.accent,
    ));
    container.push(Text::styled(
        " / ___|  ___ _ ____   _| | ___  ",
        theme.accent,
    ));
    container.push(Text::styled(
        " \\___ \\ / _ \\ '__\\ \\ / / |/ _ \\ ",
        theme.accent,
    ));
    container.push(Text::styled(
        "  ___) |  __/ |   \\ V /| |  __/ ",
        theme.accent,
    ));
    container.push(Text::styled(
        " |____/ \\___|_|    \\_/ |_|\\___| ",
        theme.accent,
    ));
    container.push(Spacer::new(2));

    container.push(Text::styled("Your AI coding companion", theme.dim));
    container.push(Spacer::new(1));

    let model_info = format!("Model: {}", model);
    container.push(Text::styled(&model_info, theme.info));
    container.push(Spacer::new(2));

    container.push(Text::styled(
        "Type a message and press Enter to start",
        theme.dim,
    ));
}

/// Render chat messages with oh-my-pi styling.
fn render_messages(container: &mut Container, messages: &[ChatMessage]) {
    let theme = Theme::default();

    for msg in messages {
        match msg.role {
            MessageRole::User => {
                container.push(Spacer::new(1));
                container.push(Text::styled("You", theme.success));
                for line in msg.content.lines() {
                    container.push(Text::new(line));
                }
            }
            MessageRole::Agent => {
                container.push(Spacer::new(1));
                container.push(Text::styled("Serana", theme.accent));
                for line in msg.content.lines() {
                    container.push(Text::new(line));
                }
                // Inline tool calls
                if !msg.tool_calls.is_empty() {
                    container.push(Spacer::new(1));
                    for tool in &msg.tool_calls {
                        container.push(Text::styled(
                            &format!("⚡ {}", tool),
                            theme.warning,
                        ));
                    }
                }
            }
            MessageRole::System => {
                container.push(Spacer::new(1));
                container.push(Text::styled(
                    &format!("System: {}", msg.content),
                    theme.warning,
                ));
            }
        }
    }
}

/// Render input box at bottom.
fn render_input(container: &mut Container, app: &App) {
    let theme = Theme::default();

    let border_style = match app.mode {
        AppMode::Normal => theme.dim,
        AppMode::Input => theme.accent,
        AppMode::Processing => theme.warning,
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
            _ => Text::styled("Type your message...", theme.dim),
        }
    } else {
        Text::new(&app.input)
    };

    box_widget.add_child(Box::new(content));
    container.push(box_widget);
}

/// Render status line at the very bottom.
fn render_status_line(container: &mut Container, model: &str, mode: AppMode) {
    let theme = Theme::default();

    let mode_text = match mode {
        AppMode::Normal => "NORMAL",
        AppMode::Input => "INPUT",
        AppMode::Processing => "BUSY",
    };

    let status = format!(" {} │ {} │ Ctrl+D: exit ", mode_text, model);
    container.push(Text::styled(&status, theme.dim));
}

/// Draw the complete UI.
pub fn draw(tui: &mut Tui, app: &App) -> crate::Result<()> {
    let root = tui.root_mut();
    root.clear();

    // Welcome or messages
    if app.messages.is_empty() {
        render_welcome(root, &app.model);
    } else {
        render_messages(root, &app.messages);
    }

    // Spacer between content and input
    root.push(Spacer::new(1));

    // Input area
    render_input(root, app);

    // Status line
    render_status_line(root, &app.model, app.mode);

    Ok(())
}
