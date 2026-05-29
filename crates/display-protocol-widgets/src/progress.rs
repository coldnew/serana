use display_protocol::{ProgressNode, Style, UiNode};
use crate::palette;

/// A progress bar.
///
/// Works on both TUI and WGPU backends.
///
/// ```ignore
/// ProgressBar::new(0.75).build()                    // 75% filled
/// ProgressBar::new(0.5).width(40).build()           // custom width
/// ProgressBar::new(0.3).show_percent(false).build() // no label
/// ```
#[derive(Debug, Clone)]
pub struct ProgressBar {
    value: f32,
    max: f32,
    width: u16,
    filled_color: Option<display_protocol::Color>,
    empty_color: Option<display_protocol::Color>,
    show_percent: bool,
}

impl ProgressBar {
    /// Create a progress bar with value in 0.0..=1.0.
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            max: 1.0,
            width: 20,
            filled_color: None,
            empty_color: None,
            show_percent: true,
        }
    }

    /// Set raw value and max.
    pub fn with_range(value: f32, max: f32) -> Self {
        Self {
            value: value.clamp(0.0, max),
            max,
            width: 20,
            filled_color: None,
            empty_color: None,
            show_percent: true,
        }
    }

    pub fn width(mut self, w: u16) -> Self { self.width = w; self }
    pub fn filled_color(mut self, c: display_protocol::Color) -> Self { self.filled_color = Some(c); self }
    pub fn empty_color(mut self, c: display_protocol::Color) -> Self { self.empty_color = Some(c); self }
    pub fn show_percent(mut self, show: bool) -> Self { self.show_percent = show; self }

    pub fn build(self) -> UiNode {
        let filled_fg = self.filled_color.unwrap_or(palette::PRIMARY);
        let empty_fg = self.empty_color.unwrap_or(palette::DARK);

        UiNode::ProgressBar(ProgressNode {
            value: self.value,
            max: self.max,
            width: self.width,
            filled_style: Style::default().fg(filled_fg),
            empty_style: Style::default().fg(empty_fg),
            show_percent: self.show_percent,
        })
    }
}

impl From<ProgressBar> for UiNode {
    fn from(pb: ProgressBar) -> Self { pb.build() }
}
