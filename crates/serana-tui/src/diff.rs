use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme;
use crate::theme::Theme;

/// Tokenize a string into (content, is_word) pairs.
/// Words are alphanumeric sequences; everything else is a separator token.
fn tokenize(s: &str) -> Vec<(String, bool)> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_word = false;
    for ch in s.chars() {
        let alnum = ch.is_alphanumeric() || ch == '_';
        let ch_str = ch.to_string();
        if current.is_empty() {
            current = ch_str;
            in_word = alnum;
        } else if alnum == in_word {
            current.push(ch);
        } else {
            tokens.push((std::mem::take(&mut current), in_word));
            current = ch_str;
            in_word = alnum;
        }
    }
    if !current.is_empty() {
        tokens.push((current, in_word));
    }
    tokens
}

/// Compute LCS match indices between two token sequences.
fn lcs_indices(a: &[(String, bool)], b: &[(String, bool)]) -> Vec<(usize, usize)> {
    let m = a.len();
    let n = b.len();
    if m == 0 || n == 0 {
        return Vec::new();
    }
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i - 1].0 == b[j - 1].0 {
                dp[i - 1][j - 1] + 1
            } else {
                dp[i - 1][j].max(dp[i][j - 1])
            };
        }
    }
    let mut result = Vec::new();
    let (mut i, mut j) = (m, n);
    while i > 0 && j > 0 {
        if a[i - 1].0 == b[j - 1].0 {
            result.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    result.reverse();
    result
}

/// Compute word-level diff for a pair of old/new lines.
/// Returns (old_spans, new_spans).
fn token_diff(old: &str, new: &str) -> (Vec<Span<'static>>, Vec<Span<'static>>) {
    let old_tokens = tokenize(old);
    let new_tokens = tokenize(new);
    let matches = lcs_indices(&old_tokens, &new_tokens);

    let mut old_spans = vec![Span::raw("-")];
    let mut new_spans = vec![Span::raw("+")];

    let mut oi = 0;
    let mut ni = 0;

    for &(mi, mj) in &matches {
        while oi < mi {
            old_spans.push(Span::styled(
                old_tokens[oi].0.clone(),
                Style::new()
                    .fg(theme::DIFF_RED)
                    .add_modifier(Modifier::BOLD),
            ));
            oi += 1;
        }
        while ni < mj {
            new_spans.push(Span::styled(
                new_tokens[ni].0.clone(),
                Style::new()
                    .fg(theme::DIFF_GREEN)
                    .add_modifier(Modifier::BOLD),
            ));
            ni += 1;
        }
        old_spans.push(Span::styled(
            old_tokens[mi].0.clone(),
            Style::new().fg(theme::BRIGHT_CORAL),
        ));
        new_spans.push(Span::styled(
            new_tokens[mj].0.clone(),
            Style::new().fg(theme::SEAFOAM_GREEN),
        ));
        oi = mi + 1;
        ni = mj + 1;
    }

    while oi < old_tokens.len() {
        old_spans.push(Span::styled(
            old_tokens[oi].0.clone(),
            Style::new()
                .fg(theme::DIFF_RED)
                .add_modifier(Modifier::BOLD),
        ));
        oi += 1;
    }
    while ni < new_tokens.len() {
        new_spans.push(Span::styled(
            new_tokens[ni].0.clone(),
            Style::new()
                .fg(theme::DIFF_GREEN)
                .add_modifier(Modifier::BOLD),
        ));
        ni += 1;
    }

    (old_spans, new_spans)
}

/// Render a unified diff with word-level intra-line highlighting.
pub fn render_diff(diff_text: &str, _width: usize) -> Vec<Line<'static>> {
    let theme = Theme::default();
    let mut lines = Vec::new();
    let diff_lines: Vec<&str> = diff_text.lines().collect();
    let mut i = 0;

    while i < diff_lines.len() {
        let line = diff_lines[i];

        if line.starts_with("@@") {
            lines.push(Line::from(Span::styled(line.to_string(), theme.info)));
            i += 1;
            continue;
        }

        if !line.is_empty()
            && line.as_bytes()[0] == b'-'
            && i + 1 < diff_lines.len()
            && !diff_lines[i + 1].is_empty()
            && diff_lines[i + 1].as_bytes()[0] == b'+'
        {
            let (old_spans, new_spans) = token_diff(&line[1..], &diff_lines[i + 1][1..]);
            lines.push(Line::from(old_spans));
            lines.push(Line::from(new_spans));
            i += 2;
            continue;
        }

        if line.starts_with('-') {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::new().fg(theme::BRIGHT_CORAL),
            )));
        } else if line.starts_with('+') {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::new().fg(theme::SEAFOAM_GREEN),
            )));
        } else {
            lines.push(Line::from(Span::raw(line.to_string())));
        }
        i += 1;
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("foo bar baz");
        let words: Vec<&str> = tokens
            .iter()
            .filter(|(_, w)| *w)
            .map(|(s, _)| s.as_str())
            .collect();
        assert_eq!(words, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn test_tokenize_with_punctuation() {
        let tokens = tokenize("foo.bar()");
        let words: Vec<&str> = tokens
            .iter()
            .filter(|(_, w)| *w)
            .map(|(s, _)| s.as_str())
            .collect();
        assert_eq!(words, vec!["foo", "bar"]);
    }

    #[test]
    fn test_render_diff_basic() {
        let diff = "\
@@ file.txt @@
-context
+context
-old line with changes
+new line with changes";
        let lines = render_diff(diff, 80);
        assert!(lines.len() >= 4);
    }

    #[test]
    fn test_render_diff_empty() {
        let lines = render_diff("", 80);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_render_diff_no_hunks() {
        let diff = "plain text\nno diff markers";
        let lines = render_diff(diff, 80);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_lcs_indices() {
        let a = vec![("a".into(), true), ("b".into(), true), ("c".into(), true)];
        let b = vec![("a".into(), true), ("x".into(), true), ("c".into(), true)];
        let matches = lcs_indices(&a, &b);
        assert_eq!(matches, vec![(0, 0), (2, 2)]);
    }
}
