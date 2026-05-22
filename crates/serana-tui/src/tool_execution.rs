use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::theme::{self, Theme};

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolState {
    Pending,
    Running,
    Success,
    Error,
}

#[derive(Debug, Clone)]
pub struct DiffPreview {
    pub path: String,
    pub diff: String,
}

pub struct ToolExecution {
    tool_name: String,
    state: ToolState,
    args: Option<String>,
    output: Option<String>,
    diff_preview: Option<DiffPreview>,
    expanded: bool,
    spinner_frame: usize,
}

impl ToolExecution {
    pub fn new(tool_name: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            state: ToolState::Pending,
            args: None,
            output: None,
            diff_preview: None,
            expanded: false,
            spinner_frame: 0,
        }
    }

    pub fn with_label(self, _label: impl Into<String>) -> Self {
        self
    }

    pub fn set_state(&mut self, state: ToolState) {
        self.state = state;
    }

    pub fn set_args(&mut self, args: impl Into<String>) {
        self.args = Some(args.into());
    }

    pub fn set_output(&mut self, output: impl Into<String>) {
        self.output = Some(output.into());
    }

    pub fn set_diff_preview(&mut self, preview: DiffPreview) {
        self.diff_preview = Some(preview);
    }

    pub fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
    }

    pub fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
    }

    pub fn advance_spinner(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
    }

    pub fn state(&self) -> ToolState {
        self.state
    }

    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    pub fn render_lines(&self, _width: usize) -> Vec<Line<'static>> {
        let theme = Theme::default();
        let mut lines = Vec::new();

        lines.push(Line::from(""));

        let icon = match self.state {
            ToolState::Pending => "○",
            ToolState::Running => SPINNER_FRAMES[self.spinner_frame],
            ToolState::Success => "✔",
            ToolState::Error => "✘",
        };

        let tool_style = match self.state {
            ToolState::Pending => theme.dim,
            ToolState::Running => theme.accent,
            ToolState::Success => theme.success,
            ToolState::Error => theme.error,
        };

        lines.push(Line::from(Span::styled(
            format!("  {} {}", icon, self.tool_name),
            tool_style,
        )));

        if let Some(ref args) = self.args {
            if self.state == ToolState::Pending || self.state == ToolState::Running {
                lines.push(Line::from(Span::styled(
                    format!("  {}", args),
                    Style::default().fg(theme::MUTED_TEAL),
                )));
            }
        }

        if let Some(ref preview) = self.diff_preview {
            if self.is_edit_tool() {
                for diff_line in preview.diff.lines() {
                    let diff_style = if diff_line.starts_with('+') {
                        Style::new().fg(theme::SEAFOAM_GREEN)
                    } else if diff_line.starts_with('-') {
                        Style::new().fg(theme::BRIGHT_CORAL)
                    } else if diff_line.starts_with("@@") {
                        Style::new().fg(theme::AQUAMARINE)
                    } else {
                        Style::default()
                    };
                    lines.push(Line::from(Span::styled(
                        diff_line.to_string(),
                        diff_style,
                    )));
                }
            }
        }

        if let Some(ref output) = self.output {
            if self.expanded {
                let out_style = match self.state {
                    ToolState::Error => Style::new().fg(theme::BRIGHT_CORAL),
                    _ => Style::new().fg(theme::MUTED_TEAL),
                };
                for out_line in output.lines().take(50) {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", out_line),
                        out_style,
                    )));
                }
            } else {
                lines.push(Line::from(Span::styled(
                    "  [Enter to expand]",
                    Style::new().fg(theme::MUTED_TEAL),
                )));
            }
        }

        lines
    }

    fn is_edit_tool(&self) -> bool {
        self.tool_name == "edit" || self.tool_name == "apply_patch"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_execution_pending() {
        let tool = ToolExecution::new("read");
        let lines = tool.render_lines(80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_tool_execution_success() {
        let mut tool = ToolExecution::new("read");
        tool.set_state(ToolState::Success);
        tool.set_output("file contents here");
        let lines = tool.render_lines(80);
        assert!(lines.iter().any(|l| l.to_string().contains("expand")));

        tool.set_expanded(true);
        let lines = tool.render_lines(80);
        assert!(lines.iter().any(|l| l.to_string().contains("file contents")));
    }

    #[test]
    fn test_tool_execution_error() {
        let mut tool = ToolExecution::new("bash");
        tool.set_state(ToolState::Error);
        tool.set_output("command failed");
        tool.set_expanded(true);
        let lines = tool.render_lines(80);
        assert!(lines.iter().any(|l| l.to_string().contains("command failed")));
    }

    #[test]
    fn test_diff_preview() {
        let mut tool = ToolExecution::new("edit");
        tool.set_diff_preview(DiffPreview {
            path: "src/main.rs".to_string(),
            diff: "@@ src/main.rs\n-old line\n+new line".to_string(),
        });
        let lines = tool.render_lines(80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_toggle_expanded() {
        let mut tool = ToolExecution::new("read");
        tool.set_output("test");
        assert!(!tool.is_expanded());
        tool.toggle_expanded();
        assert!(tool.is_expanded());
        tool.toggle_expanded();
        assert!(!tool.is_expanded());
    }
}
