use std::path::Path;
use std::sync::OnceLock;

use display_protocol::{Color, Style, StyledLine, StyledSpan};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;

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
                        .map(|(_, theme)| theme.clone())
                        .unwrap()
                });

            Self { syntax_set, theme }
        })
    }

    /// Highlight all lines (used when buffer is small or path is unknown).
    pub fn highlight_buffer(&self, lines: &[&str], path: Option<&Path>) -> Vec<StyledLine> {
        self.highlight_range(lines, path, 0, lines.len())
    }

    /// Highlight only lines [start, start+len) for efficient per-frame rendering.
    /// Lines outside the range are returned as plain text.
    pub fn highlight_range(
        &self,
        lines: &[&str],
        path: Option<&Path>,
        start: usize,
        len: usize,
    ) -> Vec<StyledLine> {
        let total = lines.len();
        let end = (start + len).min(total);

        let Some(path) = path else {
            return lines[start..end]
                .iter()
                .map(|l| StyledLine::plain(*l))
                .collect();
        };

        let syntax = match self.syntax_set.find_syntax_for_file(path) {
            Ok(Some(syntax)) => syntax,
            _ => {
                return lines[start..end]
                    .iter()
                    .map(|l| StyledLine::plain(*l))
                    .collect();
            }
        };

        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        lines[start..end]
            .iter()
            .map(
                |line| match highlighter.highlight_line(line, &self.syntax_set) {
                    Ok(ranges) => {
                        let spans = ranges
                            .into_iter()
                            .map(|(style, text)| {
                                StyledSpan::new(text.to_string(), syntect_style_to_display(style))
                            })
                            .collect();
                        StyledLine::new(spans)
                    }
                    Err(_) => StyledLine::plain(*line),
                },
            )
            .collect()
    }
}
fn syntect_style_to_display(style: syntect::highlighting::Style) -> Style {
    let mut display = Style::default().fg(color_to_display(style.foreground));

    if style.font_style.contains(FontStyle::BOLD) {
        display = display.bold();
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        display = display.italic();
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        display = display.underline();
    }

    display
}

fn color_to_display(color: syntect::highlighting::Color) -> Color {
    Color::new(color.r, color.g, color.b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_or_unnamed_buffers_stay_plain() {
        let lines = ["fn main() {}"];
        let highlighted = SyntaxHighlighter::global().highlight_buffer(&lines, None);

        assert_eq!(highlighted.len(), 1);
        assert_eq!(highlighted[0].spans.len(), 1);
        assert_eq!(highlighted[0].spans[0].text, "fn main() {}");
        assert_eq!(highlighted[0].spans[0].style, Style::default());
    }

    #[test]
    fn rust_file_gets_highlighted() {
        let lines = ["fn main() {}"];
        let highlighted =
            SyntaxHighlighter::global().highlight_buffer(&lines, Some(Path::new("main.rs")));

        assert_eq!(highlighted.len(), 1);
        assert!(highlighted[0]
            .spans
            .iter()
            .any(|span| span.text == "fn" && span.style.fg.is_some()));
    }

    #[test]
    fn toml_file_gets_highlighted() {
        let lines = ["[package]", "name = \"vivi\""];
        let highlighted =
            SyntaxHighlighter::global().highlight_buffer(&lines, Some(Path::new("Cargo.toml")));

        assert_eq!(highlighted.len(), 2);
        // The section header [package] should have colored spans
        assert!(
            highlighted[0]
                .spans
                .iter()
                .any(|span| span.style.fg.is_some()),
            "TOML section header should be highlighted"
        );
        // The key-value line should also have colored spans
        assert!(
            highlighted[1]
                .spans
                .iter()
                .any(|span| span.style.fg.is_some()),
            "TOML key-value should be highlighted"
        );
    }
}
