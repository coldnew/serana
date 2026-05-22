//! Tool execution display component.
//!
//! Renders tool calls with their results in the TUI, matching oh-my-pi's
//! tool-execution.ts component behavior.

use crate::component::{Component, Container};
use crate::components::{Spacer, Text};
use crate::style::{Color, Style};

/// Spinner frames for running state (2fps = 500ms per frame)
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Tool execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolState {
    Pending,
    Running,
    Success,
    Error,
}

/// Diff preview for edit tools
#[derive(Debug, Clone)]
pub struct DiffPreview {
    pub path: String,
    pub diff: String,
}

/// Tool execution component that displays tool calls with results
pub struct ToolExecution {
    /// Tool name (e.g., "read", "edit", "bash")
    tool_name: String,
    /// Tool label for display (can be customized)
    tool_label: String,
    /// Current execution state
    state: ToolState,
    /// Tool arguments (for preview)
    args: Option<String>,
    /// Result output
    output: Option<String>,
    /// Diff preview for edit tools
    diff_preview: Option<DiffPreview>,
    /// Whether output is expanded
    expanded: bool,
    /// Spinner frame index
    spinner_frame: usize,
    /// Container for child components
    container: Container,
}

impl ToolExecution {
    /// Create a new tool execution component
    pub fn new(tool_name: impl Into<String>) -> Self {
        let name = tool_name.into();
        Self {
            tool_label: name.clone(),
            tool_name: name,
            state: ToolState::Pending,
            args: None,
            output: None,
            diff_preview: None,
            expanded: false,
            spinner_frame: 0,
            container: Container::new(),
        }
    }

    /// Set the tool label (display name)
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.tool_label = label.into();
        self
    }

    /// Set tool arguments for preview
    pub fn set_args(&mut self, args: impl Into<String>) {
        self.args = Some(args.into());
        self.rebuild();
    }

    /// Set the execution state
    pub fn set_state(&mut self, state: ToolState) {
        self.state = state;
        self.rebuild();
    }

    /// Set the result output
    pub fn set_output(&mut self, output: impl Into<String>) {
        self.output = Some(output.into());
        self.rebuild();
    }

    /// Set diff preview for edit tools
    pub fn set_diff_preview(&mut self, preview: DiffPreview) {
        self.diff_preview = Some(preview);
        self.rebuild();
    }

    /// Toggle expanded state
    pub fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
        self.rebuild();
    }

    /// Set expanded state
    pub fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
        self.rebuild();
    }

    /// Advance spinner frame (call at 2fps)
    pub fn advance_spinner(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
        self.rebuild();
    }

    /// Get current state
    pub fn state(&self) -> ToolState {
        self.state
    }

    /// Check if expanded
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Rebuild the component tree
    fn rebuild(&mut self) {
        self.container.clear();

        // Add spacer at top
        self.container.push(Spacer::new(1));

        // Build the status line
        let status_line = self.build_status_line();
        self.container.push(Text::new(status_line));

        // Add args preview if present (dimmed)
        if let Some(ref args) = self.args {
            if self.state == ToolState::Pending || self.state == ToolState::Running {
                let args_style = Style::new().fg(Color::BrightBlack).dim();
                let args_text = Text::styled(format!("  {}", args), args_style);
                self.container.push(args_text);
            }
        }

        // Add diff preview for edit tools
        if let Some(ref preview) = self.diff_preview {
            if self.is_edit_tool() {
                self.container.push(Spacer::new(1));
                let diff_lines = self.build_diff_lines(&preview.diff);
                for line in diff_lines {
                    self.container.push(line);
                }
            }
        }

        // Add output (collapsible)
        if let Some(ref output) = self.output {
            if self.expanded {
                self.container.push(Spacer::new(1));
                let output_lines = self.build_output_lines(output);
                for line in output_lines {
                    self.container.push(line);
                }
            } else {
                // Show collapsed hint
                let hint_style = Style::new().fg(Color::BrightBlack).dim();
                let hint = Text::styled("  [Enter to expand]", hint_style);
                self.container.push(hint);
            }
        }
    }

    /// Build the status line with icon
    fn build_status_line(&self) -> String {
        let icon = match self.state {
            ToolState::Pending => "○",
            ToolState::Running => SPINNER_FRAMES[self.spinner_frame],
            ToolState::Success => "✓",
            ToolState::Error => "✗",
        };

        let color_code = match self.state {
            ToolState::Pending => "\x1b[90m", // dim gray
            ToolState::Running => "\x1b[33m", // yellow
            ToolState::Success => "\x1b[32m", // green
            ToolState::Error => "\x1b[31m",   // red
        };

        let reset = "\x1b[0m";
        let bold = "\x1b[1m";

        // Format: icon tool_name
        format!(
            "{}{}{}{} {}{}",
            color_code, bold, icon, reset, self.tool_label, reset
        )
    }

    /// Check if this is an edit-like tool
    fn is_edit_tool(&self) -> bool {
        self.tool_name == "edit" || self.tool_name == "apply_patch"
    }

    /// Build diff lines with syntax highlighting
    fn build_diff_lines(&self, diff: &str) -> Vec<Text> {
        let mut lines = Vec::new();

        for line in diff.lines() {
            let styled = if line.starts_with('+') {
                // Addition - green
                Text::styled(line, Style::new().fg(Color::Green))
            } else if line.starts_with('-') {
                // Deletion - red
                Text::styled(line, Style::new().fg(Color::Red))
            } else if line.starts_with("@@") {
                // Hunk header - cyan
                Text::styled(line, Style::new().fg(Color::Cyan))
            } else {
                // Context line
                Text::new(line.to_string())
            };
            lines.push(styled);
        }

        lines
    }

    /// Build output lines with truncation
    fn build_output_lines(&self, output: &str) -> Vec<Text> {
        let style = match self.state {
            ToolState::Error => Style::new().fg(Color::Red),
            _ => Style::new().fg(Color::BrightBlack),
        };

        output
            .lines()
            .take(50) // Limit output lines
            .map(|line| Text::styled(format!("  {}", line), style))
            .collect()
    }
}

impl Component for ToolExecution {
    fn render(&self, width: usize) -> Vec<String> {
        self.container.render(width)
    }

    fn invalidate(&mut self) {
        self.container.invalidate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_execution_pending() {
        let mut tool = ToolExecution::new("read");
        tool.set_args("path: \"src/main.rs\"");

        let lines = tool.render(80);
        assert!(!lines.is_empty());
        assert!(lines[1].contains("read"));
    }

    #[test]
    fn test_tool_execution_running_spinner() {
        let mut tool = ToolExecution::new("bash");
        tool.set_state(ToolState::Running);

        let lines1 = tool.render(80);
        tool.advance_spinner();
        let lines2 = tool.render(80);

        // Spinner should change
        assert_ne!(lines1[1], lines2[1]);
    }

    #[test]
    fn test_tool_execution_success() {
        let mut tool = ToolExecution::new("read");
        tool.set_state(ToolState::Success);
        tool.set_output("file contents here");

        // Not expanded - should show hint
        let lines = tool.render(80);
        assert!(lines.iter().any(|l| l.contains("expand")));

        // Expanded - should show output
        tool.set_expanded(true);
        let lines = tool.render(80);
        assert!(lines.iter().any(|l| l.contains("file contents")));
    }

    #[test]
    fn test_tool_execution_error() {
        let mut tool = ToolExecution::new("bash");
        tool.set_state(ToolState::Error);
        tool.set_output("command failed");

        tool.set_expanded(true);
        let lines = tool.render(80);
        assert!(lines.iter().any(|l| l.contains("command failed")));
    }

    #[test]
    fn test_diff_preview() {
        let mut tool = ToolExecution::new("edit");
        tool.set_diff_preview(DiffPreview {
            path: "src/main.rs".to_string(),
            diff: "@@ src/main.rs\n-old line\n+new line".to_string(),
        });

        let lines = tool.render(80);
        // Should contain diff lines with colors
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
