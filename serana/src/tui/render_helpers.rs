//! Rendering helpers for painting display-protocol types onto ScreenBuffer.
//!
//! These replace ratatui's widget rendering with direct ScreenBuffer painting.

use display_protocol::{Color, ScreenBuffer, Style, StyledLine, StyledSpan};

/// Paint a styled line starting at (x, y). Truncates at buffer boundary.
pub fn paint_styled_line(buf: &mut ScreenBuffer, x: u16, y: u16, line: &StyledLine) {
    let mut cx = x;
    for span in &line.spans {
        cx = paint_span(buf, cx, y, span);
        if cx >= buf.width {
            break;
        }
    }
}

/// Paint a StyledSpan starting at (x, y). Returns the x position after the span.
pub fn paint_span(buf: &mut ScreenBuffer, x: u16, y: u16, span: &StyledSpan) -> u16 {
    let fg = span.style.fg.unwrap_or(Color::WHITE);
    let bg = span.style.bg.unwrap_or(Color::BLACK);
    for (i, ch) in span.text.chars().enumerate() {
        let cx = x + i as u16;
        if cx >= buf.width {
            return cx;
        }
        buf.set_char(cx, y, ch, fg, bg, span.style.bold, span.style.dim, span.style.underline, span.style.strikethrough, span.style.italic, span.style.reverse);
    }
    x + span.text.chars().count() as u16
}

/// Paint plain text at (x, y) with a style.
pub fn paint_text(buf: &mut ScreenBuffer, x: u16, y: u16, text: &str, style: &Style) {
    let fg = style.fg.unwrap_or(Color::WHITE);
    let bg = style.bg.unwrap_or(Color::BLACK);
    for (i, ch) in text.chars().enumerate() {
        let cx = x + i as u16;
        if cx >= buf.width { break; }
        buf.set_char(cx, y, ch, fg, bg, style.bold, style.dim, style.underline, style.strikethrough, style.italic, style.reverse);
    }
}

/// Fill a region with spaces using a style (used for clearing areas).
pub fn clear_region(buf: &mut ScreenBuffer, x: u16, y: u16, w: u16, h: u16, bg: Color) {
    buf.fill_char(x, y, w, h, ' ', Color::WHITE, bg);
}

/// Paint a bordered block with optional title.
pub fn paint_block(
    buf: &mut ScreenBuffer,
    x: u16, y: u16, w: u16, h: u16,
    border_fg: Color,
    border_bg: Color,
    title: Option<&str>,
) {
    buf.draw_border(x, y, w, h, border_fg, border_bg, title);
}

/// Paint multiple styled lines within a region, with optional vertical scrolling.
/// Returns the number of lines painted.
pub fn paint_paragraph(
    buf: &mut ScreenBuffer,
    x: u16, y: u16, w: u16, h: u16,
    lines: &[StyledLine],
    scroll: usize,
) -> usize {
    let mut painted = 0;
    for (i, line) in lines.iter().skip(scroll).enumerate() {
        let ly = y + i as u16;
        if ly >= y + h { break; }
        // Truncate line to width
        paint_styled_line_truncated(buf, x, ly, line, w);
        painted += 1;
    }
    painted
}

/// Paint a styled line truncated to max_width characters.
pub fn paint_styled_line_truncated(
    buf: &mut ScreenBuffer,
    x: u16, y: u16,
    line: &StyledLine,
    max_width: u16,
) {
    let mut cx = x;
    let end = x + max_width;
    for span in &line.spans {
        for ch in span.text.chars() {
            if cx >= end { return; }
            let fg = span.style.fg.unwrap_or(Color::WHITE);
            let bg = span.style.bg.unwrap_or(Color::BLACK);
            buf.set_char(cx, y, ch, fg, bg, span.style.bold, span.style.dim, span.style.underline, span.style.strikethrough, span.style.italic, span.style.reverse);
            cx += 1;
        }
    }
}

/// Compute vertical layout regions. Returns (y_offset, height) for each constraint.
pub fn compute_vertical_layout(total_height: u16, constraints: &[VConstraint]) -> Vec<(u16, u16)> {
    let mut fixed_total: u16 = 0;
    let mut flex_count: u16 = 0;
    for c in constraints {
        match c {
            VConstraint::Length(len) => fixed_total += *len,
            VConstraint::Min(_) => flex_count += 1,
        }
    }
    let remaining = total_height.saturating_sub(fixed_total);
    let flex_each = if flex_count > 0 { remaining / flex_count } else { 0 };
    let mut extra = if flex_count > 0 { remaining % flex_count } else { 0 };

    let mut result = Vec::new();
    let mut offset: u16 = 0;
    for c in constraints {
        let h = match c {
            VConstraint::Length(len) => *len,
            VConstraint::Min(min) => {
                let h = flex_each + if extra > 0 { extra -= 1; 1 } else { 0 };
                h.max(*min)
            }
        };
        result.push((offset, h));
        offset += h;
    }
    result
}

/// Compute horizontal layout regions. Returns (x_offset, width) for each constraint.
pub fn compute_horizontal_layout(total_width: u16, constraints: &[HConstraint]) -> Vec<(u16, u16)> {
    let mut fixed_total: u16 = 0;
    let mut flex_count: u16 = 0;
    let mut pct_total: u16 = 0;
    for c in constraints {
        match c {
            HConstraint::Length(len) => fixed_total += *len,
            HConstraint::Percent(pct) => pct_total += *pct,
            HConstraint::Min(_) => flex_count += 1,
        }
    }
    let pct_width = (total_width as u32 * pct_total as u32 / 100) as u16;
    let used = fixed_total + pct_width;
    let remaining = total_width.saturating_sub(used);
    let flex_each = if flex_count > 0 { remaining / flex_count } else { 0 };

    let mut result = Vec::new();
    let mut offset: u16 = 0;
    for c in constraints {
        let w = match c {
            HConstraint::Length(len) => *len,
            HConstraint::Percent(pct) => (total_width as u32 * *pct as u32 / 100) as u16,
            HConstraint::Min(min) => flex_each.max(*min),
        };
        result.push((offset, w));
        offset += w;
    }
    result
}

#[derive(Debug, Clone, Copy)]
pub enum VConstraint {
    Length(u16),
    Min(u16),
}

#[derive(Debug, Clone, Copy)]
pub enum HConstraint {
    Length(u16),
    Percent(u16),
    Min(u16),
}

/// Convert a Vec<StyledLine> from a single message text (split by newlines).
pub fn text_to_lines(text: &str, style: Style) -> Vec<StyledLine> {
    text.lines()
        .map(|l| StyledLine::new(vec![StyledSpan::new(l.to_string(), style)]))
        .collect()
}
