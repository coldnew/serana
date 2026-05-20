//! Sidebar panel component

use std::path::PathBuf;

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Sidebar panel for context files
pub struct SidebarPanel<'a> {
    workspace: &'a PathBuf,
    context_files: &'a [PathBuf],
}

impl<'a> SidebarPanel<'a> {
    pub fn new(workspace: &'a PathBuf, context_files: &'a [PathBuf]) -> Self {
        Self {
            workspace,
            context_files,
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Context")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));

        let mut lines = vec![Line::from(Span::styled(
            format!("📁 {}", self.workspace.display()),
            Style::default().fg(Color::Yellow),
        ))];

        if !self.context_files.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Files in context:",
                Style::default().fg(Color::DarkGray),
            )));
            for file in self.context_files {
                lines.push(Line::from(format!(
                    "  {}",
                    file.file_name().unwrap_or_default().to_string_lossy()
                )));
            }
        }

        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }
}
