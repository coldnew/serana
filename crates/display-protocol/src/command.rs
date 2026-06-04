use serde::{Deserialize, Serialize};

/// Imperative commands from core to frontend
///
/// These are distinct from FrameUpdate (which carries rendered content).
/// DisplayCmd handles window management, visual effects, and UI chrome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisplayCmd {
    /// Set the terminal/window title
    SetTitle(String),
    /// Ring the terminal bell / visual bell
    Bell,
    /// Set the cursor blink state
    SetCursorBlink(bool),
    /// Suspend the frontend (e.g. Ctrl-Z in terminal)
    Suspend,
    /// Quit the frontend
    Quit,
    /// Show a popup menu with items
    ShowPopup {
        items: Vec<PopupItem>,
        x: u16,
        y: u16,
    },
    /// Hide the popup menu
    HidePopup,
    /// Show a tooltip
    ShowTooltip { text: String, x: u16, y: u16 },
    /// Hide the tooltip
    HideTooltip,
    /// Set the theme (color scheme name)
    SetTheme(String),
    /// Request a screenshot or debug dump
    DebugDump,
}

/// A popup menu item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopupItem {
    pub label: String,
    pub detail: Option<String>,
    pub kind: PopupItemKind,
}

/// Popup menu item type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PopupItemKind {
    Default,
    Selected,
    Separator,
    Disabled,
}
