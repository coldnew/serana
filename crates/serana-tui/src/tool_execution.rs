use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::app::{ToolCall, ToolCallStatus};
use crate::diff;
use crate::symbols::Symbols;
use crate::theme::{self, Theme};

/// Number of preview lines for command output.
const PREVIEW_LINES: usize = 10;
/// Max output lines when expanded.

/// Render a tool call with tool-specific formatting.
pub fn render_tool_call(
    tool: &ToolCall,
    symbols: &Symbols,
    width: usize,
) -> Vec<Line<'static>> {
    let theme = Theme::default();
    let (icon, style) = tool_status_style(tool.status, symbols, &theme);
    let header = tool_header(tool, icon, style, width, symbols);
    let footer = tool_footer(width, symbols, &theme);

    let mut lines = match tool.name.as_str() {
        "read_file" | "read" | "read_self" => {
            render_read_file(tool, header, width, symbols)
        }
        "edit_file" | "edit" | "edit_self" | "apply_patch" => {
            render_edit_diff(tool, header, width, symbols)
        }
        "write_file" | "write" => {
            render_write_file(tool, header, width, symbols)
        }
        "bash" | "cargo" | "git" | "verify_self" => {
            render_command(tool, header, width, symbols)
        }
        n if n.starts_with("lsp_") => {
            render_lsp(tool, header, symbols)
        }
        n if n.starts_with("ast_") => {
            render_ast(tool, header, symbols)
        }
        _ => render_generic(tool, header, &theme, symbols),
    };
    lines.push(footer);

    // Check for image output in tool results
    if let Some(ref result) = tool.result {
        if let Some(image_path) = crate::image::detect_image_in_result(result, &tool.name) {
            let protocol = crate::image::ImageProtocol::detect();
            if protocol.is_supported() {
                lines.push(Line::from(Span::styled(
                    format!("  {} Image: {}", symbols.expand, image_path),
                    Style::new().fg(theme::AQUAMARINE),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    format!("  {} Image detected: {} (terminal does not support inline images)", symbols.info, image_path),
                    theme.dim,
                )));
            }
        }
    }

    lines
        .into_iter()
        .map(|line| clamp_tool_line(line, width))
        .collect()
}

fn tool_header(
    tool: &ToolCall,
    icon: &str,
    style: Style,
    width: usize,
    symbols: &Symbols,
) -> Line<'static> {
    let label = match tool.status {
        ToolCallStatus::Pending => "pending",
        ToolCallStatus::Running => "running",
        ToolCallStatus::Success => "done",
        ToolCallStatus::Error => "error",
    };
    let title = format!(" {} {} {} ", icon, tool.name, label);
    let content_width = width.saturating_sub(4).max(title.chars().count() + 2);
    let title_width = title.chars().count();
    let fill = content_width.saturating_sub(title_width + 1);
    Line::from(Span::styled(
        format!(
            "  {}{}{}{}{}",
            symbols.box_round.top_left,
            symbols.box_round.horizontal,
            title,
            symbols.box_round.horizontal.repeat(fill),
            symbols.box_round.top_right,
        ),
        style,
    ))
}

fn tool_footer(width: usize, symbols: &Symbols, theme: &Theme) -> Line<'static> {
    let content_width = width.saturating_sub(4).max(2);
    Line::from(Span::styled(
        format!(
            "  {}{}{}",
            symbols.box_round.bottom_left,
            symbols.box_round.horizontal.repeat(content_width),
            symbols.box_round.bottom_right,
        ),
        theme.border,
    ))
}

fn tool_status_style<'a>(
    status: ToolCallStatus,
    symbols: &'a Symbols,
    theme: &Theme,
) -> (&'a str, Style) {
    match status {
        ToolCallStatus::Pending => (symbols.pending, theme.dim),
        ToolCallStatus::Running => (symbols.running, theme.accent),
        ToolCallStatus::Success => (symbols.success, theme.success),
        ToolCallStatus::Error => (symbols.error, theme.error),
    }
}

/// Render file content with line numbers.
fn render_read_file(
    tool: &ToolCall,
    header: Line<'static>,
    width: usize,
    _symbols: &Symbols,
) -> Vec<Line<'static>> {
    let theme = Theme::default();
    let mut lines = Vec::new();
    lines.push(Line::from(""));
    lines.push(header);

    let path = extract_arg(tool, "path").unwrap_or_default();
    let content = tool.result.as_deref().unwrap_or("");
    if !path.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("  File: {}", path),
            Style::default().fg(theme::DIM_TEAL),
        )));
    }

    if let Some((ref diff_path, ref diff_text)) = tool.diff_preview {
        if !diff_path.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("  Diff: {}", diff_path),
                Style::default().fg(theme::DIM_TEAL),
            )));
        }
        let diff_lines = diff::render_diff(diff_text, width.saturating_sub(4));
        for dl in diff_lines {
            let mut prefixed = vec![Span::raw("  ")];
            prefixed.extend(dl.spans);
            lines.push(Line::from(prefixed));
        }
    } else if !content.is_empty() {
        for text_line in content.lines().take(PREVIEW_LINES) {
            lines.push(Line::from(Span::styled(
                format!("  {}", text_line),
                theme.dim,
            )));
        }
        if content.lines().count() > PREVIEW_LINES {
            lines.push(Line::from(Span::styled(
                format!("  ... ({} lines total)", content.lines().count()),
                theme.dim,
            )));
        }
    }
    lines
}

/// Render edit/patch with inline diff.
fn render_edit_diff(
    tool: &ToolCall,
    header: Line<'static>,
    width: usize,
    _symbols: &Symbols,
) -> Vec<Line<'static>> {
    let theme = Theme::default();
    let mut lines = Vec::new();
    lines.push(Line::from(""));
    lines.push(header);

    let path = extract_arg(tool, "path").unwrap_or_default();
    if !path.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("  File: {}", path),
            Style::default().fg(theme::DIM_TEAL),
        )));
    }

    if let Some((ref diff_path, ref diff_text)) = tool.diff_preview {
        if !diff_path.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("  Diff: {}", diff_path),
                Style::default().fg(theme::DIM_TEAL),
            )));
        }
        let diff_lines = diff::render_diff(diff_text, width.saturating_sub(4));
        for dl in diff_lines {
            let mut prefixed = vec![Span::raw("  ")];
            prefixed.extend(dl.spans);
            lines.push(Line::from(prefixed));
        }
    } else {
        let content = tool.result.as_deref().unwrap_or("");
        for text_line in content.lines().take(PREVIEW_LINES) {
            lines.push(Line::from(Span::styled(
                format!("  {}", text_line),
                theme.dim,
            )));
        }
    }
    lines
}

/// Render write_file confirmation.
fn render_write_file(
    tool: &ToolCall,
    header: Line<'static>,
    width: usize,
    symbols: &Symbols,
) -> Vec<Line<'static>> {
    let theme = Theme::default();
    let mut lines = Vec::new();
    lines.push(Line::from(""));
    lines.push(header);

    let path = extract_arg(tool, "path").unwrap_or_default();
    let content = tool.result.as_deref().unwrap_or("");
    if !content.is_empty() {
        let h = symbols.box_sharp.horizontal;
        let file_width = width.saturating_sub(6);
        let title = if path.is_empty() { " written " } else { &format!(" {} ", path) };
        let title_len = title.len() as i32;
        let border_len = (file_width as i32).saturating_sub(title_len).max(2) as usize;
        lines.push(Line::from(Span::styled(
            format!(
                "  {}{}{}{}",
                symbols.box_sharp.tee_right,
                h.repeat(border_len / 2),
                title,
                h.repeat(border_len - border_len / 2),
            ),
            Style::from(theme::MUTED_TEAL),
        )));
        for text_line in content.lines().take(PREVIEW_LINES) {
            lines.push(Line::from(Span::styled(
                format!("  {} {}", symbols.box_sharp.vertical, text_line),
                Style::default().fg(theme::MUTED_TEAL),
            )));
        }
        if content.lines().count() > PREVIEW_LINES {
            lines.push(Line::from(Span::styled(
                format!("  {} ... ({} lines)", symbols.box_sharp.vertical, content.lines().count()),
                theme.dim,
            )));
        }
        lines.push(Line::from(Span::styled(
            format!("  {}{}", symbols.box_sharp.tee_left, h.repeat(file_width)),
            Style::from(theme::MUTED_TEAL),
        )));
    }
    lines
}

/// Render command output (bash, cargo, git, etc.).
fn render_command(
    tool: &ToolCall,
    header: Line<'static>,
    _width: usize,
    symbols: &Symbols,
) -> Vec<Line<'static>> {
    let theme = Theme::default();
    let mut lines = Vec::new();
    lines.push(Line::from(""));
    lines.push(header);

    let cmd = extract_arg(tool, "command").or_else(|| extract_arg(tool, "args"));
    if let Some(ref cmd_text) = cmd {
        lines.push(Line::from(Span::styled(
            format!("  {} {}", symbols.arrow, cmd_text),
            Style::default().fg(theme::DIM_TEAL),
        )));
    }

    let content = tool.result.as_deref().unwrap_or("");
    if !content.is_empty() {
        let status_icon = match tool.status {
            ToolCallStatus::Success => symbols.success,
            ToolCallStatus::Error => symbols.error,
            _ => symbols.info,
        };
        let status_style = match tool.status {
            ToolCallStatus::Success => theme.success,
            ToolCallStatus::Error => theme.error,
            _ => theme.dim,
        };
        let lines_count = content.lines().count();
        for text_line in content.lines().take(PREVIEW_LINES) {
            lines.push(Line::from(Span::styled(
                format!("  {}", text_line),
                theme.dim,
            )));
        }
        if lines_count > PREVIEW_LINES {
            lines.push(Line::from(Span::styled(
                format!("  ... ({} lines total, enter to expand)", lines_count),
                theme.dim,
            )));
        }
        lines.push(Line::from(Span::styled(
            format!("  {} exit: 0", status_icon),
            status_style,
        )));
    }
    lines
}

/// Render LSP results.
fn render_lsp(
    tool: &ToolCall,
    header: Line<'static>,
    symbols: &Symbols,
) -> Vec<Line<'static>> {
    let theme = Theme::default();
    let mut lines = Vec::new();
    lines.push(Line::from(""));
    lines.push(header);
    push_arg_summary(&mut lines, tool, &theme);

    let content = tool.result.as_deref().unwrap_or("");
    if content.is_empty() {
        return lines;
    }
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(content) {
        match &val {
            serde_json::Value::Array(items) => {
                for item in items {
                    let item_str = format_item(item);
                    lines.push(Line::from(Span::styled(
                        format!("  {} {}", symbols.bullet, item_str),
                        theme.dim,
                    )));
                }
            }
            serde_json::Value::Object(map) => {
                for (k, v) in map {
                    let v_str = match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {} ", symbols.bullet), theme.dim),
                        Span::styled(format!("{}: ", k), Style::default().fg(theme::DIM_TEAL)),
                        Span::styled(v_str, theme.dim),
                    ]));
                }
            }
            _ => {
                lines.push(Line::from(Span::styled(
                    format!("  {}", val),
                    theme.dim,
                )));
            }
        }
    } else {
        for text_line in content.lines().take(PREVIEW_LINES) {
            lines.push(Line::from(Span::styled(
                format!("  {}", text_line),
                theme.dim,
            )));
        }
    }
    lines
}

/// Render AST results.
fn render_ast(
    tool: &ToolCall,
    header: Line<'static>,
    symbols: &Symbols,
) -> Vec<Line<'static>> {
    let theme = Theme::default();
    let mut lines = Vec::new();
    lines.push(Line::from(""));
    lines.push(header);
    push_arg_summary(&mut lines, tool, &theme);

    let content = tool.result.as_deref().unwrap_or("");
    if content.is_empty() {
        return lines;
    }
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(content) {
        match &val {
            serde_json::Value::Object(map) => {
                for (k, v) in map {
                    let label = k.replace('_', " ");
                    let label = label[..1].to_uppercase() + &label[1..];
                    lines.push(Line::from(Span::styled(
                        format!("  {}", label),
                        Style::default().fg(theme::DIM_TEAL).add_modifier(Modifier::BOLD),
                    )));
                    if let serde_json::Value::Array(items) = v {
                        for item in items {
                            let item_str = format_item(item);
                            lines.push(Line::from(Span::styled(
                                format!("  {} {}", symbols.bullet, item_str),
                                theme.dim,
                            )));
                        }
                    } else {
                        lines.push(Line::from(Span::styled(
                            format!("  {} {}", symbols.bullet, v),
                            theme.dim,
                        )));
                    }
                }
            }
            _ => {
                lines.push(Line::from(Span::styled(
                    format!("  {}", val),
                    theme.dim,
                )));
            }
        }
    } else {
        for text_line in content.lines().take(PREVIEW_LINES) {
            lines.push(Line::from(Span::styled(
                format!("  {}", text_line),
                theme.dim,
            )));
        }
    }
    lines
}

/// Generic fallback renderer.
fn render_generic(
    tool: &ToolCall,
    header: Line<'static>,
    theme: &Theme,
    symbols: &Symbols,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(Line::from(""));
    lines.push(header);

    push_arg_summary(&mut lines, tool, theme);
    if let Some(ref result) = tool.result {
        let result_style = match tool.status {
            ToolCallStatus::Error => theme.error,
            _ => theme.dim,
        };
        let result_lines = result.lines().count();
        let status_icon = match tool.status {
            ToolCallStatus::Error => symbols.error,
            _ => symbols.success,
        };
        if result_lines == 0 {
            lines.push(Line::from(Span::styled(
                format!("  {} done", status_icon),
                result_style,
            )));
        } else if result_lines == 1 && result.len() < 80 {
            lines.push(Line::from(Span::styled(
                format!("  {} {}", status_icon, result.trim()),
                result_style,
            )));
        } else {
            lines.push(Line::from(Span::styled(
                format!("  {} {} lines", status_icon, result_lines),
                result_style,
            )));
            for text_line in result.lines().take(PREVIEW_LINES) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", text_line),
                    result_style,
                )));
            }
        }
        if result_lines > PREVIEW_LINES {
            lines.push(Line::from(Span::styled(
                format!("    ... ({} more)", result_lines - PREVIEW_LINES),
                theme.dim,
            )));
        }
    }
    lines
}

fn push_arg_summary(lines: &mut Vec<Line<'static>>, tool: &ToolCall, theme: &Theme) {
    if let Some(summary) = format_tool_args(tool) {
        lines.push(Line::from(Span::styled(
            format!("  {}", truncate(&summary, 120)),
            theme.dim,
        )));
    }
}

fn format_tool_args(tool: &ToolCall) -> Option<String> {
    let val = serde_json::from_str::<serde_json::Value>(&tool.args).ok()?;
    let map = val.as_object()?;
    let get_str = |name: &str| map.get(name).and_then(|v| v.as_str()).map(str::to_string);
    let get_array = |name: &str| {
        map.get(name).and_then(|v| {
            v.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
        })
    };

    let summary = match tool.name.as_str() {
        "read" | "read_file" | "read_self" | "write" | "write_file" | "edit" | "edit_file"
        | "edit_self" | "apply_patch" => get_str("path").map(|path| format!("path: {}", path)),
        "bash" | "cargo" | "git" | "verify_self" => get_str("command").or_else(|| get_str("args")),
        "search" | "grep" | "ripgrep" => {
            let mut parts = Vec::new();
            if let Some(pattern) = get_str("pattern") {
                parts.push(format!("pattern: {}", pattern));
            }
            if let Some(paths) = get_array("paths").filter(|paths| !paths.is_empty()) {
                parts.push(format!("paths: {}", paths.join(", ")));
            }
            Some(parts.join(", ")).filter(|s| !s.is_empty())
        }
        name if name.starts_with("lsp_") => {
            let parts = ["action", "file", "path", "symbol", "query"]
                .into_iter()
                .filter_map(get_str)
                .collect::<Vec<_>>();
            Some(parts.join(" ")).filter(|s| !s.is_empty())
        }
        name if name.starts_with("ast_") => get_str("path")
            .or_else(|| get_str("file"))
            .map(|path| format!("path: {}", path)),
        "task" | "todo" | "todo_write" => map.get("tasks").and_then(|tasks| {
            tasks
                .as_array()
                .map(|items| format!("{} task(s)", items.len()))
        }),
        _ => {
            let mut parts = Vec::new();
            let mut total = 0;
            for (key, value) in map {
                if key.starts_with('_') {
                    continue;
                }
                let value = match value {
                    serde_json::Value::String(s) => s.replace('\t', "   "),
                    other => other.to_string(),
                };
                let entry = format!("{}: {}", key, value);
                if total + entry.chars().count() > 160 {
                    break;
                }
                total += entry.chars().count();
                parts.push(entry);
            }
            Some(parts.join(", ")).filter(|s| !s.is_empty())
        }
    };

    summary.filter(|s| !s.is_empty())
}

/// Extract a named argument from the tool's args JSON.
fn extract_arg(tool: &ToolCall, name: &str) -> Option<String> {
    if tool.args.is_empty() {
        return None;
    }
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&tool.args) {
        if let Some(v) = val.get(name) {
            if let Some(s) = v.as_str() {
                return Some(s.to_string());
            }
        }
    }
    None
}

/// Format a JSON value as a concise string.
fn format_item(item: &serde_json::Value) -> String {
    match item {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(map) => {
            let name = map
                .get("name")
                .or_else(|| map.get("label"))
                .or_else(|| map.get("id"))
                .and_then(|v| v.as_str());
            let detail = map
                .get("signature")
                .or_else(|| map.get("detail"))
                .or_else(|| map.get("kind"))
                .and_then(|v| v.as_str());
            match (name, detail) {
                (Some(n), Some(d)) => format!("{} ({})", n, d),
                (Some(n), None) => n.to_string(),
                (None, Some(d)) => d.to_string(),
                (None, None) => serde_json::to_string(item).unwrap_or_default(),
            }
        }
        serde_json::Value::Array(arr) => {
            arr.iter()
                .map(format_item)
                .collect::<Vec<_>>()
                .join(", ")
        }
        other => other.to_string(),
    }
}

/// Truncate a string to fit within max_len.
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max_len.saturating_sub(1)).collect::<String>())
    }
}

fn clamp_tool_line(line: Line<'static>, width: usize) -> Line<'static> {
    if width == 0 {
        return Line::from("");
    }

    let mut remaining = width;
    let mut spans = Vec::new();
    for span in line.spans {
        let content = span.content.into_owned();
        let len = content.chars().count();
        if len <= remaining {
            remaining -= len;
            spans.push(Span::styled(content, span.style));
            continue;
        }

        if remaining == 0 {
            break;
        }
        spans.push(Span::styled(truncate(&content, remaining), span.style));
        break;
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{ToolCall, ToolCallStatus};
    use crate::symbols;

    fn make_tool(name: &str, status: ToolCallStatus, args: &str, result: Option<&str>) -> ToolCall {
        ToolCall {
            name: name.to_string(),
            args: args.to_string(),
            result: result.map(|s| s.to_string()),
            status,
            diff_preview: None,
        }
    }

    #[test]
    fn test_render_read_file() {
        let tool = make_tool(
            "read_file",
            ToolCallStatus::Success,
            r#"{"path":"src/main.rs"}"#,
            Some("fn main() {\n    println!(\"hello\");\n}"),
        );
        let lines = render_tool_call(&tool, &symbols::UNICODE, 80);
        assert!(lines.iter().any(|l| l.to_string().contains("src/main.rs")));
        assert!(lines.iter().any(|l| l.to_string().contains("fn main()")));
    }

    #[test]
    fn test_render_edit_diff() {
        let mut tool = make_tool(
            "edit_file",
            ToolCallStatus::Success,
            r#"{"path":"src/main.rs"}"#,
            Some("applied edits"),
        );
        tool.diff_preview = Some((
            "src/main.rs".to_string(),
            "@@ src/main.rs\n-old\n+new".to_string(),
        ));
        let lines = render_tool_call(&tool, &symbols::UNICODE, 80);
        assert!(lines.len() > 2);
    }

    #[test]
    fn test_render_bash() {
        let tool = make_tool(
            "bash",
            ToolCallStatus::Success,
            r#"{"command":"cargo test"}"#,
            Some("running 1 test\ntest result: ok"),
        );
        let lines = render_tool_call(&tool, &symbols::UNICODE, 80);
        assert!(lines.iter().any(|l| l.to_string().contains("cargo test")));
    }

    #[test]
    fn test_render_lsp() {
        let tool = make_tool(
            "lsp_definition",
            ToolCallStatus::Success,
            r#"{}"#,
            Some(r#"[{"name":"main","kind":"function","signature":"fn main()"}]"#),
        );
        let lines = render_tool_call(&tool, &symbols::UNICODE, 80);
        assert!(lines.iter().any(|l| l.to_string().contains("main")));
    }

    #[test]
    fn test_render_generic() {
        let tool = make_tool(
            "unknown_tool",
            ToolCallStatus::Success,
            r#"{"key":"val"}"#,
            Some("result line"),
        );
        let lines = render_tool_call(&tool, &symbols::UNICODE, 80);
        assert!(lines.iter().any(|l| l.to_string().contains("result line")));
    }

    #[test]
    fn test_render_error() {
        let tool = make_tool(
            "bash",
            ToolCallStatus::Error,
            r#"{"command":"false"}"#,
            Some("command failed with exit code 1"),
        );
        let lines = render_tool_call(&tool, &symbols::UNICODE, 80);
        assert!(lines.iter().any(|l| l.to_string().contains("command failed")));
    }

    #[test]
    fn test_generic_args_are_summarized() {
        let tool = make_tool(
            "custom_tool",
            ToolCallStatus::Success,
            r#"{"path":"src/main.rs","_internal":true,"count":2}"#,
            Some("ok"),
        );
        let lines = render_tool_call(&tool, &symbols::UNICODE, 80);
        let rendered: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
        assert!(rendered.iter().any(|l| l.contains("path: src/main.rs")));
        assert!(!rendered.iter().any(|l| l.contains("_internal")));
    }

    #[test]
    fn test_lsp_and_ast_args_are_summarized() {
        let lsp = make_tool(
            "lsp_definition",
            ToolCallStatus::Success,
            r#"{"file":"src/main.rs","symbol":"main"}"#,
            Some("[]"),
        );
        let ast = make_tool(
            "ast_search",
            ToolCallStatus::Success,
            r#"{"path":"src/lib.rs"}"#,
            Some("{}"),
        );
        let lsp_lines = render_tool_call(&lsp, &symbols::UNICODE, 80);
        let ast_lines = render_tool_call(&ast, &symbols::UNICODE, 80);
        assert!(lsp_lines
            .iter()
            .any(|l| l.to_string().contains("src/main.rs main")));
        assert!(ast_lines
            .iter()
            .any(|l| l.to_string().contains("path: src/lib.rs")));
    }

    #[test]
    fn test_generic_result_summary_counts_lines() {
        let tool = make_tool(
            "custom_tool",
            ToolCallStatus::Success,
            r#"{}"#,
            Some("one\ntwo\nthree"),
        );
        let lines = render_tool_call(&tool, &symbols::UNICODE, 80);
        assert!(lines.iter().any(|l| l.to_string().contains("3 lines")));
    }

    #[test]
    fn test_tool_lines_fit_requested_width() {
        let tool = make_tool(
            "bash",
            ToolCallStatus::Success,
            r#"{"command":"cargo test --workspace --all-features --very-long-flag-name"}"#,
            Some("this is a very long output line that should be truncated by the tool renderer"),
        );
        let lines = render_tool_call(&tool, &symbols::UNICODE, 32);
        assert!(lines
            .iter()
            .all(|line| line.to_string().chars().count() <= 32));
    }

    #[test]
    fn test_tool_line_clamp_preserves_prefix_style() {
        let line = Line::from(vec![
            Span::styled("  ", Style::new().fg(theme::AQUAMARINE)),
            Span::styled("very long content", Style::new().fg(theme::DIM_TEAL)),
        ]);
        let clamped = clamp_tool_line(line, 8);
        assert_eq!(clamped.to_string().chars().count(), 8);
        assert_eq!(clamped.spans[0].style.fg, Some(theme::AQUAMARINE));
    }
}
