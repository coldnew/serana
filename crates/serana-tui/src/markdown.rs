use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::syntax::SyntaxHighlighter;
use crate::theme;

pub struct MarkdownTheme {
    pub heading: Style,
    pub link: Style,
    pub code: Style,
    pub code_block: Style,
    pub quote: Style,
    pub list_bullet: Style,
    pub bold: Style,
    pub italic: Style,
    pub strikethrough: Style,
    pub hr: Style,
}

impl Default for MarkdownTheme {
    fn default() -> Self {
        Self {
            heading: Style::new()
                .fg(theme::CORAL)
                .add_modifier(ratatui::style::Modifier::BOLD),
            link: Style::new().fg(theme::AQUAMARINE),
            code: Style::new().fg(theme::CODE_PURPLE),
            code_block: Style::new().fg(theme::SEAFOAM_GREEN),
            quote: Style::new()
                .fg(theme::MUTED_TEAL)
                .add_modifier(ratatui::style::Modifier::ITALIC),
            list_bullet: Style::new().fg(theme::CORAL),
            bold: Style::new().add_modifier(ratatui::style::Modifier::BOLD),
            italic: Style::new().add_modifier(ratatui::style::Modifier::ITALIC),
            strikethrough: Style::new().add_modifier(ratatui::style::Modifier::CROSSED_OUT),
            hr: Style::new().fg(theme::DARK_BORDER),
        }
    }
}

pub fn render_markdown(text: &str, theme: &MarkdownTheme, width: usize) -> Vec<Line<'static>> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    let normalized = text.replace('\t', "   ");
    let parser = Parser::new_ext(
        &normalized,
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH,
    );
    let renderer = MarkdownRenderer::new(theme, width);
    renderer.render_events(parser)
}

struct MarkdownRenderer<'a> {
    theme: &'a MarkdownTheme,
    width: usize,
    lines: Vec<Line<'static>>,
    current_spans: Vec<Span<'static>>,
    link_target: Option<String>,
    link_text: String,
    image_target: Option<String>,
    image_alt: String,
    style_stack: Vec<Style>,
    current_style: Option<Style>,
    in_code_block: bool,
    code_block_lang: String,
    code_block_lines: Vec<String>,
    code_block_indent: usize,
    in_blockquote: bool,
    in_list: bool,
    list_ordered: bool,
    list_counter: usize,
    list_depth: usize,
    in_table: bool,
    in_table_cell: bool,
    current_table: Vec<Vec<String>>,
    current_table_row: Vec<String>,
    current_table_cell: String,
}

impl<'a> MarkdownRenderer<'a> {
    fn new(theme: &'a MarkdownTheme, width: usize) -> Self {
        Self {
            theme,
            width,
            lines: Vec::new(),
            current_spans: Vec::new(),
            link_target: None,
            link_text: String::new(),
            image_target: None,
            image_alt: String::new(),
            style_stack: Vec::new(),
            current_style: None,
            in_code_block: false,
            code_block_lang: String::new(),
            code_block_lines: Vec::new(),
            code_block_indent: 2,
            in_blockquote: false,
            in_list: false,
            list_ordered: false,
            list_counter: 0,
            list_depth: 0,
            in_table: false,
            in_table_cell: false,
            current_table: Vec::new(),
            current_table_row: Vec::new(),
            current_table_cell: String::new(),
        }
    }

    fn render_events(mut self, parser: Parser<'_>) -> Vec<Line<'static>> {
        for event in parser {
            self.handle_event(event);
        }
        self.flush_line();
        self.lines
    }

    fn handle_event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.handle_start_tag(tag),
            Event::End(tag) => self.handle_end_tag(tag),
            Event::Text(text) => {
                if self.in_code_block {
                    self.code_block_lines.push(text.to_string());
                } else if self.image_target.is_some() {
                    self.image_alt.push_str(&text);
                } else if self.in_table_cell {
                    self.current_table_cell.push_str(&text);
                } else if self.link_target.is_some() {
                    self.link_text.push_str(&text);
                    self.push_text(&text);
                } else {
                    self.push_text(&text);
                }
            }
            Event::Code(code) => {
                if self.in_table_cell {
                    self.current_table_cell.push_str(&code);
                } else if self.image_target.is_some() {
                    self.image_alt.push_str(&code);
                } else if self.link_target.is_some() {
                    self.link_text.push_str(&code);
                    self.current_spans
                        .push(Span::styled(code.to_string(), self.theme.code));
                } else {
                    self.current_spans
                        .push(Span::styled(code.to_string(), self.theme.code));
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if self.in_table_cell {
                    self.current_table_cell.push(' ');
                } else {
                    self.flush_line();
                }
            }
            Event::Rule => {
                let n = self.width.min(80);
                self.flush_line();
                self.lines.push(Line::from(Span::styled(
                    "─".repeat(n),
                    self.theme.hr,
                )));
            }
            Event::TaskListMarker(checked) => {
                let marker = if checked { "✓ " } else { "○ " };
                self.current_spans
                    .push(Span::styled(marker.to_string(), self.theme.list_bullet));
            }
            Event::Html(_) | Event::InlineHtml(_) => {}
            Event::FootnoteReference(_) | Event::InlineMath(_) | Event::DisplayMath(_) => {}
        }
    }

    fn handle_start_tag(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => {}
            Tag::Heading { level, .. } => {
                self.current_style = Some(self.theme.heading);
                if level != HeadingLevel::H1 {
                    let prefix = "#".repeat(level as usize) + " ";
                    self.current_spans
                        .push(Span::styled(prefix.to_string(), self.theme.heading));
                }
            }
            Tag::BlockQuote(_) => {
                self.in_blockquote = true;
                self.current_style = Some(self.theme.quote);
            }
            Tag::CodeBlock(kind) => {
                self.in_code_block = true;
                self.code_block_lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(l) => l.to_string(),
                    _ => String::new(),
                };
                self.code_block_lines.clear();
                let fence = if self.code_block_lang.is_empty() {
                    "```".to_string()
                } else {
                    format!("```{}", self.code_block_lang)
                };
                self.lines
                    .push(Line::from(Span::styled(fence, self.theme.code_block)));
            }
            Tag::List(ordered) => {
                if self.in_list {
                    self.list_depth += 1;
                }
                self.in_list = true;
                self.list_ordered = ordered.is_some();
                self.list_counter = ordered.unwrap_or(1).saturating_sub(1) as usize;
            }
            Tag::Item => {
                self.list_counter += 1;
                let indent = "  ".repeat(self.list_depth);
                let bullet = if self.list_ordered {
                    format!("{}. ", self.list_counter)
                } else {
                    "- ".to_string()
                };
                self.current_spans.push(Span::raw(indent));
                self.current_spans
                    .push(Span::styled(bullet.to_string(), self.theme.list_bullet));
            }
            Tag::Emphasis => {
                self.style_stack.push(self.theme.italic);
                self.current_style = Some(self.theme.italic);
            }
            Tag::Strong => {
                self.style_stack.push(self.theme.bold);
                self.current_style = Some(self.theme.bold);
            }
            Tag::Strikethrough => {
                self.style_stack.push(self.theme.strikethrough);
                self.current_style = Some(self.theme.strikethrough);
            }
            Tag::Link { dest_url, .. } => {
                self.link_target = Some(dest_url.to_string());
                self.link_text.clear();
                self.style_stack.push(self.theme.link);
                self.current_style = Some(self.theme.link);
            }
            Tag::Image { dest_url, .. } => {
                self.image_target = Some(dest_url.to_string());
                self.image_alt.clear();
            }
            Tag::Table(_) => {
                self.flush_line();
                self.in_table = true;
                self.current_table.clear();
            }
            Tag::TableHead => {}
            Tag::TableRow => {
                if self.in_table {
                    self.current_table_row.clear();
                }
            }
            Tag::TableCell => {
                if self.in_table {
                    self.in_table_cell = true;
                    self.current_table_cell.clear();
                }
            }
            Tag::FootnoteDefinition(_)
            | Tag::MetadataBlock(_)
            | Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
            | Tag::HtmlBlock => {}
        }
    }

    fn handle_end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                self.flush_line();
                self.lines.push(Line::from(""));
            }
            TagEnd::Heading { .. } => {
                self.flush_line();
                self.current_style = None;
                self.lines.push(Line::from(""));
            }
            TagEnd::BlockQuote(_) => {
                self.in_blockquote = false;
                self.current_style = None;
                self.flush_line();
            }
            TagEnd::CodeBlock => {
                self.in_code_block = false;
                let indent = " ".repeat(self.code_block_indent);
                let code = self.code_block_lines.join("");
                let has_lang = !self.code_block_lang.is_empty();
                let mut rendered_lines: Vec<Vec<Span<'static>>> = Vec::new();
                if has_lang {
                    let highlighted = SyntaxHighlighter::global()
                        .highlight_lines(&code, &self.code_block_lang);
                    if !highlighted.is_empty() {
                        rendered_lines = highlighted;
                    }
                }
                if rendered_lines.is_empty() {
                    for line in code.lines() {
                        let display = if line.is_empty() {
                            String::new()
                        } else {
                            format!("{}{}", indent, line)
                        };
                        rendered_lines.push(vec![Span::styled(
                            display,
                            self.theme.code_block,
                        )]);
                    }
                } else {
                    for spans in &mut rendered_lines {
                        if !spans.is_empty() {
                            spans.insert(0, Span::styled(indent.clone(), self.theme.code_block));
                        }
                    }
                }
                for spans in rendered_lines {
                    self.lines.push(Line::from(spans));
                }
                self.code_block_lines.clear();
                self.code_block_lang.clear();
                self.lines
                    .push(Line::from(Span::styled("```", self.theme.code_block)));
                self.lines.push(Line::from(""));
            }
            TagEnd::List(_) => {
                if self.list_depth > 0 {
                    self.list_depth -= 1;
                } else {
                    self.in_list = false;
                }
            }
            TagEnd::Item => {
                self.flush_line();
            }
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough => {
                self.style_stack.pop();
                self.current_style = self.style_stack.last().copied();
            }
            TagEnd::Link => {
                if let Some(target) = self.link_target.take() {
                    let text = std::mem::take(&mut self.link_text);
                    if !target.is_empty() && !link_text_matches_target(&text, &target) {
                        self.current_spans
                            .push(Span::styled(format!(" ({})", target), self.theme.link));
                    }
                }
                self.style_stack.pop();
                self.current_style = self.style_stack.last().copied();
            }
            TagEnd::Image => {
                if let Some(target) = self.image_target.take() {
                    let alt = std::mem::take(&mut self.image_alt);
                    self.current_spans.push(Span::styled(
                        format_image_placeholder(&alt, &target, self.width),
                        self.theme.link,
                    ));
                }
            }
            TagEnd::Table => {
                self.in_table = false;
                let table = std::mem::take(&mut self.current_table);
                self.lines.extend(render_table_lines(table, self.width, self.theme));
                self.lines.push(Line::from(""));
            }
            TagEnd::TableHead => {
                if self.in_table && !self.current_table_row.is_empty() {
                    self.current_table
                        .push(std::mem::take(&mut self.current_table_row));
                }
            }
            TagEnd::TableRow => {
                if self.in_table && !self.current_table_row.is_empty() {
                    self.current_table
                        .push(std::mem::take(&mut self.current_table_row));
                }
            }
            TagEnd::TableCell => {
                if self.in_table_cell {
                    self.in_table_cell = false;
                    self.current_table_row
                        .push(self.current_table_cell.trim().to_string());
                    self.current_table_cell.clear();
                }
            }
            TagEnd::FootnoteDefinition
            | TagEnd::MetadataBlock(_)
            | TagEnd::DefinitionList
            | TagEnd::DefinitionListTitle
            | TagEnd::DefinitionListDefinition
            | TagEnd::HtmlBlock => {}
        }
    }

    fn push_text(&mut self, text: &str) {
        if let Some(style) = self.current_style {
            self.push_styled(text, style);
        } else {
            self.current_spans.push(Span::raw(text.to_string()));
        }
    }

    fn push_styled(&mut self, text: &str, style: Style) {
        if !text.is_empty() {
            self.current_spans
                .push(Span::styled(text.to_string(), style));
        }
    }

    fn flush_line(&mut self) {
        if !self.current_spans.is_empty() {
            let mut spans = std::mem::take(&mut self.current_spans);
            let continuation_prefix = if self.in_list {
                list_continuation_prefix(&spans)
            } else {
                Vec::new()
            };
            if self.in_blockquote {
                let mut quoted = vec![Span::styled("│ ".to_string(), self.theme.quote)];
                quoted.append(&mut spans);
                let mut quoted_prefix = vec![Span::styled("│ ".to_string(), self.theme.quote)];
                quoted_prefix.extend(continuation_prefix);
                self.lines
                    .extend(wrap_spans(quoted, self.width, quoted_prefix));
            } else {
                self.lines
                    .extend(wrap_spans(spans, self.width, continuation_prefix));
            }
        }
    }
}

fn list_continuation_prefix(spans: &[Span<'static>]) -> Vec<Span<'static>> {
    let mut width = 0;
    for span in spans {
        let text = span.content.as_ref();
        if text.trim().is_empty() {
            width += text.chars().count();
            continue;
        }
        if text.ends_with(". ") || text == "- " || text == "✓ " || text == "○ " {
            width += text.chars().count();
            break;
        }
        break;
    }
    if width == 0 {
        Vec::new()
    } else {
        vec![Span::raw(" ".repeat(width))]
    }
}

fn wrap_spans(
    spans: Vec<Span<'static>>,
    width: usize,
    continuation_prefix: Vec<Span<'static>>,
) -> Vec<Line<'static>> {
    if width == 0 {
        return vec![Line::from("")];
    }

    let continuation_prefix = clamp_spans(continuation_prefix, width.saturating_sub(1));
    let continuation_width = spans_width(&continuation_prefix);
    let mut lines = Vec::new();
    let mut current = Vec::new();
    let mut current_width = 0;

    for span in spans {
        let style = span.style;
        let mut text = span.content.to_string();
        while !text.is_empty() {
            let remaining = width.saturating_sub(current_width);
            if remaining == 0 {
                lines.push(Line::from(std::mem::take(&mut current)));
                current_width = 0;
                if !continuation_prefix.is_empty() {
                    current.extend(clone_spans(&continuation_prefix));
                    current_width = continuation_width;
                }
                continue;
            }

            let text_width = text.chars().count();
            if text_width <= remaining {
                current.push(Span::styled(text, style));
                current_width += text_width;
                break;
            }

            let (head, tail) = split_at_char_width(&text, remaining);
            if !head.is_empty() {
                current.push(Span::styled(head, style));
            }
            lines.push(Line::from(std::mem::take(&mut current)));
            current_width = 0;
            text = tail;
            if !continuation_prefix.is_empty() {
                current.extend(clone_spans(&continuation_prefix));
                current_width = continuation_width;
            }
        }
    }

    if !current.is_empty() || lines.is_empty() {
        lines.push(Line::from(current));
    }
    lines
}

fn clone_spans(spans: &[Span<'static>]) -> Vec<Span<'static>> {
    spans
        .iter()
        .map(|span| Span::styled(span.content.to_string(), span.style))
        .collect()
}

fn clamp_spans(spans: Vec<Span<'static>>, width: usize) -> Vec<Span<'static>> {
    let mut remaining = width;
    let mut out = Vec::new();
    for span in spans {
        if remaining == 0 {
            break;
        }
        let text = span.content.to_string();
        let text_width = text.chars().count();
        if text_width <= remaining {
            out.push(Span::styled(text, span.style));
            remaining -= text_width;
        } else {
            let (head, _) = split_at_char_width(&text, remaining);
            out.push(Span::styled(head, span.style));
            remaining = 0;
        }
    }
    out
}

fn spans_width(spans: &[Span<'static>]) -> usize {
    spans
        .iter()
        .map(|span| span.content.chars().count())
        .sum()
}

fn split_at_char_width(text: &str, width: usize) -> (String, String) {
    let split = text
        .char_indices()
        .nth(width)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len());
    (text[..split].to_string(), text[split..].to_string())
}

fn render_table_lines(
    rows: Vec<Vec<String>>,
    width: usize,
    theme: &MarkdownTheme,
) -> Vec<Line<'static>> {
    if width < 5 || rows.is_empty() {
        return Vec::new();
    }

    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    if column_count == 0 {
        return Vec::new();
    }

    let overhead = column_count.saturating_mul(3).saturating_add(1);
    if width <= overhead {
        return rows
            .into_iter()
            .map(|row| {
                Line::from(Span::styled(
                    truncate_to_width(&row.join(" | "), width),
                    theme.code_block,
                ))
            })
            .collect();
    }

    let cell_budget = width - overhead;
    let mut column_widths = vec![cell_budget / column_count; column_count];
    for width in column_widths.iter_mut().take(cell_budget % column_count) {
        *width += 1;
    }

    let mut lines = Vec::new();
    lines.push(table_border_line("╭", "┬", "╮", &column_widths, theme));
    lines.push(table_row_line(
        rows.first().map(Vec::as_slice).unwrap_or(&[]),
        &column_widths,
        theme,
        theme.bold,
    ));
    lines.push(table_border_line("├", "┼", "┤", &column_widths, theme));
    for row in rows.iter().skip(1) {
        lines.push(table_row_line(row, &column_widths, theme, Style::default()));
    }
    lines.push(table_border_line("╰", "┴", "╯", &column_widths, theme));
    lines
}

fn table_border_line(
    left: &str,
    separator: &str,
    right: &str,
    widths: &[usize],
    theme: &MarkdownTheme,
) -> Line<'static> {
    let cells = widths
        .iter()
        .map(|width| "─".repeat(*width + 2))
        .collect::<Vec<_>>()
        .join(separator);
    Line::from(Span::styled(
        format!("{}{}{}", left, cells, right),
        theme.hr,
    ))
}

fn table_row_line(
    row: &[String],
    widths: &[usize],
    theme: &MarkdownTheme,
    style: Style,
) -> Line<'static> {
    let mut spans = Vec::new();
    spans.push(Span::styled("│".to_string(), theme.hr));
    for (idx, width) in widths.iter().enumerate() {
        let cell = row.get(idx).map(String::as_str).unwrap_or("");
        let text = truncate_to_width(cell, *width);
        let pad = " ".repeat(width.saturating_sub(text.chars().count()));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(text, style));
        spans.push(Span::raw(pad));
        spans.push(Span::raw(" "));
        spans.push(Span::styled("│".to_string(), theme.hr));
    }
    Line::from(spans)
}

fn truncate_to_width(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    if width <= 1 {
        return "…".chars().take(width).collect();
    }
    let mut out: String = text.chars().take(width - 1).collect();
    out.push('…');
    out
}

fn format_image_placeholder(alt: &str, target: &str, width: usize) -> String {
    let alt = alt.trim();
    let label = if alt.is_empty() {
        "image".to_string()
    } else {
        alt.to_string()
    };
    truncate_to_width(&format!("[image: {}] {}", label, target), width)
}

fn link_text_matches_target(text: &str, target: &str) -> bool {
    let text = text.trim();
    text == target || target.strip_prefix("mailto:") == Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading() {
        let theme = MarkdownTheme::default();
        let lines = render_markdown("# Hello World", &theme, 80);
        assert!(!lines.is_empty());
        assert!(lines.iter().any(|l| {
            let s = l.clone().to_string();
            s.contains("Hello World")
        }));
    }

    #[test]
    fn test_code_block() {
        let theme = MarkdownTheme::default();
        let lines = render_markdown("```rust\nfn main() {}\n```", &theme, 80);
        assert!(lines.iter().any(|l| {
            let s = l.clone().to_string();
            s.contains("fn main()")
        }));
    }

    #[test]
    fn test_bold_italic() {
        let theme = MarkdownTheme::default();
        let lines = render_markdown("**bold** and *italic*", &theme, 80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_strikethrough() {
        let theme = MarkdownTheme::default();
        let lines = render_markdown("~~removed~~", &theme, 80);
        assert!(lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .any(|span| span.content == "removed"
                && span
                    .style
                    .add_modifier
                    .contains(ratatui::style::Modifier::CROSSED_OUT)));
    }

    #[test]
    fn test_inline_code() {
        let theme = MarkdownTheme::default();
        let lines = render_markdown("Use `cargo run`", &theme, 80);
        assert!(lines.iter().any(|l| {
            let s = l.clone().to_string();
            s.contains("cargo run")
        }));
    }

    #[test]
    fn test_link_omits_duplicate_target() {
        let theme = MarkdownTheme::default();
        let lines = render_markdown("[https://example.com](https://example.com)", &theme, 80);
        let rendered = lines
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("https://example.com"));
        assert!(!rendered.contains("(https://example.com)"));
    }

    #[test]
    fn test_mailto_link_omits_duplicate_target() {
        let theme = MarkdownTheme::default();
        let lines = render_markdown("[me@example.com](mailto:me@example.com)", &theme, 80);
        let rendered = lines
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("me@example.com"));
        assert!(!rendered.contains("(mailto:me@example.com)"));
    }

    #[test]
    fn test_link_keeps_descriptive_target() {
        let theme = MarkdownTheme::default();
        let lines = render_markdown("[example](https://example.com)", &theme, 80);
        let rendered = lines
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("example"));
        assert!(rendered.contains("(https://example.com)"));
    }

    #[test]
    fn test_long_paragraph_wraps_to_width() {
        let theme = MarkdownTheme::default();
        let lines = render_markdown("abcdefghijabcdefghijabcdefghij", &theme, 10);
        let rendered: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
        assert!(rendered.iter().any(|line| line == "abcdefghij"));
        assert!(rendered.iter().all(|line| line.chars().count() <= 10));
    }

    #[test]
    fn test_blockquote_wrap_keeps_border() {
        let theme = MarkdownTheme::default();
        let lines = render_markdown("> abcdefghijabcdefghij", &theme, 8);
        let rendered: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
        assert!(rendered.iter().filter(|line| line.starts_with("│ ")).count() >= 2);
        assert!(rendered.iter().all(|line| line.chars().count() <= 8));
    }

    #[test]
    fn test_list_wrap_aligns_continuation() {
        let theme = MarkdownTheme::default();
        let lines = render_markdown("- abcdefghijabcdefghij", &theme, 10);
        let rendered: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
        assert!(rendered.iter().any(|line| line.starts_with("- ")));
        assert!(rendered.iter().any(|line| line.starts_with("  ")));
        assert!(rendered.iter().all(|line| line.chars().count() <= 10));
    }

    #[test]
    fn test_table_renders_with_borders() {
        let theme = MarkdownTheme::default();
        let lines = render_markdown("| Name | Value |\n| --- | --- |\n| alpha | beta |", &theme, 40);
        let rendered: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
        assert!(rendered.iter().any(|line| line.starts_with("╭")));
        assert!(rendered.iter().any(|line| line.contains("Name")));
        assert!(rendered.iter().any(|line| line.contains("alpha")));
        assert!(rendered.iter().any(|line| line.starts_with("╰")));
    }

    #[test]
    fn test_table_lines_fit_requested_width() {
        let theme = MarkdownTheme::default();
        let lines = render_markdown(
            "| Name | Value |\n| --- | --- |\n| a very long name | a very long value |",
            &theme,
            18,
        );
        let rendered: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
        assert!(rendered
            .iter()
            .filter(|line| !line.is_empty())
            .all(|line| line.chars().count() <= 18));
    }

    #[test]
    fn test_image_renders_alt_and_target() {
        let theme = MarkdownTheme::default();
        let lines = render_markdown("See ![chart](/tmp/chart.png)", &theme, 80);
        let rendered = lines
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("[image: chart]"));
        assert!(rendered.contains("/tmp/chart.png"));
    }

    #[test]
    fn test_image_placeholder_fits_requested_width() {
        let theme = MarkdownTheme::default();
        let lines = render_markdown("![very long alt text](/tmp/a-very-long-image-name.png)", &theme, 18);
        assert!(lines
            .iter()
            .map(|line| line.to_string())
            .filter(|line| !line.is_empty())
            .all(|line| line.to_string().chars().count() <= 18));
    }
}
