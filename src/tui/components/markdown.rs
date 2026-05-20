//! Markdown rendering for assistant messages.
//!
//! Parses CommonMark with pulldown-cmark and renders to styled terminal lines.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::tui::component::Component;
use crate::tui::style::{Colors, Style};

/// Markdown element styling theme.
pub struct MarkdownTheme {
    /// Heading text (adds color and weight)
    pub heading: Style,
    /// Link text (underline + color)
    pub link: Style,
    /// Inline code (different color)
    pub code: Style,
    /// Code block background style
    pub code_block: Style,
    /// Blockquote style
    pub quote: Style,
    /// List bullet color
    pub list_bullet: Style,
    /// Bold style
    pub bold: Style,
    /// Italic style
    pub italic: Style,
    /// Horizontal rule color
    pub hr: Style,
}

impl Default for MarkdownTheme {
    fn default() -> Self {
        Self {
            heading: Style::new().fg(Colors::BRIGHT_BLUE).bold(),
            link: Style::new().fg(Colors::BRIGHT_CYAN),
            code: Style::new().fg(Colors::YELLOW),
            code_block: Style::new().fg(Colors::GRAY),
            quote: Style::new().fg(Colors::GRAY).italic(),
            list_bullet: Style::new().fg(Colors::CYAN),
            bold: Style::new().bold(),
            italic: Style::new().italic(),
            hr: Style::new().fg(Colors::GRAY),
        }
    }
}

/// Markdown renderer that parses CommonMark and produces styled terminal output.
pub struct Markdown {
    text: String,
    theme: MarkdownTheme,
    padding_x: usize,
    padding_y: usize,
    code_block_indent: usize,
    // Caching
    cached_text: Option<String>,
    cached_width: Option<usize>,
    cached_lines: Option<Vec<String>>,
}

impl Markdown {
    /// Create a new Markdown renderer with default theme.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            theme: MarkdownTheme::default(),
            padding_x: 0,
            padding_y: 0,
            code_block_indent: 2,
            cached_text: None,
            cached_width: None,
            cached_lines: None,
        }
    }

    /// Create with custom theme.
    pub fn with_theme(text: impl Into<String>, theme: MarkdownTheme) -> Self {
        Self {
            text: text.into(),
            theme,
            padding_x: 0,
            padding_y: 0,
            code_block_indent: 2,
            cached_text: None,
            cached_width: None,
            cached_lines: None,
        }
    }

    /// Set horizontal padding.
    pub fn padding_x(mut self, n: usize) -> Self {
        self.padding_x = n;
        self
    }

    /// Set vertical padding.
    pub fn padding_y(mut self, n: usize) -> Self {
        self.padding_y = n;
        self
    }

    /// Set code block indentation.
    pub fn code_block_indent(mut self, n: usize) -> Self {
        self.code_block_indent = n;
        self
    }

    /// Update the markdown text.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.invalidate();
    }

    /// Render markdown to styled lines.
    fn render_markdown(&self, width: usize) -> Vec<String> {
        if self.text.trim().is_empty() {
            return Vec::new();
        }

        let content_width = width.saturating_sub(self.padding_x * 2);
        if content_width == 0 {
            return Vec::new();
        }

        // Replace tabs with 3 spaces for consistent rendering
        let normalized = self.text.replace('\t', "   ");

        // Parse markdown
        let parser = Parser::new_ext(&normalized, Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH);

        // Render events to lines
        let renderer = MarkdownRenderer::new(&self.theme, content_width, self.code_block_indent);
        renderer.render_events(parser)
    }
}

impl Component for Markdown {
    fn render(&self, width: usize) -> Vec<String> {
        // Check cache
        if let (Some(cached_text), Some(cached_width), Some(cached_lines)) =
            (&self.cached_text, self.cached_width, &self.cached_lines)
        {
            if cached_text == &self.text && cached_width == width {
                return cached_lines.clone();
            }
        }

        let raw_lines = self.render_markdown(width);

        // Apply padding
        let mut lines = Vec::new();
        let left_pad = " ".repeat(self.padding_x);
        let right_pad = " ".repeat(self.padding_x);

        // Top padding
        for _ in 0..self.padding_y {
            lines.push(" ".repeat(width));
        }

        // Content with horizontal padding
        for line in raw_lines {
            let visible_len = visible_width(&line);
            let right_needed = width.saturating_sub(visible_len + self.padding_x * 2);
            lines.push(format!("{}{}{}{}", left_pad, line, right_pad, " ".repeat(right_needed)));
        }

        // Bottom padding
        for _ in 0..self.padding_y {
            lines.push(" ".repeat(width));
        }

        lines
    }

    fn invalidate(&mut self) {
        self.cached_text = None;
        self.cached_width = None;
        self.cached_lines = None;
    }
}

/// Calculate visible width (excluding ANSI codes).
fn visible_width(s: &str) -> usize {
    let mut width = 0;
    let mut in_escape = false;

    for ch in s.chars() {
        if in_escape {
            if ch == 'm' {
                in_escape = false;
            }
        } else if ch == '\x1b' {
            in_escape = true;
        } else {
            width += unicode_width(ch);
        }
    }
    width
}

/// Get unicode display width for a character.
fn unicode_width(ch: char) -> usize {
    // Simplified width calculation - covers most common cases
    match ch {
        // Control characters
        '\x00'..='\x1F' => 0,
        // Zero-width characters
        '\u{0300}'..='\u{036F}' => 0, // Combining diacritical marks
        '\u{200B}' => 0, // Zero-width space
        '\u{200C}'..='\u{200D}' => 0, // Zero-width joiners
        '\u{FE00}'..='\u{FE0F}' => 0, // Variation selectors
        '\u{FEFF}' => 0, // BOM
        // East Asian Wide characters (width 2)
        _ if is_wide_char(ch) => 2,
        // Everything else
        _ => 1,
    }
}

/// Check if character is East Asian Wide or Fullwidth.
fn is_wide_char(ch: char) -> bool {
    matches!(ch,
        // East Asian Wide ranges
        '\u{1100}'..='\u{115F}' |
        '\u{231A}'..='\u{231B}' |
        '\u{2329}'..='\u{232A}' |
        '\u{23E9}'..='\u{23EC}' |
        '\u{23F0}'..='\u{23F3}' |
        '\u{25FD}'..='\u{25FE}' |
        '\u{2614}'..='\u{2615}' |
        '\u{2648}'..='\u{2653}' |
        '\u{267F}'..='\u{267F}' |
        '\u{2693}'..='\u{2693}' |
        '\u{26A1}'..='\u{26A1}' |
        '\u{26AA}'..='\u{26AB}' |
        '\u{26BD}'..='\u{26BE}' |
        '\u{26C4}'..='\u{26C5}' |
        '\u{26CE}'..='\u{26CE}' |
        '\u{26D4}'..='\u{26D4}' |
        '\u{26EA}'..='\u{26EA}' |
        '\u{26F2}'..='\u{26F3}' |
        '\u{26F5}'..='\u{26F5}' |
        '\u{26FA}'..='\u{26FA}' |
        '\u{26FD}'..='\u{26FD}' |
        '\u{2702}'..='\u{2702}' |
        '\u{2705}'..='\u{2705}' |
        '\u{2708}'..='\u{270D}' |
        '\u{270F}'..='\u{270F}' |
        '\u{2712}'..='\u{2712}' |
        '\u{2714}'..='\u{2714}' |
        '\u{2716}'..='\u{2716}' |
        '\u{271D}'..='\u{271D}' |
        '\u{2721}'..='\u{2721}' |
        '\u{2728}'..='\u{2728}' |
        '\u{2733}'..='\u{2734}' |
        '\u{2744}'..='\u{2744}' |
        '\u{2747}'..='\u{2747}' |
        '\u{274C}'..='\u{274C}' |
        '\u{274E}'..='\u{274E}' |
        '\u{2753}'..='\u{2755}' |
        '\u{2757}'..='\u{2757}' |
        '\u{2763}'..='\u{2764}' |
        '\u{2795}'..='\u{2797}' |
        '\u{27A1}'..='\u{27A1}' |
        '\u{27B0}'..='\u{27B0}' |
        '\u{27BF}'..='\u{27BF}' |
        '\u{2934}'..='\u{2935}' |
        '\u{2B05}'..='\u{2B07}' |
        '\u{2B1B}'..='\u{2B1C}' |
        '\u{2B50}'..='\u{2B50}' |
        '\u{2B55}'..='\u{2B55}' |
        '\u{3030}'..='\u{3030}' |
        '\u{303D}'..='\u{303D}' |
        '\u{3297}'..='\u{3297}' |
        '\u{3299}'..='\u{3299}' |
        // CJK ranges
        '\u{3000}'..='\u{303E}' |
        '\u{3041}'..='\u{3096}' |
        '\u{3099}'..='\u{30FF}' |
        '\u{3105}'..='\u{312F}' |
        '\u{3131}'..='\u{318E}' |
        '\u{3190}'..='\u{31E3}' |
        '\u{31F0}'..='\u{4DBF}' |
        '\u{4E00}'..='\u{A48C}' |
        '\u{A490}'..='\u{A4C6}' |
        '\u{A960}'..='\u{A97C}' |
        '\u{AC00}'..='\u{D7A3}' |
        '\u{F900}'..='\u{FAFF}' |
        '\u{FE10}'..='\u{FE1F}' |
        '\u{FE30}'..='\u{FE6B}' |
        '\u{FF01}'..='\u{FF60}' |
        '\u{FFE0}'..='\u{FFE6}'
    )
}

/// Format a terminal hyperlink using OSC 8.
fn format_hyperlink(text: &str, target: &str) -> String {
    // OSC 8 hyperlink format: \x1b]8;;url\x07text\x1b]8;;\x07
    let safe_target = target.replace(['\x1b', '\x07'], "");
    if safe_target.is_empty() {
        return text.to_string();
    }
    format!("\x1b]8;;{}\x07{}\x1b]8;;\x07", safe_target, text)
}

/// Internal renderer state.
struct MarkdownRenderer<'a> {
    theme: &'a MarkdownTheme,
    width: usize,
    code_block_indent: usize,
    lines: Vec<String>,
    current_line: String,
    current_style: Option<Style>,
    // State tracking
    in_code_block: bool,
    code_block_lang: String,
    in_blockquote: bool,
    in_list: bool,
    list_ordered: bool,
    list_counter: usize,
    list_depth: usize,
    link_target: Option<String>,
    /// Style stack for nested formatting
    style_stack: Vec<Style>,
}

impl<'a> MarkdownRenderer<'a> {
    fn new(theme: &'a MarkdownTheme, width: usize, code_block_indent: usize) -> Self {
        Self {
            theme,
            width,
            code_block_indent,
            lines: Vec::new(),
            current_line: String::new(),
            current_style: None,
            in_code_block: false,
            code_block_lang: String::new(),
            in_blockquote: false,
            in_list: false,
            list_ordered: false,
            list_counter: 0,
            list_depth: 0,
            link_target: None,
            style_stack: Vec::new(),
        }
    }

    fn render_events(mut self, parser: Parser<'_>) -> Vec<String> {
        for event in parser {
            self.handle_event(event);
        }

        // Flush remaining line
        if !self.current_line.is_empty() {
            self.lines.push(self.current_line);
        }

        self.lines
    }

    fn handle_event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.handle_start_tag(tag),
            Event::End(tag) => self.handle_end_tag(tag),
            Event::Text(text) => {
                if self.in_code_block {
                    // Code block content - preserve formatting
                    let indent = " ".repeat(self.code_block_indent);
                    for line in text.lines() {
                        self.current_line.push_str(&indent);
                        self.push_styled(line, self.theme.code_block);
                        self.flush_line();
                    }
                } else {
                    self.push_text(&text);
                }
            }
            Event::Code(code) => {
                let styled = self.theme.code.apply(&code);
                self.current_line.push_str(&styled);
            }
            Event::Html(html) => {
                // Render HTML as plain text
                self.push_text(html.as_ref());
            }
            Event::SoftBreak | Event::HardBreak => {
                self.flush_line();
            }
            Event::Rule => {
                let hr_line = self.theme.hr.apply(&"─".repeat(self.width.min(80)));
                self.lines.push(hr_line);
            }
            Event::FootnoteReference(_) | Event::InlineMath(_) | Event::DisplayMath(_) => {}
            Event::TaskListMarker(checked) => {
                let marker = if checked { "✓ " } else { "○ " };
                self.current_line.push_str(&self.theme.list_bullet.apply(marker));
            }
            Event::InlineHtml(html) => {
                self.push_text(html.as_ref());
            }
        }
    }

    fn handle_start_tag(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => {}
            Tag::Heading { level, .. } => {
                self.current_style = Some(self.theme.heading);
                if level != HeadingLevel::H1 {
                    let prefix = "#".repeat(level as usize) + " ";
                    self.current_line.push_str(&self.theme.heading.apply(&prefix));
                }
            }
            Tag::BlockQuote(_) => {
                self.in_blockquote = true;
                self.current_style = Some(self.theme.quote);
            }
            Tag::CodeBlock(kind) => {
                self.in_code_block = true;
                self.code_block_lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                    _ => String::new(),
                };
                // Push opening fence
                let fence = if self.code_block_lang.is_empty() {
                    "```".to_string()
                } else {
                    format!("```{}", self.code_block_lang)
                };
                self.lines.push(self.theme.code_block.apply(&fence));
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
                self.current_line.push_str(&indent);
                self.current_line.push_str(&self.theme.list_bullet.apply(&bullet));
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
                // Strikethrough not directly supported, use dim style
                self.style_stack.push(Style::new().dim());
                self.current_style = Some(Style::new().dim());
            }
            Tag::Link { dest_url, .. } => {
                self.link_target = Some(dest_url.to_string());
                self.style_stack.push(self.theme.link);
                self.current_style = Some(self.theme.link);
            }
            Tag::Image { .. } => {}
            Tag::Table(_) | Tag::TableHead | Tag::TableRow | Tag::TableCell => {}
            Tag::FootnoteDefinition(_) | Tag::MetadataBlock(_) | Tag::DefinitionList | Tag::DefinitionListTitle | Tag::DefinitionListDefinition | Tag::HtmlBlock => {}
        }
    }

    fn handle_end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                self.flush_line();
                self.lines.push(String::new()); // Add spacing
            }
            TagEnd::Heading { .. } => {
                self.flush_line();
                self.current_style = None;
                self.lines.push(String::new()); // Add spacing after heading
            }
            TagEnd::BlockQuote(_) => {
                self.in_blockquote = false;
                self.current_style = None;
                self.flush_line();
            }
            TagEnd::CodeBlock => {
                self.in_code_block = false;
                self.lines.push(self.theme.code_block.apply("```"));
                self.lines.push(String::new()); // Add spacing
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
                // Add clickable hyperlink
                if let Some(target) = self.link_target.take() {
                    let clickable = format_hyperlink("", &target);
                    self.current_line.push_str(&clickable);
                }
                self.style_stack.pop();
                self.current_style = self.style_stack.last().copied();
            }
            TagEnd::Image => {}
            TagEnd::Table | TagEnd::TableHead | TagEnd::TableRow | TagEnd::TableCell => {}
            TagEnd::FootnoteDefinition | TagEnd::MetadataBlock(_) | TagEnd::DefinitionList | TagEnd::DefinitionListTitle | TagEnd::DefinitionListDefinition | TagEnd::HtmlBlock => {}
        }
    }

    fn push_text(&mut self, text: &str) {
        if let Some(style) = self.current_style {
            self.push_styled(text, style);
        } else {
            self.current_line.push_str(text);
        }
    }

    fn push_styled(&mut self, text: &str, style: Style) {
        let styled = style.apply(text);
        self.current_line.push_str(&styled);
    }

    fn flush_line(&mut self) {
        if !self.current_line.is_empty() {
            // Apply blockquote prefix if needed
            if self.in_blockquote {
                let prefix = self.theme.quote.apply("│ ");
                self.current_line = format!("{}{}", prefix, self.current_line);
            }
            self.lines.push(self.current_line.clone());
            self.current_line.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading() {
        let md = Markdown::new("# Hello World");
        let lines = md.render(80);
        assert!(!lines.is_empty());
        assert!(lines[0].contains("Hello World"));
    }

    #[test]
    fn test_list() {
        let md = Markdown::new("- Item 1\n- Item 2");
        let lines = md.render(80);
        assert!(!lines.is_empty());
        assert!(lines.iter().any(|l| l.contains("Item 1")));
        assert!(lines.iter().any(|l| l.contains("Item 2")));
    }

    #[test]
    fn test_code_block() {
        let md = Markdown::new("```rust\nfn main() {}\n```");
        let lines = md.render(80);
        assert!(lines.iter().any(|l| l.contains("```rust")));
        assert!(lines.iter().any(|l| l.contains("fn main()")));
        assert!(lines.iter().any(|l| l.contains("```")));
    }

    #[test]
    fn test_bold_italic() {
        let md = Markdown::new("**bold** and *italic*");
        let lines = md.render(80);
        assert!(!lines.is_empty());
        assert!(lines[0].contains("bold"));
        assert!(lines[0].contains("italic"));
    }

    #[test]
    fn test_link() {
        let md = Markdown::new("[Click here](https://example.com)");
        let lines = md.render(80);
        assert!(!lines.is_empty());
        // Link text should be present
        assert!(lines[0].contains("Click here"));
    }

    #[test]
    fn test_inline_code() {
        let md = Markdown::new("Use `cargo run` to execute");
        let lines = md.render(80);
        assert!(!lines.is_empty());
        assert!(lines[0].contains("cargo run"));
    }
}
