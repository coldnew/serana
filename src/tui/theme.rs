//! Theme definitions for the TUI

use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub accent: Color,
    pub success: Color,
    pub error: Color,
    pub warning: Color,
    pub info: Color,
    pub dim: Color,
    pub border: Color,
    pub user_msg_bg: Color,
    pub agent_msg_bg: Color,
    pub system_msg_bg: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            background: Color::Rgb(18, 18, 24),
            foreground: Color::Rgb(220, 220, 240),
            accent: Color::Rgb(88, 166, 255),
            success: Color::Rgb(68, 180, 120),
            error: Color::Rgb(230, 80, 80),
            warning: Color::Rgb(255, 180, 60),
            info: Color::Rgb(100, 180, 200),
            dim: Color::Rgb(100, 100, 120),
            border: Color::Rgb(60, 60, 72),
            user_msg_bg: Color::Rgb(35, 35, 45),
            agent_msg_bg: Color::Rgb(25, 25, 35),
            system_msg_bg: Color::Rgb(30, 30, 40),
        }
    }

    pub fn light() -> Self {
        Self {
            background: Color::Rgb(248, 248, 248),
            foreground: Color::Rgb(40, 40, 50),
            accent: Color::Rgb(0, 100, 200),
            success: Color::Rgb(40, 140, 80),
            error: Color::Rgb(200, 50, 50),
            warning: Color::Rgb(200, 120, 20),
            info: Color::Rgb(30, 120, 140),
            dim: Color::Rgb(120, 120, 140),
            border: Color::Rgb(200, 200, 210),
            user_msg_bg: Color::Rgb(235, 235, 245),
            agent_msg_bg: Color::Rgb(245, 245, 255),
            system_msg_bg: Color::Rgb(240, 240, 250),
        }
    }

    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border)
    }

    pub fn block<'a>(&self, title: &'a str) -> ratatui::widgets::Block<'a> {
        ratatui::widgets::Block::bordered()
            .title(title)
            .border_set(ratatui::symbols::border::ROUNDED)
            .border_style(self.border_style())
    }

    pub fn title_style(&self) -> Style {
        Style::default().fg(self.accent).add_modifier(Modifier::BOLD)
    }

    pub fn user_style(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn agent_style(&self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn system_style(&self) -> Style {
        Style::default().fg(self.warning)
    }

    pub fn accent_style(&self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn dim_style(&self) -> Style {
        Style::default().fg(self.dim)
    }

    pub fn status_mode_normal(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn status_mode_input(&self) -> Style {
        Style::default().fg(self.warning)
    }

    pub fn status_mode_processing(&self) -> Style {
        Style::default().fg(self.error)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
