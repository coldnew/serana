//! Chat panel component

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Chat panel for displaying conversation
pub struct ChatPanel<'a> {
    messages: &'a [super::super::app::ChatMessage],
}

impl<'a> ChatPanel<'a> {
    pub fn new(messages: &'a [super::super::app::ChatMessage]) -> Self {
        Self { messages }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Chat")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));

        let lines: Vec<Line> = self
            .messages
            .iter()
            .flat_map(|msg| {
                use super::super::app::MessageRole;
                
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

                lines
            })
            .collect();

        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }
}
