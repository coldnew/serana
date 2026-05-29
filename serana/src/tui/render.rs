//! Declarative UI builder using display-protocol-jsx.
//!
//! Replaces the ratatui-based ui.rs with a UiNode tree built via jsx! macro.
//! The tree is painted to ScreenBuffer by the Tui layer.

use display_protocol::*;
use display_protocol_jsx::jsx;

use super::app::{App, AppMode, MessageRole, TodoStatus};
use super::theme::Theme;

/// Build the full UI tree for the current app state.
pub fn build_ui(app: &App, width: u16, height: u16) -> UiNode {
    let theme = Theme::default();
    let showing_welcome = app.show_welcome && app.messages.is_empty();

    let header_h: u16 = if showing_welcome { 0 } else { 3 };
    let input_h: u16 = 3;
    let _content_h = height.saturating_sub(header_h + input_h);

    let mut children = Vec::new();

    if header_h > 0 {
        children.push(build_header(app, width, &theme));
    }

    if showing_welcome {
        children.push(build_welcome(app, width, &theme));
    } else {
        children.push(build_messages(app, width, &theme));
    }

    children.push(build_input(app, width, &theme));

    if let Some(ref dialog) = app.active_dialog {
        children.push(build_dialog(dialog, width, height));
    }

    let mut root = build_column(children);

    if let Some(ref ac) = app.autocomplete {
        let ac_node = build_autocomplete(ac, width, height, &theme);
        root = build_column(vec![root, ac_node]);
    }

    root
}

fn build_column(children: Vec<UiNode>) -> UiNode {
    UiNode::Column(FlexNode {
        children,
        style: Style::default(),
        gap: 0,
        align: Align::Start,
        justify: Justify::Start,
        padding: Padding::default(),
        width: None,
        height: None,
        flex_grow: 0.0,
        flex_shrink: 1.0,
    })
}

fn build_header(app: &App, _width: u16, theme: &Theme) -> UiNode {
    let title = format!(" Serana v{} ", app.version);
    jsx! {
        <Box title={title.as_str()} border style={theme.border} height={3}>
            <Text style={theme.muted}>{title.as_str()}</Text>
        </Box>
    }
}

fn build_welcome(app: &App, width: u16, theme: &Theme) -> UiNode {
    let mut children = Vec::new();

    children.push(jsx! { <Text>{""}</Text> });

    let welcome = center_text("Welcome back!", width as usize);
    children.push(jsx! {
        <Text style={theme.accent}>{welcome.as_str()}</Text>
    });

    children.push(jsx! { <Text>{""}</Text> });

    let logo = [
        "▀████████████▀",
        " ╘███    ███  ",
        "  ███    ███  ",
        "  ███    ███  ",
        " ▄███▄  ▄███▄ ",
    ];
    let logo_style = Style::new().fg(Color::new(60, 200, 255));
    for line in &logo {
        let centered = center_text(line, width as usize);
        children.push(jsx! {
            <Text style={logo_style}>{centered.as_str()}</Text>
        });
    }

    children.push(jsx! { <Text>{""}</Text> });

    let model_line = center_text(&app.model, width as usize);
    children.push(jsx! {
        <Text style={theme.muted}>{model_line.as_str()}</Text>
    });

    let provider_line = center_text(&app.provider, width as usize);
    children.push(jsx! {
        <Text style={theme.dim}>{provider_line.as_str()}</Text>
    });

    children.push(jsx! { <Text>{""}</Text> });
    children.push(jsx! { <Text style={theme.accent}>{" Tips"}</Text> });
    children.push(jsx! { <Text style={theme.dim}>{" ? for keyboard shortcuts"}</Text> });
    children.push(jsx! { <Text style={theme.dim}>{" # for prompt actions"}</Text> });
    children.push(jsx! { <Text style={theme.dim}>{" / for commands"}</Text> });
    children.push(jsx! { <Text style={theme.dim}>{" ! to run bash"}</Text> });

    build_column(children)
}

fn build_messages(app: &App, _width: u16, theme: &Theme) -> UiNode {
    let mut children = Vec::new();

    for pending in &app.pending_messages {
        children.push(jsx! { <Text>{""}</Text> });
        let text = format!("  ◌ {}", pending);
        children.push(jsx! { <Text style={theme.dim}>{text.as_str()}</Text> });
    }

    for msg in &app.messages {
        children.push(jsx! { <Text>{""}</Text> });
        match msg.role {
            MessageRole::User => {
                for line in msg.content.lines() {
                    let text = format!("  {}", line);
                    let user_style = Style::new()
                        .fg(Color::new(156, 163, 176))
                        .bg(Color::new(15, 18, 22));
                    children.push(jsx! {
                        <Text style={user_style}>{text.as_str()}</Text>
                    });
                }
            }
            MessageRole::Agent => {
                if let Some(ref thinking) = msg.thinking {
                    for line in thinking.lines() {
                        let text = format!("  · {}", line);
                        children.push(jsx! {
                            <Text style={theme.thinking}>{text.as_str()}</Text>
                        });
                    }
                    children.push(jsx! { <Text>{""}</Text> });
                }
                for line in msg.content.lines() {
                    let text = format!("    {}", line);
                    children.push(jsx! {
                        <Text style={theme.agent_fg}>{text.as_str()}</Text>
                    });
                }
                for tool in &msg.tool_calls {
                    let tool_text = format!("  ⚡ {}", tool.name);
                    children.push(jsx! {
                        <Text style={theme.dim}>{tool_text.as_str()}</Text>
                    });
                }
            }
            MessageRole::System => {
                let text = format!("  ⚠ {}", msg.content);
                children.push(jsx! {
                    <Text style={theme.warning}>{text.as_str()}</Text>
                });
            }
        }
    }

    for status_msg in &app.status_messages {
        children.push(jsx! { <Text>{""}</Text> });
        let text = format!("  {}", status_msg);
        children.push(jsx! { <Text style={theme.info}>{text.as_str()}</Text> });
    }

    if !app.todo_items.is_empty() {
        children.push(jsx! { <Text>{""}</Text> });
        children.push(jsx! { <Text style={theme.accent}>{" Todos"}</Text> });
        for item in &app.todo_items {
            let (marker, style) = match item.status {
                TodoStatus::Pending => (" ", theme.dim),
                TodoStatus::InProgress => ("/", theme.accent),
                TodoStatus::Done => ("x", theme.muted),
                TodoStatus::Abandoned => ("-", theme.muted),
            };
            let text = format!("  [{}] {}", marker, item.content);
            children.push(jsx! { <Text style={style}>{text.as_str()}</Text> });
        }
    }

    if !app.btw_notes.is_empty() {
        children.push(jsx! { <Text>{""}</Text> });
        for note in &app.btw_notes {
            let text = format!("  💡 {}", note);
            children.push(jsx! { <Text style={theme.info}>{text.as_str()}</Text> });
        }
    }

    build_column(children)
}

fn build_input(app: &App, width: u16, theme: &Theme) -> UiNode {
    let border_style = match app.mode {
        AppMode::Normal => Style::new().fg(Color::new(156, 163, 176)),
        AppMode::Input => Style::new().fg(Color::new(0, 180, 255)),
        AppMode::Processing => Style::new().fg(Color::new(255, 71, 87)),
    };

    let placeholder = match app.mode {
        AppMode::Normal => "Type your message...",
        AppMode::Input => "Type your message...",
        AppMode::Processing => "Waiting for AI response...",
    };

    let content = if app.editor.is_empty() {
        placeholder.to_string()
    } else {
        app.editor.content()
    };

    let status = build_status_text(app, width);

    jsx! {
        <Box border style={border_style} height={3} title={status.as_str()}>
            <Text style={theme.muted}>{content.as_str()}</Text>
        </Box>
    }
}

fn build_autocomplete(
    ac: &super::app::AutocompleteState,
    _width: u16,
    height: u16,
    _theme: &Theme,
) -> UiNode {
    let popup_h = (ac.items.len() as u16 + 2).min(8).min(height.saturating_sub(6));
    let popup_w = 40u16;

    let mut children = Vec::new();
    for (i, item) in ac.items.iter().enumerate().take(6) {
        let selected = i == ac.selected;
        let style = if selected {
            Style::new().fg(Color::new(0, 180, 255)).bold()
        } else {
            Style::default()
        };
        let prefix = if selected { "› " } else { "  " };
        let text = format!("{}{}", prefix, item.value);
        children.push(jsx! { <Text style={style}>{text.as_str()}</Text> });
    }

    jsx! {
        <Box border style={Style::new().fg(Color::new(0, 180, 255))} title=" Commands " width={popup_w} height={popup_h}>
            {build_column(children)}
        </Box>
    }
}

fn build_dialog(dialog: &super::dialog::Dialog, width: u16, height: u16) -> UiNode {
    let aquamarine = Color::new(0, 180, 255);
    let popup_w = width.min(60).min(width * 2 / 3);
    let popup_h = height.min(20).min(height * 2 / 3);

    let filtered = dialog.filtered_items();
    let mut children = Vec::new();

    if dialog.filter.is_empty() {
        children.push(jsx! {
            <Text style={Style::new().fg(Color::new(156, 163, 176))}>{"Type to filter..."}</Text>
        });
    } else {
        let filter_text = format!("{}▏", dialog.filter);
        children.push(jsx! {
            <Text style={Style::new().fg(aquamarine)}>{filter_text.as_str()}</Text>
        });
    }

    for (orig_idx, item) in &filtered {
        let selected = dialog.selected == Some(*orig_idx);
        let prefix = if selected { "› " } else { "  " };
        let style = if selected {
            Style::new().fg(aquamarine).bold()
        } else {
            Style::default()
        };
        let text = format!("{}{} — {}", prefix, item.label, item.description);
        children.push(jsx! { <Text style={style}>{text.as_str()}</Text> });
    }

    let help = "↑↓: navigate  Enter: select  Esc: cancel  Type: filter";
    children.push(jsx! {
        <Text style={Style::new().fg(Color::new(107, 114, 128))}>{help}</Text>
    });

    let title = format!(" {} ", dialog.title);
    jsx! {
        <Box border style={Style::new().fg(aquamarine)} title={title.as_str()} width={popup_w} height={popup_h}>
            {build_column(children)}
        </Box>
    }
}

fn build_status_text(app: &App, _width: u16) -> String {
    let mut parts = Vec::new();
    parts.push(format!(" {}", app.mode_label()));
    if let Some(ref branch) = app.git_branch {
        parts.push(format!(" {}", branch));
    }
    parts.push(format!(" {}", app.model));
    parts.join(" │")
}

fn center_text(text: &str, width: usize) -> String {
    let text_len = text.chars().count();
    if text_len >= width {
        return text.to_string();
    }
    let padding = (width - text_len) / 2;
    format!("{}{}", " ".repeat(padding), text)
}
