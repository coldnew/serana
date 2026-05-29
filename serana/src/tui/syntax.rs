use std::sync::OnceLock;

use display_protocol::{Color, Style, StyledSpan};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

static INSTANCE: OnceLock<SyntaxHighlighter> = OnceLock::new();

pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl SyntaxHighlighter {
    pub fn global() -> &'static Self {
        INSTANCE.get_or_init(|| {
            let syntax_set = SyntaxSet::load_defaults_newlines();
            let theme_set = ThemeSet::load_defaults();
            let theme = theme_set
                .themes
                .get("base16-ocean.dark")
                .cloned()
                .unwrap_or_else(|| {
                    theme_set
                        .themes
                        .iter()
                        .next()
                        .map(|(_, t)| t.clone())
                        .unwrap()
                });
            Self { syntax_set, theme }
        })
    }

    pub fn highlight_lines(&self, code: &str, lang: &str) -> Vec<Vec<StyledSpan>> {
        let syntax = self
            .syntax_set
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        let mut result = Vec::new();

        for line in LinesWithEndings::from(code) {
            if let Ok(ranges) = highlighter.highlight_line(line, &self.syntax_set) {
                let spans: Vec<StyledSpan> = ranges
                    .into_iter()
                    .map(|(s, text)| StyledSpan::new(text.to_string(), syntect_style_to_display(s)))
                    .collect();
                result.push(spans);
            }
        }

        result
    }
}

fn syntect_style_to_display(style: syntect::highlighting::Style) -> Style {
    let mut s = Style::default();
    if style.foreground != syntect::highlighting::Color::BLACK {
        s = s.fg(color_to_display(style.foreground));
    }
    if style.font_style.contains(FontStyle::BOLD) {
        s = s.bold();
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        s = s.italic();
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        s = s.underline();
    }
    s
}

fn color_to_display(c: syntect::highlighting::Color) -> Color {
    Color::new(c.r, c.g, c.b)
}
