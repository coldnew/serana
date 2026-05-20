//! Simple vertical layout matching oh-my-pi style

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    Frame,
};

use super::app::{App, AppMode, MessageRole};
use super::theme::Theme;

/// Draw the main UI - simple vertical layout like oh-my-pi
pub fn draw(f: &mut Frame, app: &App) {
    let theme = Theme::default();
    
    // Simple vertical layout: chat area + input area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),     // Chat area (flexible)
            Constraint::Length(3),   // Input area (compact)
        ])
        .split(f.area());

    draw_chat(f, app, theme, chunks[0]);
    draw_input(f, app, theme, chunks[1]);
}

/// Draw chat area with messages (scrolling vertically)
fn draw_chat(f: &mut Frame, app: &App, theme: Theme, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    
    if app.messages.is_empty() {
        // Welcome screen - simple and clean like oh-my-pi
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("serana", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Your AI coding companion", Style::default().fg(theme.dim)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Model: ", Style::default().fg(theme.dim)),
            Span::styled(&app.model, Style::default().fg(theme.info)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Type a message and press Enter to start", Style::default().fg(theme.dim)),
        ]));
    } else {
        // Render messages
        for msg in &app.messages {
            match msg.role {
                MessageRole::User => {
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("You", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
                    ]));
                    
                    for line in msg.content.lines() {
                        lines.push(Line::from(line.to_string()));
                    }
                }
                MessageRole::Agent => {
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("Serana", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    ]));
                    
                    for line in msg.content.lines() {
                        lines.push(Line::from(line.to_string()));
                    }
                    
                    // Tool calls inline
                    if !msg.tool_calls.is_empty() {
                        lines.push(Line::from(""));
                        for tool in &msg.tool_calls {
                            lines.push(Line::from(vec![
                                Span::styled("⚡ ", Style::default().fg(theme.warning)),
                                Span::styled(tool, Style::default().fg(theme.info)),
                            ]));
                        }
                    }
                }
                MessageRole::System => {
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("System: ", Style::default().fg(theme.warning)),
                        Span::styled(&msg.content, Style::default().fg(theme.dim)),
                    ]));
                }
            }
        }
    }
    
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

/// Draw input area at bottom
fn draw_input(f: &mut Frame, app: &App, theme: Theme, area: Rect) {
    let border_color = match app.mode {
        AppMode::Normal => theme.dim,
        AppMode::Input => theme.accent,
        AppMode::Processing => theme.warning,
    };
    
    let title = match app.mode {
        AppMode::Input => " Input ",
        AppMode::Processing => " Working ",
        AppMode::Normal => " Input ",
    };
    
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::new(1, 1, 0, 0));
    
    let text = if app.input.is_empty() {
        if app.mode == AppMode::Processing {
            Span::styled("Processing...", Style::default().fg(theme.dim))
        } else {
            Span::styled("Type your message...", Style::default().fg(Color::DarkGray))
        }
    } else {
        Span::raw(&app.input)
    };
    
    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}
