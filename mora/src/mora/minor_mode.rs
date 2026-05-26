use crossterm::event::KeyEvent;
use super::keymap::KeyAction;

pub trait MinorMode: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn modeline_indicator(&self) -> &'static str;
    fn on_key(&self, _key: KeyEvent) -> Option<KeyAction> { None }
    fn on_enable(&self) {}
    fn on_disable(&self) {}
    fn priority(&self) -> i32 { 0 }
}

#[derive(Debug)]
pub struct MinorModeRegistry {
    modes: Vec<Box<dyn MinorMode>>,
}

impl MinorModeRegistry {
    pub fn new() -> Self {
        Self { modes: Vec::new() }
    }

    pub fn register(&mut self, mode: Box<dyn MinorMode>) {
        mode.on_enable();
        self.modes.push(mode);
        self.modes.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    pub fn enable_by_name(&mut self, name: &str) -> bool {
        if self.is_enabled(name) {
            return false;
        }
        if let Some(mode) = create_minor_mode(name) {
            self.register(mode);
            true
        } else {
            false
        }
    }

    pub fn disable_by_name(&mut self, name: &str) -> bool {
        if let Some(pos) = self.modes.iter().position(|m| m.name() == name) {
            self.modes[pos].on_disable();
            self.modes.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn toggle_by_name(&mut self, name: &str) -> bool {
        if self.is_enabled(name) {
            self.disable_by_name(name);
            false
        } else {
            self.enable_by_name(name)
        }
    }

    pub fn is_enabled(&self, name: &str) -> bool {
        self.modes.iter().any(|m| m.name() == name)
    }

    pub fn intercept_key(&self, key: KeyEvent) -> Option<KeyAction> {
        for mode in &self.modes {
            if let Some(action) = mode.on_key(key) {
                return Some(action);
            }
        }
        None
    }

    pub fn modeline_string(&self) -> String {
        let indicators: Vec<&str> = self.modes.iter()
            .map(|m| m.modeline_indicator())
            .filter(|s| !s.is_empty())
            .collect();
        if indicators.is_empty() {
            String::new()
        } else {
            format!(" {} ", indicators.join(" "))
        }
    }

    pub fn enabled_names(&self) -> Vec<&'static str> {
        self.modes.iter().map(|m| m.name()).collect()
    }

    pub fn count(&self) -> usize {
        self.modes.len()
    }
}

pub fn all_minor_mode_names() -> Vec<&'static str> {
    vec![
        "line-numbers",
        "relative-line-numbers",
        "auto-fill",
        "whitespace",
        "highlight-trailing",
        "auto-save",
        "read-only",
    ]
}

pub fn create_minor_mode(name: &str) -> Option<Box<dyn MinorMode>> {
    match name.to_lowercase().as_str() {
        "line-numbers" | "line-numbers-mode" => Some(Box::new(LineNumbersMode)),
        "relative-line-numbers" | "relative-line-numbers-mode" => Some(Box::new(RelativeLineNumbersMode)),
        "auto-fill" | "auto-fill-mode" => Some(Box::new(AutoFillMode)),
        "whitespace" | "whitespace-mode" => Some(Box::new(WhitespaceMode)),
        "highlight-trailing" | "highlight-trailing-whitespace" => Some(Box::new(HighlightTrailingMode)),
        "auto-save" | "auto-save-mode" => Some(Box::new(AutoSaveMode)),
        "read-only" | "read-only-mode" => Some(Box::new(ReadOnlyMode)),
        _ => None,
    }
}

#[derive(Debug)]
pub struct LineNumbersMode;

impl MinorMode for LineNumbersMode {
    fn name(&self) -> &'static str { "line-numbers" }
    fn modeline_indicator(&self) -> &'static str { "LN" }
    fn priority(&self) -> i32 { 0 }
}

#[derive(Debug)]
pub struct RelativeLineNumbersMode;

impl MinorMode for RelativeLineNumbersMode {
    fn name(&self) -> &'static str { "relative-line-numbers" }
    fn modeline_indicator(&self) -> &'static str { "RLN" }
    fn priority(&self) -> i32 { 0 }
}

#[derive(Debug)]
pub struct AutoFillMode;

impl MinorMode for AutoFillMode {
    fn name(&self) -> &'static str { "auto-fill" }
    fn modeline_indicator(&self) -> &'static str { "Fill" }
    fn priority(&self) -> i32 { -10 }
}

#[derive(Debug)]
pub struct WhitespaceMode;

impl MinorMode for WhitespaceMode {
    fn name(&self) -> &'static str { "whitespace" }
    fn modeline_indicator(&self) -> &'static str { "WS" }
    fn priority(&self) -> i32 { 0 }
}

#[derive(Debug)]
pub struct HighlightTrailingMode;

impl MinorMode for HighlightTrailingMode {
    fn name(&self) -> &'static str { "highlight-trailing" }
    fn modeline_indicator(&self) -> &'static str { "TW" }
    fn priority(&self) -> i32 { 0 }
}

#[derive(Debug)]
pub struct AutoSaveMode;

impl MinorMode for AutoSaveMode {
    fn name(&self) -> &'static str { "auto-save" }
    fn modeline_indicator(&self) -> &'static str { "AS" }
    fn priority(&self) -> i32 { -20 }
}

#[derive(Debug)]
pub struct ReadOnlyMode;

impl MinorMode for ReadOnlyMode {
    fn name(&self) -> &'static str { "read-only" }
    fn modeline_indicator(&self) -> &'static str { "RO" }
    fn priority(&self) -> i32 { 100 }

    fn on_key(&self, key: KeyEvent) -> Option<KeyAction> {
        use crossterm::event::{KeyCode, KeyModifiers};
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c'))
            | (KeyModifiers::CONTROL, KeyCode::Char('g'))
            | (_, KeyCode::Esc) => None,
            (_, KeyCode::Char(_)) | (_, KeyCode::Enter) | (_, KeyCode::Backspace) | (_, KeyCode::Tab) => {
                Some(KeyAction::None)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_create() {
        let registry = MinorModeRegistry::new();
        assert_eq!(registry.count(), 0);
        assert!(registry.modeline_string().is_empty());
    }

    #[test]
    fn test_enable_disable() {
        let mut registry = MinorModeRegistry::new();
        assert!(registry.enable_by_name("line-numbers"));
        assert_eq!(registry.count(), 1);
        assert!(registry.is_enabled("line-numbers"));
        assert!(!registry.is_enabled("auto-fill"));

        assert!(registry.disable_by_name("line-numbers"));
        assert_eq!(registry.count(), 0);
        assert!(!registry.is_enabled("line-numbers"));
    }

    #[test]
    fn test_toggle() {
        let mut registry = MinorModeRegistry::new();
        assert!(registry.toggle_by_name("line-numbers"));
        assert!(registry.is_enabled("line-numbers"));
        assert!(!registry.toggle_by_name("line-numbers"));
        assert!(!registry.is_enabled("line-numbers"));
    }

    #[test]
    fn test_enable_same_twice() {
        let mut registry = MinorModeRegistry::new();
        assert!(registry.enable_by_name("line-numbers"));
        assert!(!registry.enable_by_name("line-numbers"));
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_enable_unknown() {
        let mut registry = MinorModeRegistry::new();
        assert!(!registry.enable_by_name("nonexistent"));
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_modeline_string() {
        let mut registry = MinorModeRegistry::new();
        registry.enable_by_name("line-numbers");
        registry.enable_by_name("auto-fill");
        let s = registry.modeline_string();
        assert!(s.contains("LN"));
        assert!(s.contains("Fill"));
    }

    #[test]
    fn test_enabled_names() {
        let mut registry = MinorModeRegistry::new();
        registry.enable_by_name("whitespace");
        registry.enable_by_name("auto-save");
        let names = registry.enabled_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"whitespace"));
        assert!(names.contains(&"auto-save"));
    }

    #[test]
    fn test_priority_order() {
        let mut registry = MinorModeRegistry::new();
        registry.enable_by_name("line-numbers");  // priority 0
        registry.enable_by_name("read-only");     // priority 100
        registry.enable_by_name("auto-fill");     // priority -10
        let names = registry.enabled_names();
        assert_eq!(names[0], "read-only");
        assert_eq!(names[1], "line-numbers");
        assert_eq!(names[2], "auto-fill");
    }

    #[test]
    fn test_read_only_intercept() {
        let mode = ReadOnlyMode;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let insert_key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert!(mode.on_key(insert_key).is_some());
        let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert!(mode.on_key(esc_key).is_none());
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(mode.on_key(ctrl_c).is_none());
    }

    #[test]
    fn test_all_names() {
        let names = all_minor_mode_names();
        assert!(names.len() >= 7);
        assert!(names.contains(&"line-numbers"));
        assert!(names.contains(&"read-only"));
    }

    #[test]
    fn test_create_minor_mode() {
        assert!(create_minor_mode("line-numbers").is_some());
        assert!(create_minor_mode("line-numbers-mode").is_some());
        assert!(create_minor_mode("read-only").is_some());
        assert!(create_minor_mode("nonexistent").is_none());
    }
}
