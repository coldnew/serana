use std::fmt;

/// Editor modes, modeled after vim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Normal mode — navigation and commands.
    Normal,
    /// Insert mode — text entry.
    Insert,
    /// Command mode — `:` command line.
    Command,
    /// Visual mode — character-wise selection.
    Visual,
    /// Visual line mode — line-wise selection.
    VisualLine,
    /// Replace mode — overwrite characters.
    Replace,
}

impl Mode {
    /// Indicator string for the status bar.
    pub fn indicator(&self) -> &'static str {
        match self {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Command => "COMMAND",
            Mode::Visual => "VISUAL",
            Mode::VisualLine => "V-LINE",
            Mode::Replace => "REPLACE",
        }
    }

    /// Padded label for the status bar mode indicator (avoids format!).
    pub fn mode_label(&self) -> &'static str {
        match self {
            Mode::Normal => " NORMAL ",
            Mode::Insert => " INSERT ",
            Mode::Command => " COMMAND ",
            Mode::Visual => " VISUAL ",
            Mode::VisualLine => " V-LINE ",
            Mode::Replace => " REPLACE ",
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.indicator())
    }
}
