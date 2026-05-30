use super::display::event::{MoraKeyCode, MoraKeyEvent, MoraKeyModifiers};
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
    Iedit,
}

impl EditorMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Normal => "EVIL",
            Self::Insert => "INSERT",
            Self::Command => "COMMAND",
            Self::SearchForward => "SEARCH /",
            Self::SearchBackward => "SEARCH ?",
            Self::Emacs => "EMACS",
            Self::ReplaceChar => "REPLACE",
            Self::Visual => "VISUAL",
            Self::Iedit => "IEDIT",
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
    PopMark,
    UniversalArg,
    UppercaseRegion,
    LowercaseRegion,
    MwimBeginning,
    MwimEnd,
    ExpandRegion,
    ContractRegion,
    HungryDeleteForward,
    HungryDeleteBackward,
    DabbrevExpand,
    ZapToChar,
    InsertEmptyLineBelow,
    InsertEmptyLineAbove,
    GotoLastChange,
    CleanupBuffer,
    CopyAndComment,
    NarrowRegion,
    Widen,
    Dos2Unix,
    Unix2Dos,
    ToggleFold,
    MxComplete,
    ToggleCase,
    SubstituteLine,
    SearchWordForward,
    SearchWordBackward,
    RepeatLastChange,
    GotoMatchingBracket,
    DeleteToStartOfLine,
    YankWord,
    DeleteInnerWord,
    DeleteAroundWord,
    ChangeInnerWord,
    ChangeAroundWord,
    DeleteToEndOfWord,
    DeleteInnerBrackets(char, char),
    DeleteAroundBrackets(char, char),
    ChangeSurround(char, char),
    DeleteSurround(char),
    AddSurround(char),
    AceJump,
    OpenFold,
    CloseFold,
    ToggleFoldEvil,
    ReduceFolds,
    MaximizeFolds,
    SplitHorizontal,
    SplitVertical,
    DeleteWindow,
    DeleteOtherWindows,
    BalanceWindows,
    OtherWindow,
    SwitchMajorMode(String),
    ToggleMinorMode(String),
}

impl KeyAction {
    pub fn needs_digit(&self) -> bool {
        matches!(self, KeyAction::GotoLine(_))
    }
}

fn ctrl_char(c: char) -> (MoraKeyModifiers, MoraKeyCode) {
    (MoraKeyModifiers::CTRL, MoraKeyCode::Char(c))
}

fn alt_char(c: char) -> (MoraKeyModifiers, MoraKeyCode) {
    (MoraKeyModifiers::ALT, MoraKeyCode::Char(c))
}

pub fn normal_key(key: MoraKeyEvent) -> KeyAction {
    match (key.modifiers, key.code) {
        (_, MoraKeyCode::Char('i')) => KeyAction::SetMode(EditorMode::Insert),
        (_, MoraKeyCode::Char('a')) => KeyAction::MoveRight,
        (_, MoraKeyCode::Char('A')) => KeyAction::MoveLineEnd,
        (_, MoraKeyCode::Char('I')) => KeyAction::MoveLineStart,
        (_, MoraKeyCode::Char(':')) => KeyAction::SetMode(EditorMode::Command),
        (_, MoraKeyCode::Char('/')) => KeyAction::SetMode(EditorMode::SearchForward),
        (_, MoraKeyCode::Char('?')) => KeyAction::SetMode(EditorMode::SearchBackward),
        (_, MoraKeyCode::Char('v')) => KeyAction::ToggleVisual,
        (_, MoraKeyCode::Char('V')) => KeyAction::ToggleVisual,
        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('e')) => KeyAction::SwitchToEmacs,

        (_, MoraKeyCode::Char('h')) | (_, MoraKeyCode::Left) => KeyAction::MoveLeft,
        (_, MoraKeyCode::Char('j')) | (_, MoraKeyCode::Down) => KeyAction::MoveDown,
        (_, MoraKeyCode::Char('k')) | (_, MoraKeyCode::Up) => KeyAction::MoveUp,
        (_, MoraKeyCode::Char('l')) | (_, MoraKeyCode::Right) => KeyAction::MoveRight,
        (_, MoraKeyCode::Char('w')) => KeyAction::MoveWordForward,
        (_, MoraKeyCode::Char('b')) => KeyAction::MoveWordBackward,
        (_, MoraKeyCode::Char('e')) | (_, MoraKeyCode::Char('E')) => KeyAction::MoveWordEnd,
        (_, MoraKeyCode::Char('0')) | (_, MoraKeyCode::Home) => KeyAction::MoveLineStart,
        (_, MoraKeyCode::Char('$')) | (_, MoraKeyCode::End) => KeyAction::MoveLineEnd,
        (_, MoraKeyCode::Char('g')) => KeyAction::None,
        (_, MoraKeyCode::Char('z')) => KeyAction::None,
        (_, MoraKeyCode::Char('G')) => KeyAction::MoveFileEnd,

        (_, MoraKeyCode::Char('x')) => KeyAction::DeleteForward,
        (_, MoraKeyCode::Char('o')) => KeyAction::OpenLineBelow,
        (_, MoraKeyCode::Char('O')) => KeyAction::OpenLineAbove,
        (_, MoraKeyCode::Char('J')) => KeyAction::JoinLines,
        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('u')) => KeyAction::ScrollHalfPageUp,
        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('d')) => KeyAction::ScrollHalfPageDown,
        (_, MoraKeyCode::Char('u')) => KeyAction::Undo,
        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('r')) => KeyAction::Redo,
        (_, MoraKeyCode::Char('p')) => KeyAction::PasteAfter,
        (_, MoraKeyCode::Char('P')) => KeyAction::PasteAfter,
        (_, MoraKeyCode::Char('r')) => KeyAction::SetMode(EditorMode::ReplaceChar),
        (_, MoraKeyCode::Char('>')) => KeyAction::IndentLine,
        (_, MoraKeyCode::Char('<')) => KeyAction::UnindentLine,
        (_, MoraKeyCode::Char('n')) => KeyAction::RepeatFind(true),
        (_, MoraKeyCode::Char('N')) => KeyAction::RepeatFind(false),

        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('s')) => KeyAction::Save,
        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('q')) => KeyAction::Quit,
        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('c')) => KeyAction::Quit,

        (_, MoraKeyCode::PageUp) => KeyAction::PageUp,
        (_, MoraKeyCode::PageDown) => KeyAction::PageDown,

        (_, MoraKeyCode::Esc) => KeyAction::SetMode(EditorMode::Normal),

        _ => KeyAction::None,
    }
}

pub fn normal_key_post(key: MoraKeyEvent) -> Option<KeyAction> {
    match key.code {
        MoraKeyCode::Char('a') | MoraKeyCode::Char('A') => {
            Some(KeyAction::SetMode(EditorMode::Insert))
        }
        MoraKeyCode::Char('i') | MoraKeyCode::Char('I') => {
            Some(KeyAction::SetMode(EditorMode::Insert))
        }
        _ => None,
    }
}

pub fn insert_key(key: MoraKeyEvent) -> KeyAction {
    match (key.modifiers, key.code) {
        (_, MoraKeyCode::Esc) => KeyAction::SetMode(EditorMode::Normal),
        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('c')) => KeyAction::SetMode(EditorMode::Normal),
        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('s')) => KeyAction::Save,
        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('w')) => KeyAction::MoveWordForward,
        // Emacs-style navigation in insert mode
        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('p')) => KeyAction::MoveUp,
        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('n')) => KeyAction::MoveDown,
        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('b')) => KeyAction::MoveLeft,
        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('f')) => KeyAction::MoveRight,
        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('a')) => KeyAction::MoveLineStart,
        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('e')) => KeyAction::MoveLineEnd,
        // Emacs-style editing in insert mode
        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('d')) => KeyAction::DeleteForward,
        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('k')) => KeyAction::KillLine,
        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('y')) => KeyAction::Yank,
        (MoraKeyModifiers::CTRL, MoraKeyCode::Char('/')) => KeyAction::Undo,
        (_, MoraKeyCode::Enter) => KeyAction::InsertNewline,
        (_, MoraKeyCode::Tab) => KeyAction::InsertChar('\t'),
        (_, MoraKeyCode::Backspace) => KeyAction::DeleteBackward,
        (_, MoraKeyCode::Delete) => KeyAction::DeleteForward,
        (_, MoraKeyCode::Left) => KeyAction::MoveLeft,
        (_, MoraKeyCode::Right) => KeyAction::MoveRight,
        (_, MoraKeyCode::Up) => KeyAction::MoveUp,
        (_, MoraKeyCode::Down) => KeyAction::MoveDown,
        (_, MoraKeyCode::Home) => KeyAction::MoveLineStart,
        (_, MoraKeyCode::End) => KeyAction::MoveLineEnd,
        (_, MoraKeyCode::Char(c)) => KeyAction::InsertChar(c),
        _ => KeyAction::None,
    }
}

pub fn command_key(key: MoraKeyEvent) -> KeyAction {
    match (key.modifiers, key.code) {
        (_, MoraKeyCode::Esc) => KeyAction::SetMode(EditorMode::Normal),
        (_, MoraKeyCode::Enter) => KeyAction::ExecuteInput,
        (_, MoraKeyCode::Tab) => KeyAction::MxComplete,
        (_, MoraKeyCode::Char(c)) => KeyAction::InputChar(c),
        (_, MoraKeyCode::Backspace) => KeyAction::InputBackspace,
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
        s if s.parse::<usize>().is_ok() => KeyAction::GotoLine(s.parse::<usize>().unwrap_or(1)),
        s if s.starts_with('%') => parse_substitute(&s[1..], true),
        s if s.starts_with("s/") => parse_substitute(s, false),
        "goto-line" | "goto-line-number" => KeyAction::GotoLine(0),
        "replace-string" => KeyAction::ReplaceAll(String::new(), String::new()),
        "cleanup-buffer" | "delete-trailing-whitespace" => KeyAction::CleanupBuffer,
        "dos2unix" | "unix-line-endings" => KeyAction::Dos2Unix,
        "unix2dos" | "dos-line-endings" => KeyAction::Unix2Dos,
        "toggle-fold" | "folding" => KeyAction::ToggleFold,
        "narrow-to-region" => KeyAction::NarrowRegion,
        "widen" => KeyAction::Widen,
        "copy-and-comment" => KeyAction::CopyAndComment,
        "goto-last-change" => KeyAction::GotoLastChange,
        "transpose-char" => KeyAction::TransposeChar,
        "transpose-word" => KeyAction::TransposeWord,
        "transpose-line" => KeyAction::TransposeLine,
        "capitalize-word" => KeyAction::CapitalizeWord,
        "uppercase-word" => KeyAction::UppercaseWord,
        "lowercase-word" => KeyAction::LowercaseWord,
        "uppercase-region" => KeyAction::UppercaseRegion,
        "lowercase-region" => KeyAction::LowercaseRegion,
        "iedit" | "multi-cursor-edit" => KeyAction::None,
        "indent-region" => KeyAction::IndentLine,
        "save-buffer" => KeyAction::Save,
        "save-some-buffers" => KeyAction::Save,
        "kill-buffer" => KeyAction::Quit,
        "kill-emacs" => KeyAction::ForceQuit,
        "recenter" | "recenter-top-bottom" => KeyAction::None,
        "describe-mode" | "describe-key" => KeyAction::None,
        "what-cursor-position" => KeyAction::None,
        s if s.starts_with("switch-mode ") => {
            let mode_name = s["switch-mode ".len()..].trim();
            KeyAction::SwitchMajorMode(mode_name.to_string())
        }
        s if s.starts_with("set-mode ") => {
            let mode_name = s["set-mode ".len()..].trim();
            KeyAction::SwitchMajorMode(mode_name.to_string())
        }
        s if s.starts_with("toggle-minor-mode ") => {
            let mode_name = s["toggle-minor-mode ".len()..].trim();
            KeyAction::ToggleMinorMode(mode_name.to_string())
        }
        s if s.starts_with("enable-minor-mode ") => {
            let mode_name = s["enable-minor-mode ".len()..].trim();
            KeyAction::ToggleMinorMode(mode_name.to_string())
        }
        s if s.starts_with("disable-minor-mode ") => {
            let mode_name = s["disable-minor-mode ".len()..].trim();
            KeyAction::ToggleMinorMode(format!("!{}", mode_name))
        }
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
