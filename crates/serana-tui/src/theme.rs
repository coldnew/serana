use ratatui::style::{Color, Modifier, Style};

pub const CORAL: Color = Color::Rgb(255, 139, 109);
pub const BRIGHT_CORAL: Color = Color::Rgb(255, 111, 94);
pub const TEAL: Color = Color::Rgb(91, 192, 190);
pub const AQUAMARINE: Color = Color::Rgb(111, 255, 233);
pub const SEAFOAM_GREEN: Color = Color::Rgb(167, 240, 232);
pub const MUTED_TEAL: Color = Color::Rgb(119, 141, 169);
pub const DIM_TEAL: Color = Color::Rgb(90, 123, 153);
pub const DEEP_BLUE: Color = Color::Rgb(11, 19, 43);
pub const NAVY_BLUE: Color = Color::Rgb(28, 37, 65);
pub const OCEAN_BLUE: Color = Color::Rgb(58, 80, 107);
pub const MID_WATER: Color = Color::Rgb(27, 38, 59);
pub const DARK_BORDER: Color = Color::Rgb(30, 45, 63);
pub const ABYSS_BLACK: Color = Color::Rgb(5, 10, 26);
pub const DEEP_WATER: Color = Color::Rgb(13, 27, 42);
pub const USER_MSG_BG: Color = Color::Rgb(15, 26, 40);
pub const TOOL_PENDING_BG: Color = Color::Rgb(13, 21, 32);
pub const TOOL_SUCCESS_BG: Color = Color::Rgb(10, 20, 24);
pub const TOOL_ERROR_BG: Color = Color::Rgb(26, 13, 15);
pub const CODE_PURPLE: Color = Color::Rgb(212, 165, 255);
pub const DIFF_GREEN: Color = Color::Rgb(167, 240, 232);
pub const DIFF_RED: Color = Color::Rgb(255, 111, 94);
pub const DIFF_YELLOW: Color = Color::Rgb(255, 209, 102);

pub struct Theme {
    pub accent: Style,
    pub success: Style,
    pub warning: Style,
    pub error: Style,
    pub dim: Style,
    pub muted: Style,
    pub info: Style,
    pub user_fg: Style,
    pub agent_fg: Style,
    pub thinking: Style,
    pub border: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: Style::new().fg(CORAL).add_modifier(Modifier::BOLD),
            success: Style::new()
                .fg(SEAFOAM_GREEN)
                .add_modifier(Modifier::BOLD),
            warning: Style::new()
                .fg(DIFF_YELLOW)
                .add_modifier(Modifier::BOLD),
            error: Style::new()
                .fg(BRIGHT_CORAL)
                .add_modifier(Modifier::BOLD),
            dim: Style::new().fg(DIM_TEAL),
            muted: Style::new().fg(MUTED_TEAL),
            info: Style::new().fg(TEAL),
            user_fg: Style::new().fg(SEAFOAM_GREEN),
            agent_fg: Style::new().fg(CORAL),
            thinking: Style::new()
                .fg(MUTED_TEAL)
                .add_modifier(Modifier::ITALIC),
            border: Style::new().fg(DARK_BORDER),
        }
    }
}
