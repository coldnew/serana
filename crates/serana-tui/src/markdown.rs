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
}

impl<'a> MarkdownRenderer<'a> {
    fn new(theme: &'a MarkdownTheme, width: usize) -> Self {
        Self {
            theme,
            width,
            lines: Vec::new(),
            current_spans: Vec::new(),
            link_target: None,
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
                } else {
                    self.push_text(&text);
                }
            }
            Event::Code(code) => {
                self.current_spans
                    .push(Span::styled(code.to_string(), self.theme.code));
            }
            Event::SoftBreak | Event::HardBreak => {
                self.flush_line();
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
                self.style_stack.push(Style::new());
                self.current_style = Some(Style::new());
            }
            Tag::Link { dest_url, .. } => {
                self.link_target = Some(dest_url.to_string());
                self.style_stack.push(self.theme.link);
                self.current_style = Some(self.theme.link);
            }
            Tag::Image { .. } => {}
            Tag::Table(_) | Tag::TableHead | Tag::TableRow | Tag::TableCell => {}
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
                    if !target.is_empty() {
                        self.current_spans
                            .push(Span::styled(format!(" ({})", target), self.theme.link));
                    }
                }
                self.style_stack.pop();
                self.current_style = self.style_stack.last().copied();
            }
            TagEnd::Image => {}
            TagEnd::Table | TagEnd::TableHead | TagEnd::TableRow | TagEnd::TableCell => {}
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
            if self.in_blockquote {
                let mut quoted = vec![Span::styled("│ ".to_string(), self.theme.quote)];
                quoted.append(&mut spans);
                self.lines.push(Line::from(quoted));
            } else {
                self.lines.push(Line::from(spans));
            }
        }
    }
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
    fn test_inline_code() {
        let theme = MarkdownTheme::default();
        let lines = render_markdown("Use `cargo run`", &theme, 80);
        assert!(lines.iter().any(|l| {
            let s = l.clone().to_string();
            s.contains("cargo run")
        }));
    }
}
