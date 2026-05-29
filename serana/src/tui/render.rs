//! Declarative UI builder using display-protocol-jsx.
//!
//! Replaces the ratatui-based ui.rs with a UiNode tree built via jsx! macro.
//! The tree is painted to ScreenBuffer by the Tui layer.

use display_protocol::*;
use display_protocol_jsx::jsx;

use super::app::{App, AppMode, MessageRole, TodoStatus, ToolCallStatus};
use super::markdown;
use super::theme::Theme;

/// Build the full UI tree for the current app state.
pub fn build_ui(app: &App, width: u16, height: u16) -> UiNode {
    let theme = Theme::default();
    let showing_welcome = app.show_welcome && app.messages.is_empty();
    let header_h: u16 = if showing_welcome { 0 } else { 3 };
    let status_h: u16 = 1;
    let input_h: u16 = 3;
    let _content_h = height.saturating_sub(header_h + status_h + input_h);

    let mut children = Vec::new();

    if header_h > 0 {
        children.push(build_header(app, width, &theme));
    }

    if showing_welcome {
        children.push(build_welcome(app, width, &theme));
    } else {
        children.push(build_messages(app, width, &theme));
    }

    children.push(build_status_bar(app, width, &theme));

    if app.mode == AppMode::Processing {
        children.push(build_processing_indicator(app, width, &theme));
    }

    children.push(build_input(app, width, &theme));

    if app.show_help {
        children.push(build_help_overlay(width, height, &theme));
    } else if let Some(ref dialog) = app.active_dialog {
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

fn build_messages(app: &App, width: u16, theme: &Theme) -> UiNode {
    let md_theme = markdown::MarkdownTheme::default();
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
                let md_width = width.saturating_sub(4) as usize;
                let md_lines = markdown::render_markdown(&msg.content, &md_theme, md_width);
                let prefix = "  ";
                let user_style = Style::new()
                    .fg(Color::new(156, 163, 176))
                    .bg(Color::new(15, 18, 22));
                for md_line in &md_lines {
                    let mut text = String::from(prefix);
                    for span in &md_line.spans {
                        text.push_str(&span.text);
                    }
                    children.push(jsx! {
                        <Text style={user_style}>{text.as_str()}</Text>
                    });
                }
            }
            MessageRole::Agent => {
                if let Some(ref thinking) = msg.thinking {
                    let md_width = width.saturating_sub(4) as usize;
                    let md_lines = markdown::render_markdown(thinking, &md_theme, md_width);
                    for md_line in &md_lines {
                        children.push(styled_line_to_row(md_line, "  ", &theme.thinking));
                    }
                    children.push(jsx! { <Text>{""}</Text> });
                }
                let md_width = width.saturating_sub(4) as usize;
                let md_lines = markdown::render_markdown(&msg.content, &md_theme, md_width);
                for md_line in &md_lines {
                    children.push(styled_line_to_row(md_line, "    ", &theme.agent_fg));
                }
                for tool in &msg.tool_calls {
                    let tool_lines = build_tool_call(tool, width, &theme);
                    children.extend(tool_lines);
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

/// Convert a StyledLine into a Row of Span nodes with a text prefix.
fn styled_line_to_row(line: &StyledLine, prefix: &str, base_style: &Style) -> UiNode {
    let mut spans = vec![SpanNode {
        content: prefix.to_string(),
        style: *base_style,
    }];
    for span in &line.spans {
        let merged = Style::default()
            .merge(base_style)
            .merge(&span.style);
        spans.push(SpanNode {
            content: span.text.clone(),
            style: merged,
        });
    }
    UiNode::Row(FlexNode {
        children: spans.into_iter().map(|s| UiNode::Span(s)).collect(),
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

fn build_status_bar(app: &App, _width: u16, theme: &Theme) -> UiNode {
    let mode_label = app.mode_label();
    let mode_style = match app.mode {
        AppMode::Normal => Style::new().fg(Color::new(0, 255, 136)).bold(),
        AppMode::Input => Style::new().fg(Color::new(0, 180, 255)).bold(),
        AppMode::Processing => Style::new().fg(Color::new(255, 71, 87)).bold(),
    };
    let bg = Style::new().bg(Color::new(31, 37, 45));

    let mut left_items = vec![jsx! {
        <Span style={mode_style.merge(&bg)}>{mode_label}</Span>
    }];

    if let Some(ref branch) = app.git_branch {
        let branch_text = format!(" {}", branch);
        left_items.push(jsx! {
            <Span style={theme.dim.merge(&bg)}>{branch_text.as_str()}</Span>
        });
    }

    let mut right_items = Vec::new();

    // Model
    let model_text = format!("{} ", app.model);
    right_items.push(jsx! {
        <Span style={theme.muted.merge(&bg)}>{model_text.as_str()}</Span>
    });

    // Iterations
    if app.iterations_max > 0 {
        let iter_text = format!("{}/{} ", app.iterations_used, app.iterations_max);
        right_items.push(jsx! {
            <Span style={theme.dim.merge(&bg)}>{iter_text.as_str()}</Span>
        });
    }

    // Tokens
    if app.tokens_input > 0 {
        let total = app.tokens_input + app.tokens_output;
        let token_text = format!("{}k ", total / 1000);
        right_items.push(jsx! {
            <Span style={theme.dim.merge(&bg)}>{token_text.as_str()}</Span>
        });
    }

    let status_node = UiNode::StatusBar(StatusBarNode {
        left: left_items,
        right: right_items,
        style: bg,
    });
    status_node
}

fn build_processing_indicator(app: &App, width: u16, theme: &Theme) -> UiNode {
    let frames = ["◌", "◜", "◠", "◝", "◞", "◡", "◟", "●"];
    let frame = frames[(app.tick_count as usize / 4) % frames.len()];
    let elapsed = app.session_elapsed();

    let mut items = Vec::new();

    // Blank line
    items.push(jsx! { <Text>{""}</Text> });

    // Spinner line
    let indicator = format!("  {} Working  {}  Esc to interrupt", frame, elapsed);
    items.push(jsx! {
        <Text style={Style::new().fg(Color::new(212, 192, 144))}>{indicator.as_str()}</Text>
    });

    // Streaming content preview
    if !app.pending_messages.is_empty() {
        let preview: String = app.pending_messages.iter()
            .flat_map(|m| m.lines())
            .rev()
            .take(3)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        let display_width = width.saturating_sub(6) as usize;
        for line in preview.lines() {
            let truncated = if line.len() > display_width {
                format!("{}…", &line[..display_width])
            } else {
                line.to_string()
            };
            items.push(jsx! {
                <Text style={theme.dim}>{format!("    {}", truncated).as_str()}</Text>
            });
        }
    }

    build_column(items)
}

fn build_tool_call(tool: &super::app::ToolCall, width: u16, theme: &Theme) -> Vec<UiNode> {
    let mut children = Vec::new();
    let indent = "  ";

    // Status icon
    let (icon, header_style) = match tool.status {
        ToolCallStatus::Pending => ("◌", theme.dim),
        ToolCallStatus::Running => ("⚡", Style::new().fg(Color::new(212, 192, 144))),
        ToolCallStatus::Success => ("✓", Style::new().fg(Color::new(0, 255, 136))),
        ToolCallStatus::Error => ("✗", Style::new().fg(Color::new(255, 71, 87))),
    };

    // Header: tool name + args
    let header = format!("{}{} {}", indent, icon, tool.name);
    children.push(jsx! {
        <Text style={header_style}>{header.as_str()}</Text>
    });

    // Arguments (truncated)
    if !tool.args.is_empty() {
        let max_len = width.saturating_sub(6) as usize;
        let display_args = if tool.args.len() > max_len {
            format!("{}{}", &tool.args[..max_len], "…")
        } else {
            tool.args.clone()
        };
        let args_text = format!("{}  {}", indent, display_args);
        children.push(jsx! {
            <Text style={theme.dim}>{args_text.as_str()}</Text>
        });
    }

    // Diff preview for edit operations
    if let Some((ref _path, ref diff_text)) = tool.diff_preview {
        let md_theme = markdown::MarkdownTheme::default();
        let diff_width = width.saturating_sub(6) as usize;
        let md_lines = markdown::render_markdown(diff_text, &md_theme, diff_width);
        for md_line in md_lines.iter().take(8) {
            children.push(styled_line_to_row(md_line, indent, &theme.dim));
        }
    }

    // Result preview (truncated)
    if let Some(ref result) = tool.result {
        let preview_lines: Vec<&str> = result.lines().take(5).collect();
        for line in &preview_lines {
            let max_len = width.saturating_sub(6) as usize;
            let display = if line.len() > max_len {
                format!("{}{}", &line[..max_len], "…")
            } else {
                line.to_string()
            };
            let result_text = format!("{}  {}", indent, display);
            let style = match tool.status {
                ToolCallStatus::Error => Style::new().fg(Color::new(255, 71, 87)),
                _ => theme.dim,
            };
            children.push(jsx! {
                <Text style={style}>{result_text.as_str()}</Text>
            });
        }
    }

    children
}

fn build_input(app: &App, _width: u16, theme: &Theme) -> UiNode {
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

    jsx! {
        <Box border style={border_style} height={3}>
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

fn build_help_overlay(width: u16, height: u16, theme: &Theme) -> UiNode {
    let popup_w = width.min(52).min(width * 3 / 4);
    let popup_h = height.min(24).min(height * 3 / 4);
    let aquamarine = Color::new(0, 180, 255);

    let shortcuts: &[(&str, &str)] = &[
        ("General", ""),
        ("  ?",           "Show this help"),
        ("  Ctrl+Q/D",    "Quit"),
        ("  Esc",          "Cancel / back to Normal"),
        ("", ""),
        ("Input", ""),
        ("  Enter",        "Send message"),
        ("  Shift+Enter",  "New line"),
        ("  Tab",          "Autocomplete"),
        ("  Up/Down",      "Autocomplete navigation"),
        ("", ""),
        ("Dialogs", ""),
        ("  Ctrl+M",       "Model selector"),
        ("  Ctrl+T",       "Theme selector"),
        ("", ""),
        ("Processing", ""),
        ("  Esc",          "Interrupt agent"),
    ];

    let mut children = Vec::new();
    for &(key, desc) in shortcuts {
        if key.is_empty() && desc.is_empty() {
            children.push(jsx! { <Text>{""}</Text> });
        } else if desc.is_empty() {
            // Section header
            children.push(jsx! {
                <Text style={Style::new().fg(aquamarine).bold()}>{key}</Text>
            });
        } else {
            let line = format!("{:<18} {}", key, desc);
            children.push(jsx! {
                <Text style={theme.dim}>{line.as_str()}</Text>
            });
        }
    }

    let title = " Keyboard Shortcuts ";
    jsx! {
        <Box border style={Style::new().fg(aquamarine)} title={title} width={popup_w} height={popup_h}>
            {build_column(children)}
        </Box>
    }
}

fn center_text(text: &str, width: usize) -> String {
    let text_len = text.chars().count();
    if text_len >= width {
        return text.to_string();
    }
    let padding = (width - text_len) / 2;
    format!("{}{}", " ".repeat(padding), text)
}
