//! Status bar component

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::super::app::AppMode;

/// Status bar
pub struct StatusBar<'a> {
    mode: AppMode,
    status: &'a str,
    model: &'a str,
    token_count: usize,
}

impl<'a> StatusBar<'a> {
    pub fn new(mode: AppMode, status: &'a str, model: &'a str, token_count: usize) -> Self {
        Self {
            mode,
            status,
            model,
            token_count,
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let mode_style = match self.mode {
            AppMode::Normal => Style::default().fg(Color::Green),
            AppMode::Input => Style::default().fg(Color::Yellow),
            AppMode::Processing => Style::default().fg(Color::Red),
        };

        let status = Line::from(vec![
            Span::styled(
                match self.mode {
                    AppMode::Normal => "[NORMAL]",
                    AppMode::Input => "[INPUT]",
                    AppMode::Processing => "[PROCESSING]",
                },
                mode_style.add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | "),
            Span::styled(self.status, Style::default().fg(Color::White)),
            Span::raw(" | "),
            Span::styled(self.model, Style::default().fg(Color::Cyan)),
            Span::raw(" | "),
            Span::styled(
                format!("{}k tokens", self.token_count / 1000),
                Style::default().fg(Color::DarkGray),
            ),
        ]);

        let paragraph = Paragraph::new(status);
        f.render_widget(paragraph, area);
    }
}
