use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::rectangle::RectRegion;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    Normal,
    Insert,
    Command,
    SearchForward,
    SearchBackward,
    Emacs,
    ReplaceChar,
    Visual,
}

impl EditorMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Insert => "INSERT",
            Self::Command => "COMMAND",
            Self::SearchForward => "SEARCH /",
            Self::SearchBackward => "SEARCH ?",
            Self::Emacs => "EMACS",
            Self::ReplaceChar => "REPLACE",
            Self::Visual => "VISUAL",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingOp {
    Delete,
    Yank,
    Change,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAction {
    None,
    InsertChar(char),
    InsertNewline,
    DeleteBackward,
    DeleteForward,
    DeleteLine,
    DeleteToEol,
    DeleteWordForward,
    DeleteWordBackward,
    KillLine,
    KillWordForward,
    KillRegion,
    KillRect,
    KillAppend,
    CopyRegion,
    Yank,
    YankPop,
    YankRect,
    ClearRect,
    InsertRect,
    SetMode(EditorMode),
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MoveLineStart,
    MoveLineEnd,
    MoveFileStart,
    MoveFileEnd,
    MoveWordForward,
    MoveWordBackward,
    MoveWordEnd,
    ScrollUp,
    ScrollDown,
    ScrollHalfPageUp,
    ScrollHalfPageDown,
    PageUp,
    PageDown,
    Undo,
    Redo,
    Save,
    SaveAs,
    Quit,
    ForceQuit,
    InputChar(char),
    ExecuteInput,
    InputBackspace,
    GotoLine(usize),
    FindForward(String),
    FindBackward(String),
    ReplaceFirst(String, String),
    ReplaceAll(String, String),
    RepeatFind(bool),
    ToggleVisual,
    YankLine,
    PasteAfter,
    IndentLine,
    UnindentLine,
    OpenLineBelow,
    OpenLineAbove,
    JoinLines,
    ReplaceChar(char),
    SwitchToEmacs,
    StartMacro,
    EndMacro,
    CallMacro,
    IndentRegion(RectRegion),
    TransposeChar,
    TransposeWord,
    TransposeLine,
    CapitalizeWord,
    UppercaseWord,
    LowercaseWord,
}

impl KeyAction {
    pub fn needs_digit(&self) -> bool {
        matches!(self, KeyAction::GotoLine(_))
    }
}

pub fn normal_key(key: KeyEvent) -> KeyAction {
    match (key.modifiers, key.code) {
        (_, KeyCode::Char('i')) => KeyAction::SetMode(EditorMode::Insert),
        (_, KeyCode::Char('a')) => KeyAction::MoveRight,
        (_, KeyCode::Char('A')) => KeyAction::MoveLineEnd,
        (_, KeyCode::Char('I')) => KeyAction::MoveLineStart,
        (_, KeyCode::Char(':')) => KeyAction::SetMode(EditorMode::Command),
        (_, KeyCode::Char('/')) => KeyAction::SetMode(EditorMode::SearchForward),
        (_, KeyCode::Char('?')) => KeyAction::SetMode(EditorMode::SearchBackward),
        (_, KeyCode::Char('v')) => KeyAction::ToggleVisual,
        (_, KeyCode::Char('V')) => {
            KeyAction::ToggleVisual
        }
        (KeyModifiers::CONTROL, KeyCode::Char('e')) => KeyAction::SwitchToEmacs,

        (_, KeyCode::Char('h')) | (_, KeyCode::Left) => KeyAction::MoveLeft,
        (_, KeyCode::Char('j')) | (_, KeyCode::Down) => KeyAction::MoveDown,
        (_, KeyCode::Char('k')) | (_, KeyCode::Up) => KeyAction::MoveUp,
        (_, KeyCode::Char('l')) | (_, KeyCode::Right) => KeyAction::MoveRight,
        (_, KeyCode::Char('w')) => KeyAction::MoveWordForward,
        (_, KeyCode::Char('b')) => KeyAction::MoveWordBackward,
        (_, KeyCode::Char('0')) | (_, KeyCode::Home) => KeyAction::MoveLineStart,
        (_, KeyCode::Char('$')) | (_, KeyCode::End) => KeyAction::MoveLineEnd,
        (_, KeyCode::Char('g')) => KeyAction::None,
        (_, KeyCode::Char('G')) => KeyAction::MoveFileEnd,

        (_, KeyCode::Char('x')) => KeyAction::DeleteForward,
        (_, KeyCode::Char('o')) => KeyAction::OpenLineBelow,
        (_, KeyCode::Char('O')) => KeyAction::OpenLineAbove,
        (_, KeyCode::Char('J')) => KeyAction::JoinLines,
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => KeyAction::ScrollHalfPageUp,
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => KeyAction::ScrollHalfPageDown,
        (_, KeyCode::Char('u')) => KeyAction::Undo,
        (KeyModifiers::CONTROL, KeyCode::Char('r')) => KeyAction::Redo,
        (_, KeyCode::Char('p')) => KeyAction::PasteAfter,
        (_, KeyCode::Char('P')) => {
            KeyAction::PasteAfter
        }
        (_, KeyCode::Char('r')) => KeyAction::SetMode(EditorMode::ReplaceChar),
        (_, KeyCode::Char('>')) => KeyAction::IndentLine,
        (_, KeyCode::Char('<')) => KeyAction::UnindentLine,
        (_, KeyCode::Char('n')) => KeyAction::RepeatFind(true),
        (_, KeyCode::Char('N')) => KeyAction::RepeatFind(false),

        (KeyModifiers::CONTROL, KeyCode::Char('s')) => KeyAction::Save,
        (KeyModifiers::CONTROL, KeyCode::Char('q')) => KeyAction::Quit,
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => KeyAction::Quit,

        (_, KeyCode::PageUp) => KeyAction::PageUp,
        (_, KeyCode::PageDown) => KeyAction::PageDown,

        (_, KeyCode::Esc) => KeyAction::SetMode(EditorMode::Normal),

        _ => KeyAction::None,
    }
}

pub fn normal_key_post(key: KeyEvent) -> Option<KeyAction> {
    match key.code {
        KeyCode::Char('a') | KeyCode::Char('A') => Some(KeyAction::SetMode(EditorMode::Insert)),
        KeyCode::Char('i') | KeyCode::Char('I') => Some(KeyAction::SetMode(EditorMode::Insert)),
        _ => None,
    }
}

pub fn insert_key(key: KeyEvent) -> KeyAction {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => KeyAction::SetMode(EditorMode::Normal),
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => KeyAction::SetMode(EditorMode::Normal),
        (KeyModifiers::CONTROL, KeyCode::Char('s')) => KeyAction::Save,
        (KeyModifiers::CONTROL, KeyCode::Char('w')) => KeyAction::MoveWordForward,
        (_, KeyCode::Enter) => KeyAction::InsertNewline,
        (_, KeyCode::Tab) => KeyAction::InsertChar('\t'),
        (_, KeyCode::Backspace) => KeyAction::DeleteBackward,
        (_, KeyCode::Delete) => KeyAction::DeleteForward,
        (_, KeyCode::Left) => KeyAction::MoveLeft,
        (_, KeyCode::Right) => KeyAction::MoveRight,
        (_, KeyCode::Up) => KeyAction::MoveUp,
        (_, KeyCode::Down) => KeyAction::MoveDown,
        (_, KeyCode::Home) => KeyAction::MoveLineStart,
        (_, KeyCode::End) => KeyAction::MoveLineEnd,
        (_, KeyCode::Char(c)) => KeyAction::InsertChar(c),
        _ => KeyAction::None,
    }
}

pub fn command_key(key: KeyEvent) -> KeyAction {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => KeyAction::SetMode(EditorMode::Normal),
        (_, KeyCode::Enter) => KeyAction::ExecuteInput,
        (_, KeyCode::Char(c)) => KeyAction::InputChar(c),
        (_, KeyCode::Backspace) => KeyAction::InputBackspace,
        _ => KeyAction::None,
    }
}

pub fn parse_command(input: &str) -> KeyAction {
    let trimmed = input.trim();
    match trimmed {
        "q" | "quit" => KeyAction::Quit,
        "q!" | "quit!" => KeyAction::ForceQuit,
        "w" | "write" => KeyAction::Save,
        "wq" | "x" => KeyAction::Save,
        "wq!" => KeyAction::Save,
        s if s.starts_with("w ") => KeyAction::SaveAs,
        s if s.starts_with('g') && s.len() > 1 && s[1..].trim().parse::<usize>().is_ok() => {
            let line = s[1..].trim().parse::<usize>().unwrap_or(1);
            KeyAction::GotoLine(line)
        }
        s if s.parse::<usize>().is_ok() => {
            KeyAction::GotoLine(s.parse::<usize>().unwrap_or(1))
        }
        s if s.starts_with('%') => parse_substitute(&s[1..], true),
        s if s.starts_with("s/") => parse_substitute(s, false),
        _ => KeyAction::None,
    }
}

fn parse_substitute(s: &str, global: bool) -> KeyAction {
    let parts: Vec<&str> = s.splitn(4, '/').collect();
    if parts.len() >= 3 {
        let old = parts[1];
        let new_part = parts[2];
        let flags = parts.get(3).copied().unwrap_or("");
        if global || flags.contains('g') {
            KeyAction::ReplaceAll(old.to_string(), new_part.to_string())
        } else {
            KeyAction::ReplaceFirst(old.to_string(), new_part.to_string())
        }
    } else {
        KeyAction::None
    }
}
