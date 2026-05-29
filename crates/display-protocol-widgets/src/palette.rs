pub use display_protocol::Color;

pub const PRIMARY: Color = Color::new(59, 130, 246);
pub const SUCCESS: Color = Color::new(34, 197, 94);
pub const WARNING: Color = Color::new(234, 179, 8);
pub const DANGER: Color = Color::new(239, 68, 68);
pub const INFO: Color = Color::new(14, 165, 233);

pub const DARK: Color = Color::new(30, 30, 30);
pub const LIGHT: Color = Color::new(245, 245, 245);
pub const MUTED: Color = Color::new(107, 114, 128);
pub const WHITE: Color = Color::WHITE;
pub const BLACK: Color = Color::BLACK;

pub fn foreground_for_bg(bg: Color) -> Color {
    let brightness = 0.299 * bg.r as f32 + 0.587 * bg.g as f32 + 0.114 * bg.b as f32;
    if brightness > 128.0 { BLACK } else { WHITE }
}
