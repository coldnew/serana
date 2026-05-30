use super::display::style::MoraColor;

/// Theme variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Night,
    Day,
}

fn rgb(hex: &str) -> MoraColor {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    MoraColor { r, g, b }
}

/// coldnew-theme color palette.
/// Based on ref/coldnew-emacs/user-lisp/coldnew-theme.el
pub struct ThemeColors {
    pub background: MoraColor,
    pub foreground: MoraColor,
    pub cursor: MoraColor,
    pub current_line: MoraColor,
    pub selection: MoraColor,
    pub highlight: MoraColor,
    pub builtin: MoraColor,
    pub constant: MoraColor,
    pub comment: MoraColor,
    pub comment_delimiter: MoraColor,
    pub doc: MoraColor,
    pub function_name: MoraColor,
    pub keyword: MoraColor,
    pub type_color: MoraColor,
    pub variable_name: MoraColor,
    pub string: MoraColor,
    pub red: MoraColor,
    pub yellow: MoraColor,
    pub orange: MoraColor,
    pub green: MoraColor,
    pub blue: MoraColor,
    pub magenta: MoraColor,
    pub cyan: MoraColor,
    pub white: MoraColor,
    pub black: MoraColor,
    pub modeline_fg: MoraColor,
    pub modeline_bg: MoraColor,
    pub gutter_dim: MoraColor,
    pub gutter_current: MoraColor,
    pub echo_prompt: MoraColor,
    pub echo_input: MoraColor,
    pub echo_msg: MoraColor,
    pub rainbow: [MoraColor; 9],
}

/// Night theme — coldnew's default dark theme.
pub fn night() -> ThemeColors {
    ThemeColors {
        background: rgb("#202020"),
        foreground: rgb("#c6cccc"),
        cursor: rgb("#00c8c8"),
        current_line: rgb("#2a2a2a"),
        selection: rgb("#3b3f41"),
        highlight: rgb("#CAE682"),
        builtin: rgb("#ccaaff"),
        constant: rgb("#ccaaff"),
        comment: rgb("#99aacc"),
        comment_delimiter: rgb("#5f5f5f"),
        doc: rgb("#97abc6"),
        function_name: rgb("#aaccff"),
        keyword: rgb("#aaffaa"),
        type_color: rgb("#fff59d"),
        variable_name: rgb("#aaccff"),
        string: rgb("#aadddd"),
        red: rgb("#ff3333"),
        yellow: rgb("#fff59d"),
        orange: rgb("#ff8888"),
        green: rgb("#aaffaa"),
        blue: rgb("#aaccff"),
        magenta: rgb("#ccaaff"),
        cyan: rgb("#aadddd"),
        white: rgb("#ffffff"),
        black: rgb("#2a2a2a"),
        modeline_fg: rgb("#c6cccc"),
        modeline_bg: rgb("#292929"),
        gutter_dim: rgb("#5f5f5f"),
        gutter_current: rgb("#aaccff"),
        echo_prompt: rgb("#aaccff"),
        echo_input: rgb("#c6cccc"),
        echo_msg: rgb("#97abc6"),
        rainbow: [
            rgb("#aadddd"),
            rgb("#81d4fa"),
            rgb("#aaccff"),
            rgb("#aaeecc"),
            rgb("#ccaaff"),
            rgb("#fff59d"),
            rgb("#ff8888"),
            rgb("#795548"),
            rgb("#827717"),
        ],
    }
}

/// Day theme — coldnew's light theme.
pub fn day() -> ThemeColors {
    ThemeColors {
        background: rgb("#FAFAFA"),
        foreground: rgb("#212121"),
        cursor: rgb("#00c8c8"),
        current_line: rgb("#ECEFF1"),
        selection: rgb("#3b3f41"),
        highlight: rgb("#CAE682"),
        builtin: rgb("#ccaaff"),
        constant: rgb("#ccaaff"),
        comment: rgb("#607d8b"),
        comment_delimiter: rgb("#5f5f5f"),
        doc: rgb("#97abc6"),
        function_name: rgb("#aaccff"),
        keyword: rgb("#558b2f"),
        type_color: rgb("#fff59d"),
        variable_name: rgb("#aaccff"),
        string: rgb("#aadddd"),
        red: rgb("#B71C1C"),
        yellow: rgb("#FFA000"),
        orange: rgb("#FF5722"),
        green: rgb("#558b2f"),
        blue: rgb("#2196f3"),
        magenta: rgb("#4527A0"),
        cyan: rgb("#aadddd"),
        white: rgb("#ffffff"),
        black: rgb("#2a2a2a"),
        modeline_fg: rgb("#212121"),
        modeline_bg: rgb("#ECEFF1"),
        gutter_dim: rgb("#999999"),
        gutter_current: rgb("#2196f3"),
        echo_prompt: rgb("#2196f3"),
        echo_input: rgb("#212121"),
        echo_msg: rgb("#607d8b"),
        rainbow: [
            rgb("#e91e63"),
            rgb("#1565C0"),
            rgb("#EF6C00"),
            rgb("#B388FF"),
            rgb("#76FF03"),
            rgb("#26A69A"),
            rgb("#B71C1C"),
            rgb("#795548"),
            rgb("#827717"),
        ],
    }
}

/// Convert ThemeColors to display-protocol Colors for syntax highlighting.
pub fn syntax_color_for_kind(t: &ThemeColors, kind: crate::mora::syntax::HighlightKind) -> MoraColor {
    use crate::mora::syntax::HighlightKind;
    match kind {
        HighlightKind::Keyword => t.keyword,
        HighlightKind::String => t.string,
        HighlightKind::Comment => t.comment,
        HighlightKind::Number => t.constant,
        HighlightKind::Function => t.function_name,
        HighlightKind::Type => t.type_color,
        HighlightKind::Operator => t.orange,
        HighlightKind::Bracket => t.foreground,
        HighlightKind::Property => t.yellow,
        HighlightKind::Variable => t.variable_name,
        HighlightKind::Constant => t.constant,
        HighlightKind::Heading => t.keyword,
        HighlightKind::Bold => t.foreground,
        HighlightKind::Italic => t.foreground,
        HighlightKind::Link => t.blue,
        HighlightKind::Code => t.string,
        HighlightKind::ListMarker => t.keyword,
        HighlightKind::Blockquote => t.comment,
        HighlightKind::HorizontalRule => t.comment_delimiter,
        HighlightKind::Tag => t.keyword,
        HighlightKind::Attribute => t.yellow,
        HighlightKind::Normal => t.foreground,
    }
}
