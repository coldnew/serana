//! UI rendering for TUI

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::app::{App, AppMode, MessageRole};

/// Draw the main UI
pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),       // Main area
            Constraint::Length(3),    // Input
            Constraint::Length(1),    // Status
        ])
        .split(f.area());

    // Main area with sidebar and chat
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // Sidebar
            Constraint::Percentage(80), // Chat
        ])
        .split(chunks[0]);

    draw_sidebar(f, app, main_chunks[0]);
    draw_chat(f, app, main_chunks[1]);
    draw_input(f, app, chunks[1]);
    draw_status(f, app, chunks[2]);
}

fn draw_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title("Context")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));

    let mut lines = vec![Line::from(Span::styled(
        format!("📁 {}", app.workspace.display()),
        Style::default().fg(Color::Yellow),
    ))];

    if !app.context_files.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Files in context:",
            Style::default().fg(Color::DarkGray),
        )));
        for file in &app.context_files {
            lines.push(Line::from(format!(
                "  {}",
                file.file_name().unwrap_or_default().to_string_lossy()
            )));
        }
    }

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn draw_chat(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title("Chat")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));

    let lines: Vec<Line> = app
        .messages
        .iter()
        .flat_map(|msg| {
            let (prefix, style) = match msg.role {
                MessageRole::User => ("You: ", Style::default().fg(Color::Green)),
                MessageRole::Agent => ("Agent: ", Style::default().fg(Color::Cyan)),
                MessageRole::System => ("System: ", Style::default().fg(Color::Yellow)),
            };
            
            let mut lines = vec![Line::from(vec![
                Span::styled(prefix, style.add_modifier(Modifier::BOLD)),
            ])];
            
            for line in msg.content.lines() {
                lines.push(Line::from(format!("  {}", line)));
            }
            
            for tool in &msg.tool_calls {
                lines.push(Line::from(Span::styled(
                    format!("  🔧 {}", tool),
                    Style::default().fg(Color::Magenta),
                )));
            }
            
            lines.push(Line::from(""));
            lines
        })
        .collect();

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    let style = match app.mode {
        AppMode::Input => Style::default().fg(Color::White),
        AppMode::Processing => Style::default().fg(Color::DarkGray),
        _ => Style::default().fg(Color::Gray),
    };

    let block = Block::default()
        .title("Input (i to edit, Enter to send, Esc to cancel)")
        .borders(Borders::ALL)
        .border_style(style);

    let paragraph = Paragraph::new(app.input.as_str())
        .style(style)
        .block(block);

    f.render_widget(paragraph, area);

    // Show cursor in input mode
    if app.mode == AppMode::Input {
        f.set_cursor_position((
            area.x + app.input_cursor as u16 + 1, // +1 for border
            area.y + 1,
        ));
    }
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let mode_style = match app.mode {
        AppMode::Normal => Style::default().fg(Color::Green),
        AppMode::Input => Style::default().fg(Color::Yellow),
        AppMode::Processing => Style::default().fg(Color::Red),
    };

    let status = Line::from(vec![
        Span::styled(
            match app.mode {
                AppMode::Normal => "[NORMAL]",
                AppMode::Input => "[INPUT]",
                AppMode::Processing => "[PROCESSING]",
            },
            mode_style.add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(&app.status, Style::default().fg(Color::White)),
        Span::raw(" | "),
        Span::styled(&app.model, Style::default().fg(Color::Cyan)),
        Span::raw(" | "),
        Span::styled(
            format!("{}k tokens", app.token_count / 1000),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let paragraph = Paragraph::new(status);
    f.render_widget(paragraph, area);
}
