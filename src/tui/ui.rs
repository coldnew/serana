//! Rich UI rendering for TUI with oh-my-pi inspired design

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    Frame,
};

use super::app::{App, AppMode, MessageRole};
use super::theme::Theme;

/// Draw the main UI
pub fn draw(f: &mut Frame, app: &App) {
    let theme = Theme::default();
    
    // Main layout with oh-my-pi style proportions
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),   // Header bar
            Constraint::Min(10),     // Main content
            Constraint::Length(5),   // Input area (larger)
            Constraint::Length(1),   // Status bar
        ])
        .split(f.area());

    // Content area: sidebar + chat
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(30),  // Sidebar
            Constraint::Min(50),     // Chat area
        ])
        .split(chunks[1]);

    draw_header(f, app, theme, chunks[0]);
    draw_sidebar(f, app, theme, content_chunks[0]);
    draw_chat(f, app, theme, content_chunks[1]);
    draw_input(f, app, theme, chunks[2]);
    draw_status_bar(f, app, theme, chunks[3]);
}

/// Draw slim header bar with workspace info
fn draw_header(f: &mut Frame, app: &App, theme: Theme, area: Rect) {
    let workspace_name = app.workspace
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    
    let header = Line::from(vec![
        Span::styled(" ◉ ", Style::default().fg(theme.accent)),
        Span::styled("serana", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(" │ ", theme.dim_style()),
        Span::styled(workspace_name, Style::default().fg(theme.info)),
        Span::styled(
            "─".repeat(area.width.saturating_sub(30) as usize),
            theme.dim_style(),
        ),
    ]);
    
    f.render_widget(Paragraph::new(header), area);
}

/// Draw sidebar with context and tools
#[allow(clippy::vec_init_then_push)]
fn draw_sidebar(f: &mut Frame, app: &App, theme: Theme, area: Rect) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(theme.border))
        .padding(Padding::new(1, 0, 0, 0));
    
    let mut lines: Vec<Line> = Vec::new();
    
    // Workspace section
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("┈", theme.dim_style()),
        Span::styled(" WORKSPACE ", Style::default().fg(theme.dim).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  ⬡ ", Style::default().fg(theme.success)),
        Span::styled(
            app.workspace.display().to_string(),
            Style::default().fg(theme.foreground),
        ),
    ]));
    lines.push(Line::from(""));
    
    // Context files section
    if app.context_files.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("┈", theme.dim_style()),
            Span::styled(" CONTEXT ", Style::default().fg(theme.dim).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  No files in context", theme.dim_style()),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("┈", theme.dim_style()),
            Span::styled(" CONTEXT ", Style::default().fg(theme.dim).add_modifier(Modifier::BOLD)),
            Span::styled(format!("({})", app.context_files.len()), theme.dim_style()),
        ]));
        
        for file in app.context_files.iter().take(8) {
            let name = file.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?");
            lines.push(Line::from(vec![
                Span::styled("  › ", Style::default().fg(theme.accent)),
                Span::styled(name, Style::default().fg(theme.foreground)),
            ]));
        }
        
        if app.context_files.len() > 8 {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  +{} more", app.context_files.len() - 8),
                    theme.dim_style(),
                ),
            ]));
        }
    }
    
    lines.push(Line::from(""));
    
    // Tools section
    lines.push(Line::from(vec![
        Span::styled("┈", theme.dim_style()),
        Span::styled(" TOOLS ", Style::default().fg(theme.dim).add_modifier(Modifier::BOLD)),
    ]));
    
    let tools = [
        ("read", "Read files"),
        ("write", "Write files"),
        ("edit", "Edit files"),
        ("bash", "Run commands"),
    ];
    
    for (name, desc) in tools {
        lines.push(Line::from(vec![
            Span::styled("  ⚡ ", Style::default().fg(theme.warning)),
            Span::styled(name, Style::default().fg(theme.foreground)),
            Span::styled(format!(" • {}", desc), theme.dim_style()),
        ]));
    }
    
    // Keybindings at bottom
    lines.push(Line::from(""));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("┈", theme.dim_style()),
        Span::styled(" KEYS ", Style::default().fg(theme.dim).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled("i", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(" input  ", theme.dim_style()),
        Span::styled("q", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(" quit", theme.dim_style()),
    ]));
    
    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

/// Draw chat area with messages
fn draw_chat(f: &mut Frame, app: &App, theme: Theme, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    
    if app.messages.is_empty() {
        // Welcome screen with oh-my-pi style branding
        lines.push(Line::from(""));
        lines.push(Line::from(""));
        lines.push(Line::from(""));
        
        // ASCII art logo
        let logo_lines = vec![
            "   ╭─────────╮",
            "   │  ◉ ◉ ◉  │",
            "   │  SERANA │",
            "   ╰─────────╯",
        ];
        
        for logo_line in logo_lines {
            lines.push(Line::from(vec![
                Span::styled(logo_line, Style::default().fg(theme.accent)),
            ]));
        }
        
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("   Your AI coding companion", Style::default().fg(theme.dim).add_modifier(Modifier::ITALIC)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("   Model: ", theme.dim_style()),
            Span::styled(&app.model, Style::default().fg(theme.info)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("   Press ", theme.dim_style()),
            Span::styled("i", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" to start a conversation", theme.dim_style()),
        ]));
    } else {
        // Render messages with oh-my-pi style backgrounds
        for msg in &app.messages {
            match msg.role {
                MessageRole::User => {
                    // User message with subtle background
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("▌", Style::default().fg(theme.success)),
                        Span::styled(" You ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
                    ]));
                    
                    for line in msg.content.lines() {
                        lines.push(Line::from(format!("  {}", line)));
                    }
                }
                MessageRole::Agent => {
                    // Agent message with accent
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("▌", Style::default().fg(theme.accent)),
                        Span::styled(" Serana ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    ]));
                    
                    for line in msg.content.lines() {
                        lines.push(Line::from(format!("  {}", line)));
                    }
                    
                    // Tool calls with distinct styling
                    if !msg.tool_calls.is_empty() {
                        lines.push(Line::from(""));
                        for tool in &msg.tool_calls {
                            lines.push(Line::from(vec![
                                Span::styled("  ⚡ ", Style::default().fg(theme.warning)),
                                Span::styled(tool, Style::default().fg(theme.info)),
                            ]));
                        }
                    }
                }
                MessageRole::System => {
                    // System message with warning color
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("▌", Style::default().fg(theme.warning)),
                        Span::styled(" System ", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD)),
                    ]));
                    
                    for line in msg.content.lines() {
                        lines.push(Line::from(vec![
                            Span::styled(format!("  {}", line), theme.dim_style()),
                        ]));
                    }
                }
            }
        }
        
        // Add padding at bottom for scrolling room
        lines.push(Line::from(""));
        lines.push(Line::from(""));
    }
    
    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

/// Draw input area with oh-my-pi style
fn draw_input(f: &mut Frame, app: &App, theme: Theme, area: Rect) {
    let (border_color, input_style, placeholder) = match app.mode {
        AppMode::Normal => (theme.border, theme.dim_style(), Some("Press 'i' to start typing...")),
        AppMode::Input => (theme.accent, Style::default().fg(theme.foreground), None),
        AppMode::Processing => (theme.warning, theme.dim_style(), Some("⏳ Processing...")),
    };
    
    let title = match app.mode {
        AppMode::Input => " ✏ Input ",
        AppMode::Processing => " ⏳ Working ",
        _ => " Input ",
    };
    
    let block = Block::bordered()
        .title(title)
        .border_set(ratatui::symbols::border::ROUNDED)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::new(1, 1, 0, 0));
    
    let text = if app.input.is_empty() {
        if let Some(ph) = placeholder {
            Span::styled(ph, theme.dim_style())
        } else {
            Span::raw("")
        }
    } else {
        Span::styled(&app.input, input_style)
    };
    
    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
    
    // Show cursor in input mode
    if app.mode == AppMode::Input {
        f.set_cursor_position((
            area.x + app.input_cursor as u16 + 2, // +2 for border and padding
            area.y + 1,
        ));
    }
}

/// Draw status bar with oh-my-pi style
fn draw_status_bar(f: &mut Frame, app: &App, theme: Theme, area: Rect) {
    let mode_text = match app.mode {
        AppMode::Normal => "NORMAL",
        AppMode::Input => "INPUT",
        AppMode::Processing => "WORKING",
    };
    
    let mode_color = match app.mode {
        AppMode::Normal => theme.success,
        AppMode::Input => theme.warning,
        AppMode::Processing => theme.accent,
    };
    
    // Spinner animation for processing mode
    let spinner = if app.mode == AppMode::Processing {
        match std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| (d.as_millis() / 200) % 4)
            .unwrap_or(0)
        {
            0 => "⠋",
            1 => "⠙",
            2 => "⠹",
            _ => "⠸",
        }
    } else {
        ""
    };
    
    let status = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(mode_text, Style::default().fg(mode_color).add_modifier(Modifier::BOLD)),
        Span::styled(" │ ", theme.dim_style()),
        Span::styled(spinner, Style::default().fg(theme.accent)),
        Span::styled(&app.status, Style::default().fg(theme.foreground)),
        Span::styled(" │ ", theme.dim_style()),
        Span::styled("◈ ", Style::default().fg(theme.accent)),
        Span::styled(&app.model, Style::default().fg(theme.info)),
        Span::styled(" │ ", theme.dim_style()),
        Span::styled("◪ ", Style::default().fg(theme.warning)),
        Span::styled(format!("{}k", app.token_count / 1000), theme.dim_style()),
    ]);
    
    f.render_widget(Paragraph::new(status), area);
}
