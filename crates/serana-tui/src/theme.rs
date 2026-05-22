//! JSON-configurable theme system with 62+ color tokens.
//!
//! Supports auto dark/light detection, colorblind mode, and live reload.

use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, RwLock};

// Default oceanic palette
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

/// RGB color representation for JSON serialization.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn to_color(self) -> Color {
        Color::Rgb(self.r, self.g, self.b)
    }

    pub fn from_color(c: Color) -> Option<Self> {
        match c {
            Color::Rgb(r, g, b) => Some(Self { r, g, b }),
            _ => None,
        }
    }
}

impl From<RgbColor> for Color {
    fn from(c: RgbColor) -> Self {
        Color::Rgb(c.r, c.g, c.b)
    }
}

/// Full theme configuration with 62+ color tokens.
/// All colors are stored as RgbColor for JSON round-tripping.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    // === Primary colors ===
    pub accent: RgbColor,
    pub accent_secondary: RgbColor,
    pub success: RgbColor,
    pub warning: RgbColor,
    pub error: RgbColor,
    pub info: RgbColor,

    // === Text colors ===
    pub text_primary: RgbColor,
    pub text_secondary: RgbColor,
    pub text_dim: RgbColor,
    pub text_muted: RgbColor,
    pub text_inverse: RgbColor,

    // === Message colors ===
    pub user_fg: RgbColor,
    pub user_bg: RgbColor,
    pub agent_fg: RgbColor,
    pub agent_bg: RgbColor,
    pub system_fg: RgbColor,
    pub system_bg: RgbColor,

    // === Tool colors ===
    pub tool_pending_fg: RgbColor,
    pub tool_pending_bg: RgbColor,
    pub tool_success_fg: RgbColor,
    pub tool_success_bg: RgbColor,
    pub tool_error_fg: RgbColor,
    pub tool_error_bg: RgbColor,
    pub tool_running_fg: RgbColor,
    pub tool_running_bg: RgbColor,

    // === Diff colors ===
    pub diff_add_fg: RgbColor,
    pub diff_add_bg: RgbColor,
    pub diff_remove_fg: RgbColor,
    pub diff_remove_bg: RgbColor,
    pub diff_modify_fg: RgbColor,
    pub diff_modify_bg: RgbColor,
    pub diff_context_fg: RgbColor,
    pub diff_hunk_fg: RgbColor,

    // === Code colors ===
    pub code_fg: RgbColor,
    pub code_bg: RgbColor,
    pub code_keyword: RgbColor,
    pub code_string: RgbColor,
    pub code_comment: RgbColor,
    pub code_function: RgbColor,
    pub code_number: RgbColor,
    pub code_operator: RgbColor,
    pub code_type: RgbColor,

    // === UI chrome ===
    pub border: RgbColor,
    pub border_active: RgbColor,
    pub border_focused: RgbColor,
    pub title: RgbColor,
    pub title_active: RgbColor,
    pub scrollbar_thumb: RgbColor,
    pub scrollbar_track: RgbColor,
    pub selection_fg: RgbColor,
    pub selection_bg: RgbColor,

    // === Background tokens ===
    pub bg_primary: RgbColor,
    pub bg_secondary: RgbColor,
    pub bg_elevated: RgbColor,
    pub bg_input: RgbColor,
    pub bg_header: RgbColor,
    pub bg_status: RgbColor,
    pub bg_popup: RgbColor,

    // === Thinking/reasoning ===
    pub thinking_fg: RgbColor,
    pub thinking_bg: RgbColor,

    // === Special ===
    pub hyperlink: RgbColor,
    pub badge_fg: RgbColor,
    pub badge_bg: RgbColor,
    pub progress_bar_filled: RgbColor,
    pub progress_bar_empty: RgbColor,
    pub highlight: RgbColor,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            accent: RgbColor::new(255, 139, 109),
            accent_secondary: RgbColor::new(212, 165, 255),
            success: RgbColor::new(167, 240, 232),
            warning: RgbColor::new(255, 209, 102),
            error: RgbColor::new(255, 111, 94),
            info: RgbColor::new(91, 192, 190),
            text_primary: RgbColor::new(230, 237, 243),
            text_secondary: RgbColor::new(168, 179, 200),
            text_dim: RgbColor::new(90, 123, 153),
            text_muted: RgbColor::new(119, 141, 169),
            text_inverse: RgbColor::new(5, 10, 26),
            user_fg: RgbColor::new(167, 240, 232),
            user_bg: RgbColor::new(15, 26, 40),
            agent_fg: RgbColor::new(255, 139, 109),
            agent_bg: RgbColor::new(11, 19, 43),
            system_fg: RgbColor::new(255, 209, 102),
            system_bg: RgbColor::new(28, 37, 65),
            tool_pending_fg: RgbColor::new(119, 141, 169),
            tool_pending_bg: RgbColor::new(13, 21, 32),
            tool_success_fg: RgbColor::new(167, 240, 232),
            tool_success_bg: RgbColor::new(10, 20, 24),
            tool_error_fg: RgbColor::new(255, 111, 94),
            tool_error_bg: RgbColor::new(26, 13, 15),
            tool_running_fg: RgbColor::new(255, 139, 109),
            tool_running_bg: RgbColor::new(27, 38, 59),
            diff_add_fg: RgbColor::new(167, 240, 232),
            diff_add_bg: RgbColor::new(10, 20, 24),
            diff_remove_fg: RgbColor::new(255, 111, 94),
            diff_remove_bg: RgbColor::new(26, 13, 15),
            diff_modify_fg: RgbColor::new(255, 209, 102),
            diff_modify_bg: RgbColor::new(25, 22, 10),
            diff_context_fg: RgbColor::new(119, 141, 169),
            diff_hunk_fg: RgbColor::new(91, 192, 190),
            code_fg: RgbColor::new(167, 240, 232),
            code_bg: RgbColor::new(11, 19, 43),
            code_keyword: RgbColor::new(212, 165, 255),
            code_string: RgbColor::new(167, 240, 232),
            code_comment: RgbColor::new(90, 123, 153),
            code_function: RgbColor::new(255, 139, 109),
            code_number: RgbColor::new(255, 209, 102),
            code_operator: RgbColor::new(91, 192, 190),
            code_type: RgbColor::new(111, 255, 233),
            border: RgbColor::new(30, 45, 63),
            border_active: RgbColor::new(58, 80, 107),
            border_focused: RgbColor::new(91, 192, 190),
            title: RgbColor::new(119, 141, 169),
            title_active: RgbColor::new(255, 139, 109),
            scrollbar_thumb: RgbColor::new(58, 80, 107),
            scrollbar_track: RgbColor::new(11, 19, 43),
            selection_fg: RgbColor::new(5, 10, 26),
            selection_bg: RgbColor::new(91, 192, 190),
            bg_primary: RgbColor::new(5, 10, 26),
            bg_secondary: RgbColor::new(11, 19, 43),
            bg_elevated: RgbColor::new(28, 37, 65),
            bg_input: RgbColor::new(13, 27, 42),
            bg_header: RgbColor::new(11, 19, 43),
            bg_status: RgbColor::new(13, 27, 42),
            bg_popup: RgbColor::new(28, 37, 65),
            thinking_fg: RgbColor::new(119, 141, 169),
            thinking_bg: RgbColor::new(15, 26, 40),
            hyperlink: RgbColor::new(111, 255, 233),
            badge_fg: RgbColor::new(5, 10, 26),
            badge_bg: RgbColor::new(255, 139, 109),
            progress_bar_filled: RgbColor::new(91, 192, 190),
            progress_bar_empty: RgbColor::new(30, 45, 63),
            highlight: RgbColor::new(255, 209, 102),
        }
    }
}

impl ThemeConfig {
    /// Load from a JSON file, falling back to defaults.
    pub fn load_from_file(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(json) => match serde_json::from_str(&json) {
                Ok(theme) => return theme,
                Err(e) => {
                    tracing::warn!("Failed to parse theme file {}: {}", path.display(), e);
                }
            },
            Err(_) => {}
        }
        Self::default()
    }

    /// Save to a JSON file.
    pub fn save_to_file(&self, path: &Path) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self).unwrap();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, json)
    }

    /// Create a light-mode variant.
    pub fn light_mode() -> Self {
        let mut theme = Self::default();
        theme.bg_primary = RgbColor::new(248, 249, 252);
        theme.bg_secondary = RgbColor::new(255, 255, 255);
        theme.bg_elevated = RgbColor::new(240, 243, 248);
        theme.bg_input = RgbColor::new(255, 255, 255);
        theme.bg_header = RgbColor::new(240, 243, 248);
        theme.bg_status = RgbColor::new(240, 243, 248);
        theme.bg_popup = RgbColor::new(255, 255, 255);
        theme.text_primary = RgbColor::new(15, 23, 42);
        theme.text_secondary = RgbColor::new(71, 85, 105);
        theme.text_dim = RgbColor::new(148, 163, 184);
        theme.text_muted = RgbColor::new(100, 116, 139);
        theme.border = RgbColor::new(203, 213, 225);
        theme.border_active = RgbColor::new(148, 163, 184);
        theme.border_focused = RgbColor::new(91, 192, 190);
        theme.title = RgbColor::new(71, 85, 105);
        theme.title_active = RgbColor::new(200, 83, 60);
        theme.accent = RgbColor::new(200, 83, 60);
        theme.success = RgbColor::new(16, 120, 90);
        theme.warning = RgbColor::new(180, 130, 20);
        theme.error = RgbColor::new(200, 60, 50);
        theme.info = RgbColor::new(40, 130, 150);
        theme.user_fg = RgbColor::new(16, 120, 90);
        theme.agent_fg = RgbColor::new(200, 83, 60);
        theme.code_fg = RgbColor::new(40, 50, 80);
        theme.code_bg = RgbColor::new(248, 249, 252);
        theme.code_keyword = RgbColor::new(120, 70, 200);
        theme.code_string = RgbColor::new(16, 120, 90);
        theme.code_comment = RgbColor::new(148, 163, 184);
        theme.code_function = RgbColor::new(200, 83, 60);
        theme.code_number = RgbColor::new(180, 130, 20);
        theme.code_operator = RgbColor::new(40, 130, 150);
        theme.code_type = RgbColor::new(60, 160, 200);
        theme.diff_add_fg = RgbColor::new(16, 120, 90);
        theme.diff_add_bg = RgbColor::new(230, 248, 235);
        theme.diff_remove_fg = RgbColor::new(200, 60, 50);
        theme.diff_remove_bg = RgbColor::new(255, 235, 233);
        theme.diff_modify_fg = RgbColor::new(180, 130, 20);
        theme.diff_modify_bg = RgbColor::new(255, 250, 230);
        theme.thinking_fg = RgbColor::new(100, 116, 139);
        theme.thinking_bg = RgbColor::new(240, 243, 248);
        theme.hyperlink = RgbColor::new(40, 130, 200);
        theme.selection_bg = RgbColor::new(91, 192, 190);
        theme.highlight = RgbColor::new(180, 130, 20);
        theme
    }

    /// Create a colorblind-friendly variant (deuteranopia/protanopia safe).
    pub fn colorblind_mode() -> Self {
        let mut theme = Self::default();
        // Use blue/orange instead of red/green
        theme.success = RgbColor::new(86, 180, 233);     // Blue
        theme.error = RgbColor::new(230, 159, 0);        // Orange
        theme.warning = RgbColor::new(240, 228, 66);     // Yellow
        theme.diff_add_fg = RgbColor::new(86, 180, 233); // Blue, not green
        theme.diff_remove_fg = RgbColor::new(230, 159, 0); // Orange, not red
        theme.diff_modify_fg = RgbColor::new(240, 228, 66);
        theme.tool_success_fg = RgbColor::new(86, 180, 233);
        theme.tool_error_fg = RgbColor::new(230, 159, 0);
        theme.user_fg = RgbColor::new(86, 180, 233);
        theme
    }
}

/// Runtime Theme: compiled from ThemeConfig, ready for rendering.
#[derive(Debug)]
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
        let config = ThemeConfig::default();
        Self::from_config(&config)
    }
}

impl Theme {
    /// Build a Theme from a ThemeConfig.
    pub fn from_config(config: &ThemeConfig) -> Self {
        Self {
            accent: Style::new().fg(config.accent.to_color()).add_modifier(Modifier::BOLD),
            success: Style::new().fg(config.success.to_color()).add_modifier(Modifier::BOLD),
            warning: Style::new().fg(config.warning.to_color()).add_modifier(Modifier::BOLD),
            error: Style::new().fg(config.error.to_color()).add_modifier(Modifier::BOLD),
            dim: Style::new().fg(config.text_dim.to_color()),
            muted: Style::new().fg(config.text_muted.to_color()),
            info: Style::new().fg(config.info.to_color()),
            user_fg: Style::new().fg(config.user_fg.to_color()),
            agent_fg: Style::new().fg(config.agent_fg.to_color()),
            thinking: Style::new().fg(config.thinking_fg.to_color()).add_modifier(Modifier::ITALIC),
            border: Style::new().fg(config.border.to_color()),
        }
    }
}

/// Global theme manager with live reload support.
pub struct ThemeManager {
    config: Arc<RwLock<ThemeConfig>>,
    theme: Arc<RwLock<Theme>>,
    dark_mode: bool,
    watch_path: Option<std::path::PathBuf>,
    last_modified: Option<std::time::SystemTime>,
}

impl ThemeManager {
    pub fn new() -> Self {
        let config = ThemeConfig::default();
        let theme = Theme::from_config(&config);
        Self {
            config: Arc::new(RwLock::new(config)),
            theme: Arc::new(RwLock::new(theme)),
            dark_mode: true,
            watch_path: None,
            last_modified: None,
        }
    }

    /// Load theme from file, enabling live reload tracking.
    pub fn load(&mut self, path: &Path) {
        let config = ThemeConfig::load_from_file(path);
        let theme = Theme::from_config(&config);
        self.last_modified = std::fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok());
        self.watch_path = Some(path.to_path_buf());
        *self.config.write().unwrap() = config;
        *self.theme.write().unwrap() = theme;
    }

    /// Poll for file changes and reload if modified. Call on each tick.
    /// Returns true if the theme was reloaded.
    pub fn check_reload(&mut self) -> bool {
        let path = match &self.watch_path {
            Some(p) => p.clone(),
            None => return false,
        };
        let modified = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());
        let should_reload = match (modified, self.last_modified) {
            (Some(current), Some(last)) => current > last,
            (Some(_), None) => true,
            _ => false,
        };
        if should_reload {
            let config = ThemeConfig::load_from_file(&path);
            let theme = Theme::from_config(&config);
            *self.config.write().unwrap() = config;
            *self.theme.write().unwrap() = theme;
            self.last_modified = modified;
            true
        } else {
            false
        }
    }

    /// Set dark/light mode.
    pub fn set_dark_mode(&mut self, dark: bool) {
        self.dark_mode = dark;
        let config = if dark { ThemeConfig::default() } else { ThemeConfig::light_mode() };
        let theme = Theme::from_config(&config);
        *self.config.write().unwrap() = config;
        *self.theme.write().unwrap() = theme;
    }

    /// Set colorblind mode.
    pub fn set_colorblind_mode(&mut self) {
        let config = ThemeConfig::colorblind_mode();
        let theme = Theme::from_config(&config);
        *self.config.write().unwrap() = config;
        *self.theme.write().unwrap() = theme;
    }

    /// Get the current Theme.
    pub fn get(&self) -> Theme {
        Theme::from_config(&self.config.read().unwrap())
    }

    /// Get the current ThemeConfig for serialization.
    pub fn get_config(&self) -> ThemeConfig {
        self.config.read().unwrap().clone()
    }
}

/// Auto-detect whether the terminal uses a dark background.
pub fn detect_dark_mode() -> bool {
    // Check COLORFGBG env var (set by some terminals)
    if let Ok(fg_bg) = std::env::var("COLORFGBG") {
        let parts: Vec<&str> = fg_bg.split(';').collect();
        if let Some(bg) = parts.last() {
            if let Ok(bg_num) = bg.parse::<u8>() {
                // Dark backgrounds are typically 0-6 or 8
                return bg_num <= 8;
            }
        }
    }

    // Check TERM_PROGRAM for known dark/light defaults
    if let Ok(program) = std::env::var("TERM_PROGRAM") {
        match program.as_str() {
            // Most of these default to dark
            "kitty" | "ghostty" | "iTerm.app" | "Alacritty" | "WezTerm" => return true,
            _ => {}
        }
    }

    // Default to dark mode
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_config_default() {
        let config = ThemeConfig::default();
        assert_eq!(config.accent.r, 255);
        assert_eq!(config.accent.g, 139);
    }

    #[test]
    fn test_theme_config_serialize_roundtrip() {
        let config = ThemeConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let restored: ThemeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.accent, restored.accent);
        assert_eq!(config.bg_primary, restored.bg_primary);
    }

    #[test]
    fn test_theme_from_config() {
        let config = ThemeConfig::default();
        let theme = Theme::from_config(&config);
        // Verify accent has BOLD modifier
        assert!(theme.accent.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_light_mode() {
        let config = ThemeConfig::light_mode();
        assert_eq!(config.bg_primary.r, 248);
        assert_eq!(config.text_primary.r, 15);
    }

    #[test]
    fn test_colorblind_mode() {
        let config = ThemeConfig::colorblind_mode();
        // Success should be blue, not green
        assert_eq!(config.success, RgbColor::new(86, 180, 233));
        // Error should be orange, not red
        assert_eq!(config.error, RgbColor::new(230, 159, 0));
    }

    #[test]
    fn test_rgb_color_conversions() {
        let c = RgbColor::new(100, 200, 50);
        let color: Color = c.into();
        assert_eq!(color, Color::Rgb(100, 200, 50));
        assert_eq!(RgbColor::from_color(color), Some(c));
    }

    #[test]
    fn test_json_file_roundtrip() {
        let dir = std::env::temp_dir().join("serana_theme_test");
        let path = dir.join("test_theme.json");
        let config = ThemeConfig::default();
        config.save_to_file(&path).unwrap();
        let loaded = ThemeConfig::load_from_file(&path);
        assert_eq!(config.accent, loaded.accent);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
